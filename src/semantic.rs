//! Hybrid search: lexical shortlist, dense re-scoring with one or two local models, convex-combination fusion.
//!
//! Design (bench/experiments/RESULTS.md): a Tantivy BM25 shortlist (top 1000 chunks) is re-scored by the dense model(s) from int8 vectors
//! stored in SQLite, so a query never scans the corpus and memory does not grow with it. Two models are blended 50/50 (z-scores over the
//! shortlist), then fused with the BM25 scores: `0.55 * dense + 0.45 * lexical`, each z-scored over its own top-100. Query encoding goes
//! through a resident helper process (one per model), so the model is loaded once and a query costs milliseconds.
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Mutex, OnceLock, mpsc},
    thread,
    time::Duration,
};

pub const MIGRATION: &str = "CREATE TABLE embedding_models(identity TEXT PRIMARY KEY,manifest TEXT NOT NULL,helper_sha256 TEXT NOT NULL,dimension INTEGER NOT NULL); CREATE TABLE embeddings(model TEXT NOT NULL REFERENCES embedding_models(identity),kind TEXT NOT NULL,id INTEGER NOT NULL,signature TEXT NOT NULL,vector TEXT NOT NULL,PRIMARY KEY(model,kind,id));";
const DIM: usize = 384;
const SHORTLIST: usize = 1000;
const POOL: usize = 100;
const SCAN_LIMIT: usize = 50_000;
/// Entries up to which every stored vector is scored. `BORG_KNOWLEDGE_SCAN_LIMIT` lowers it (tests, very small machines).
fn scan_limit() -> usize {
    std::env::var("BORG_KNOWLEDGE_SCAN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .map_or(SCAN_LIMIT, |n: usize| n.min(SCAN_LIMIT))
}
const ALPHA: f64 = 0.55;
const BATCH: usize = 128;
const MAX_MODELS: usize = 3;
const MAX_LINE: u64 = 128_000_000;
const WORKER_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Deserialize)]
struct Encoded {
    vectors: Vec<Vec<f64>>,
    model_sha256: String,
    truncated_at_limit: usize,
}
fn check_identity(expected: &str, encoded: &Encoded) -> Result<()> {
    if encoded.model_sha256 != expected {
        bail!("embedding model changed during request; retry with immutable model artifacts");
    }
    Ok(())
}
fn identity(model: &Path) -> Result<(String, String)> {
    crate::app::no_links(model)?;
    let manifest = String::from_utf8(crate::app::bounded_read(&model.join("manifest.json"))?)?;
    for name in ["model.onnx", "tokenizer.json"] {
        crate::app::no_links(&model.join(name))?;
    }
    Ok((crate::app::hash(manifest.as_bytes()), manifest))
}
fn helper_hash() -> Result<String> {
    Ok(crate::app::hash(&fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/embed.py"),
    )?))
}

// ------------------------------------------------------------------ resident helper
struct Worker {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<std::io::Result<String>>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
static WORKERS: OnceLock<Mutex<HashMap<PathBuf, Worker>>> = OnceLock::new();

fn python() -> PathBuf {
    std::env::var_os("BORG_KNOWLEDGE_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join(if cfg!(windows) {
                ".venv/Scripts/python.exe"
            } else {
                ".venv/bin/python"
            })
        })
}
fn read_bounded_line<R: BufRead>(r: &mut R) -> std::io::Result<Option<String>> {
    let mut line = String::new();
    let n = r.by_ref().take(MAX_LINE).read_line(&mut line)?;
    if n == 0 {
        return Ok(None);
    }
    if !line.ends_with('\n') && n as u64 >= MAX_LINE {
        return Err(std::io::Error::other(
            "embedding helper response exceeds 128 MB",
        ));
    }
    Ok(Some(line))
}
fn spawn_worker(model: &Path, expected: &str) -> Result<Worker> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::new(python());
    cmd.arg("-I")
        .arg(root.join("tools/embed.py"))
        .arg("--serve")
        .arg(model)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // The helper is scrubbed like every other one: no agent credentials in a text parser.
    crate::app::helper_env(&mut cmd);
    let mut child = cmd.spawn().context("start the embedding helper")?;
    let stdin = child.stdin.take().context("helper stdin")?;
    let stdout = child.stdout.take().context("helper stdout")?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_bounded_line(&mut reader) {
                Ok(Some(line)) => {
                    if tx.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    });
    let worker = Worker {
        child,
        stdin,
        lines: rx,
    };
    let ready = worker
        .lines
        .recv_timeout(Duration::from_secs(180))
        .context("embedding helper did not start in 180 s")??;
    let ready: Value = serde_json::from_str(&ready).context("helper handshake")?;
    if let Some(e) = ready["error"].as_str() {
        bail!(
            "embedding helper failed: {}",
            e.chars().take(1000).collect::<String>()
        );
    }
    if ready["ready"] != true || ready["model_sha256"] != expected {
        bail!("embedding helper loaded a different model than the manifest on disk");
    }
    Ok(worker)
}
fn ask(worker: &mut Worker, body: &str) -> Result<Encoded> {
    worker.stdin.write_all(body.as_bytes())?;
    worker.stdin.write_all(b"\n")?;
    worker.stdin.flush()?;
    let line = worker
        .lines
        .recv_timeout(WORKER_TIMEOUT)
        .context("embedding helper timed out (600 s)")??;
    let v: Value = serde_json::from_str(&line)?;
    if let Some(e) = v["error"].as_str() {
        bail!(
            "embedding helper failed: {}",
            e.chars().take(1000).collect::<String>()
        );
    }
    Ok(serde_json::from_value(v)?)
}
fn encode_batch(model: &Path, expected: &str, texts: &[String], kind: &str) -> Result<Encoded> {
    let body = json!({"kind":kind,"texts":texts}).to_string();
    if body.len() > 64_000_000 {
        bail!("embedding request exceeds 64 MB");
    }
    let pool = WORKERS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = pool
        .lock()
        .map_err(|_| anyhow::anyhow!("helper pool poisoned"))?;
    for attempt in 0..2 {
        if !guard.contains_key(model) {
            guard.insert(model.to_path_buf(), spawn_worker(model, expected)?);
        }
        let worker = guard.get_mut(model).expect("inserted above");
        match ask(worker, &body) {
            Ok(e) => return Ok(e),
            Err(e) if attempt == 0 && e.to_string().contains("timed out") => {
                guard.remove(model); // a stuck helper is killed by Drop and replaced once
            }
            Err(e) => {
                let broken = e.downcast_ref::<std::io::Error>().is_some();
                if broken {
                    guard.remove(model);
                }
                return Err(e);
            }
        }
    }
    bail!("embedding helper unavailable")
}
fn encode(model: &Path, texts: &[String], kind: &str) -> Result<Encoded> {
    if texts.is_empty() {
        bail!("embedding batch must not be empty");
    }
    let (expected, _) = identity(model)?;
    let mut merged: Option<Encoded> = None;
    for part in texts.chunks(BATCH) {
        let mut e = encode_batch(model, &expected, part, kind)?;
        check_identity(&expected, &e)?;
        if e.vectors.len() != part.len()
            || e.vectors.iter().any(|v| {
                v.len() != DIM
                    || v.iter().any(|x| !x.is_finite())
                    || (v.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() > 0.01
            })
        {
            bail!("embedding model/dimension/norm mismatch");
        }
        match merged.as_mut() {
            None => merged = Some(e),
            Some(m) => {
                m.truncated_at_limit += e.truncated_at_limit;
                m.vectors.append(&mut e.vectors);
            }
        }
    }
    Ok(merged.expect("texts is not empty"))
}

// ------------------------------------------------------------------ int8 vectors
/// `[scale: f32 LE][384 x i8]`: a quarter of the memory of f32, the same ranking (bench E10).
fn quantize(v: &[f64]) -> Vec<u8> {
    let max = v.iter().fold(0f64, |m, x| m.max(x.abs()));
    let scale = if max > 0.0 { max / 127.0 } else { 1.0 };
    let mut out = Vec::with_capacity(4 + v.len());
    out.extend_from_slice(&(scale as f32).to_le_bytes());
    out.extend(
        v.iter()
            .map(|x| (x / scale).round().clamp(-127.0, 127.0) as i8 as u8),
    );
    out
}
/// Cosine of a stored vector with a unit query vector. `Err` when the stored vector is not a plausible unit vector
/// (wrong length, non-finite or zero scale, norm far from 1): derived data that was damaged is refused, never ranked.
fn dot_q(blob: &[u8], q: &[f64]) -> Result<f64> {
    if blob.len() != 4 + DIM || q.len() != DIM {
        bail!("corrupt derived vector; rebuild embeddings");
    }
    let scale = f32::from_le_bytes(blob[..4].try_into()?) as f64;
    let norm = blob[4..]
        .iter()
        .map(|b| ((*b as i8) as f64).powi(2))
        .sum::<f64>()
        .sqrt()
        * scale;
    if !scale.is_finite() || scale <= 0.0 || (norm - 1.0).abs() > 0.05 {
        bail!("corrupt derived vector; rebuild embeddings");
    }
    Ok(blob[4..]
        .iter()
        .zip(q)
        .map(|(b, x)| (*b as i8) as f64 * x)
        .sum::<f64>()
        * scale)
}

// ------------------------------------------------------------------ entries
fn chunk_text(heading: &str, text: &str) -> String {
    format!("{heading}\n{text}")
}
fn chunk_signature(sha: &str, revision: i64, text: &str) -> String {
    crate::app::hash(format!("{sha}:{revision}:{text}").as_bytes())
}
fn record_text(payload: &str) -> Result<String> {
    let p: Value = serde_json::from_str(payload)?;
    Ok([
        "technology",
        "symptom",
        "root_cause",
        "remediation",
        "applicability",
        "exclusions",
    ]
    .iter()
    .map(|k| p[k].as_str().unwrap_or(""))
    .collect::<Vec<_>>()
    .join("\n"))
}

pub fn rebuild(c: &mut Connection, models: &[PathBuf]) -> Result<Value> {
    if models.is_empty() || models.len() > MAX_MODELS {
        bail!("give 1 to {MAX_MODELS} --model directories");
    }
    let helper = helper_hash()?;
    // Lock corpus/record revisions for the complete rebuild; readers remain read-only.
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut report = Vec::new();
    let mut embedded_total = 0usize;
    for model in models {
        let (identity, manifest) = identity(model)?;
        // Start (or reuse) the helper first: it verifies the model files against the manifest, even when nothing needs embedding.
        check_identity(&identity, &encode(model, &["verify".to_string()], "query")?)?;
        let stored: Option<String> = tx
            .query_row(
                "SELECT helper_sha256 FROM embedding_models WHERE identity=?1",
                [&identity],
                |r| r.get(0),
            )
            .optional()?;
        if stored.as_deref() != Some(helper.as_str()) {
            // New model or a changed recipe: nothing stored under this identity can be trusted.
            tx.execute("DELETE FROM embeddings WHERE model=?1", [&identity])?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO embedding_models VALUES(?1,?2,?3,?4)",
            params![identity, manifest, helper, DIM as i64],
        )?;
        let mut have: HashMap<(String, i64), String> = HashMap::new();
        {
            let mut s = tx.prepare("SELECT kind,id,signature FROM embeddings WHERE model=?1")?;
            for row in s.query_map([&identity], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))? {
                let (kind, id, signature): (String, i64, String) = row?;
                have.insert((kind, id), signature);
            }
        }
        let mut live: HashSet<(String, i64)> = HashSet::new();
        let (mut embedded, mut reused, mut truncated) = (0usize, 0usize, 0usize);
        let mut batch: Vec<(String, i64, String, String)> = Vec::new(); // kind, id, signature, text
        let mut flush = |batch: &mut Vec<(String, i64, String, String)>| -> Result<()> {
            if batch.is_empty() {
                return Ok(());
            }
            let texts: Vec<String> = batch.iter().map(|b| b.3.clone()).collect();
            let encoded = encode(model, &texts, "passage")?;
            check_identity(&identity, &encoded)?;
            truncated += encoded.truncated_at_limit;
            for ((kind, id, signature, _), vector) in batch.iter().zip(&encoded.vectors) {
                tx.execute(
                    "INSERT OR REPLACE INTO embeddings VALUES(?1,?2,?3,?4,?5)",
                    params![identity, kind, id, signature, quantize(vector)],
                )?;
            }
            embedded += batch.len();
            batch.clear();
            Ok(())
        };
        {
            let mut s = tx.prepare("SELECT c.id,s.sha256,s.revision,c.heading,c.text FROM chunks c JOIN sources s ON s.id=c.source_id WHERE s.status='indexed' ORDER BY c.id")?;
            let mut rows = s.query([])?;
            while let Some(r) = rows.next()? {
                let (id, sha, revision, heading, text): (i64, String, i64, String, String) =
                    (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?);
                let text = chunk_text(&heading, &text);
                let signature = chunk_signature(&sha, revision, &text);
                live.insert(("chunk".into(), id));
                if have.get(&("chunk".to_string(), id)) == Some(&signature) {
                    reused += 1;
                } else {
                    batch.push(("chunk".into(), id, signature, text));
                    if batch.len() >= BATCH {
                        flush(&mut batch)?;
                    }
                }
            }
        }
        {
            let mut s =
                tx.prepare("SELECT id,payload_sha256,revision,payload FROM records ORDER BY id")?;
            let mut rows = s.query([])?;
            while let Some(r) = rows.next()? {
                let (id, sha, revision, payload): (i64, String, i64, String) =
                    (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?);
                let signature = format!("{sha}:{revision}");
                live.insert(("record".into(), id));
                if have.get(&("record".to_string(), id)) == Some(&signature) {
                    reused += 1;
                } else {
                    batch.push(("record".into(), id, signature, record_text(&payload)?));
                    if batch.len() >= BATCH {
                        flush(&mut batch)?;
                    }
                }
            }
        }
        flush(&mut batch)?;
        let mut removed = 0usize;
        for (kind, id) in have.keys().filter(|k| !live.contains(*k)) {
            tx.execute(
                "DELETE FROM embeddings WHERE model=?1 AND kind=?2 AND id=?3",
                params![identity, kind, id],
            )?;
            removed += 1;
        }
        embedded_total += embedded;
        report.push(json!({"model_sha256":identity,"embedded":embedded,"reused":reused,"removed":removed,"truncated_at_limit":truncated}));
    }
    tx.commit()?;
    Ok(
        json!({"embedded":embedded_total,"models":report,"dimension":DIM,"storage":"int8","derived_only":true}),
    )
}

// ------------------------------------------------------------------ fusion
#[derive(Debug, Clone)]
struct Cand {
    kind: u8, // 0 chunk, 1 record
    id: i64,
    lex: Option<f64>,
    dense: Option<f64>, // blended z-score over the shortlist, None when any model has no current vector
}
fn zscore(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let sd = if var > 0.0 { var.sqrt() } else { 1.0 };
    values.iter().map(|x| (x - mean) / sd).collect()
}
/// Convex combination of z-scored BM25 and z-scored dense similarity over their own top `pool`, best first.
/// Without any lexical match the dense order decides; without any dense score the lexical order does.
fn fuse(cands: &[Cand], pool: usize, alpha: f64) -> Vec<(usize, f64)> {
    let order = |a: &(usize, f64), b: &(usize, f64)| {
        b.1.total_cmp(&a.1)
            .then(cands[a.0].kind.cmp(&cands[b.0].kind))
            .then(cands[a.0].id.cmp(&cands[b.0].id))
    };
    let mut lex: Vec<(usize, f64)> = cands
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.lex.filter(|l| *l > 0.0).map(|l| (i, l)))
        .collect();
    let mut den: Vec<(usize, f64)> = cands
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.dense.map(|d| (i, d)))
        .collect();
    lex.sort_by(order);
    den.sort_by(order);
    lex.truncate(pool);
    den.truncate(pool);
    if lex.is_empty() {
        return den;
    }
    if den.is_empty() {
        return lex;
    }
    let nl = zscore(&lex.iter().map(|x| x.1).collect::<Vec<_>>());
    let nd = zscore(&den.iter().map(|x| x.1).collect::<Vec<_>>());
    let floor_l = nl.iter().cloned().fold(f64::INFINITY, f64::min) - 1e-9;
    let floor_d = nd.iter().cloned().fold(f64::INFINITY, f64::min) - 1e-9;
    let mut score: HashMap<usize, (f64, f64)> = HashMap::new();
    for ((i, _), z) in lex.iter().zip(&nl) {
        score.entry(*i).or_insert((floor_d, floor_l)).1 = *z;
    }
    for ((i, _), z) in den.iter().zip(&nd) {
        score.entry(*i).or_insert((floor_d, floor_l)).0 = *z;
    }
    let mut out: Vec<(usize, f64)> = score
        .into_iter()
        .map(|(i, (d, l))| (i, alpha * d + (1.0 - alpha) * l))
        .collect();
    out.sort_by(order);
    out
}

pub fn search(
    c: &Connection,
    q: &str,
    limit: usize,
    f: &crate::records::Filters,
    models: &[PathBuf],
) -> Result<Value> {
    if q.len() > 512 || q.trim().is_empty() || !(1..=20).contains(&limit) {
        bail!("query must be 1..512 bytes and limit 1..20");
    }
    if models.is_empty() || models.len() > MAX_MODELS {
        bail!("give 1 to {MAX_MODELS} embedding models");
    }
    let helper = helper_hash()?;
    let mut recipes = Vec::new();
    for m in models {
        let (identity, manifest) = identity(m)?;
        let stored: String = c
            .query_row(
                "SELECT helper_sha256 FROM embedding_models WHERE identity=?1",
                [&identity],
                |r| r.get(0),
            )
            .context("model not indexed; run rebuild-embeddings explicitly")?;
        if stored != helper {
            bail!("embedding recipe changed; run rebuild-embeddings");
        }
        recipes.push((m.clone(), identity, manifest));
    }
    let structured = f.technology.is_some()
        || f.project.is_some()
        || f.tested_version.is_some()
        || f.status.is_some();
    let eligible = crate::records::eligible(c, f)?;
    // 1. lexical shortlist: chunks (Tantivy, FTS5 fallback) and structured records
    let expr = crate::app::match_expr(q);
    let chunk_lex = if structured {
        vec![]
    } else {
        crate::app::lexical_chunks(c, q, SHORTLIST)?
    };
    let record_lex = crate::records::lexical_hits(c, &expr, POOL, f)?;
    let mut lex: HashMap<(u8, i64), f64> = HashMap::new();
    for (id, s) in &chunk_lex {
        lex.insert((0, *id), *s);
    }
    for (id, s) in &record_lex {
        lex.insert((1, *id), *s);
    }
    // 2. query vectors, one per model (resident helpers)
    let mut qvecs = Vec::new();
    for (m, identity, _) in &recipes {
        let e = encode(m, &[q.to_string()], "query")?;
        check_identity(identity, &e)?;
        qvecs.push(e.vectors[0].clone());
    }
    // 3. dense scores from the stored int8 vectors. Up to SCAN_LIMIT entries every vector is scored (exact, finds paraphrases and
    //    other-language matches that share no word with the query); beyond that only the lexical shortlist is re-scored.
    let stored: i64 = c.query_row(
        "SELECT count(*) FROM embeddings WHERE model=?1",
        [&recipes[0].1],
        |r| r.get(0),
    )?;
    let scan = stored as usize <= scan_limit();
    let keep = |kind: &str, id: i64| match kind {
        "record" => eligible.contains(&id),
        _ => !structured,
    };
    let mut per_model: Vec<HashMap<(u8, i64), (String, f64)>> = Vec::new();
    for ((_, identity, _), qv) in recipes.iter().zip(&qvecs) {
        let mut found: HashMap<(u8, i64), (String, f64)> = HashMap::new();
        if scan {
            let mut s =
                c.prepare("SELECT kind,id,signature,vector FROM embeddings WHERE model=?1")?;
            let mut rows = s.query([identity])?;
            while let Some(r) = rows.next()? {
                let (kind, id, sig, blob): (String, i64, String, Vec<u8>) =
                    (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?);
                if keep(&kind, id) {
                    found.insert((u8::from(kind == "record"), id), (sig, dot_q(&blob, qv)?));
                }
            }
        } else {
            let mut s = c.prepare(
                "SELECT signature,vector FROM embeddings WHERE model=?1 AND kind=?2 AND id=?3",
            )?;
            for (kind, id) in lex.keys() {
                let name = if *kind == 1 { "record" } else { "chunk" };
                if let Some((sig, blob)) = s
                    .query_row(params![identity, name, id], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
                    })
                    .optional()?
                {
                    found.insert((*kind, *id), (sig, dot_q(&blob, qv)?));
                }
            }
        }
        per_model.push(found);
    }
    // blend the models: each model's cosines are z-scored over the entries every model has (same signature), then averaged
    let n = recipes.len();
    let mut keys: Vec<(u8, i64)> = per_model[0]
        .iter()
        .filter(|(k, (sig, _))| {
            per_model
                .iter()
                .all(|m| m.get(*k).is_some_and(|(s2, _)| s2 == sig))
        })
        .map(|(k, _)| *k)
        .collect();
    keys.sort();
    let mut blended = vec![0.0; keys.len()];
    for m in &per_model {
        let z = zscore(&keys.iter().map(|k| m[k].1).collect::<Vec<_>>());
        for (b, v) in blended.iter_mut().zip(&z) {
            *b += v / n as f64;
        }
    }
    let dense: HashMap<(u8, i64), (f64, &String)> = keys
        .iter()
        .zip(&blended)
        .map(|(k, b)| (*k, (*b, &per_model[0][k].0)))
        .collect();
    // 4. candidates: the lexical shortlist plus the best dense entries, each checked against the live source and its signature
    let mut order: Vec<((u8, i64), f64)> = dense.iter().map(|(k, (b, _))| (*k, *b)).collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut wanted: Vec<(u8, i64)> = lex.keys().cloned().collect();
    wanted.extend(order.iter().take(POOL).map(|(k, _)| *k));
    wanted.sort();
    wanted.dedup();
    let mut cands: Vec<Cand> = Vec::new();
    let mut chunk_row = c.prepare("SELECT s.sha256,s.revision,c.heading,c.text FROM chunks c JOIN sources s ON s.id=c.source_id WHERE c.id=?1 AND s.status='indexed'")?;
    let mut record_row = c.prepare("SELECT payload_sha256,revision FROM records WHERE id=?1")?;
    let mut with_vectors = 0usize;
    for (kind, id) in wanted {
        let live = if kind == 1 {
            record_row
                .query_row([id], |r| {
                    Ok(format!(
                        "{}:{}",
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?
                    ))
                })
                .optional()?
        } else {
            chunk_row
                .query_row([id], |r| {
                    let text = chunk_text(&r.get::<_, String>(2)?, &r.get::<_, String>(3)?);
                    Ok(chunk_signature(
                        &r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        &text,
                    ))
                })
                .optional()?
        };
        let Some(signature) = live else {
            continue; // deleted, or its source is no longer indexed
        };
        let d = dense
            .get(&(kind, id))
            .filter(|(_, sig)| **sig == signature)
            .map(|(b, _)| *b);
        with_vectors += usize::from(d.is_some());
        cands.push(Cand {
            kind,
            id,
            lex: lex.get(&(kind, id)).copied(),
            dense: d,
        });
    }
    let ranked = fuse(&cands, POOL, ALPHA);
    // 4. build the answer: same hit shape as lexical search, de-duplicated by content
    let mut results = Vec::new();
    let mut records = Vec::new();
    let mut seen = HashSet::new();
    for (i, score) in &ranked {
        if results.len() + records.len() >= limit {
            break;
        }
        let cand = &cands[*i];
        let mut v = if cand.kind == 1 {
            crate::records::get(c, cand.id, false)?
        } else {
            c.query_row(
                &format!("{} WHERE c.id=?1", crate::app::HIT),
                [cand.id],
                |r| crate::app::hit(r, false),
            )?
        };
        if cand.kind == 0 {
            let text: String =
                c.query_row("SELECT text FROM chunks WHERE id=?1", [cand.id], |r| {
                    r.get(0)
                })?;
            let key = format!(
                "{}:{}:{}:{}:{}",
                v["sha256"], v["heading"], v["page"], v["line"], text
            );
            if !seen.insert(key) {
                continue;
            }
        }
        v["fusion_score"] = json!(score);
        if cand.kind == 1 {
            records.push(v)
        } else {
            results.push(v)
        }
    }
    let manifests = recipes
        .iter()
        .map(|r| serde_json::from_str::<Value>(&r.2))
        .collect::<Result<Vec<_>, _>>()?;
    let identities: Vec<&String> = recipes.iter().map(|r| &r.1).collect();
    Ok(json!({
        "mode":"hybrid_cc","models":manifests,"model_sha256":identities,
        "results":results,"records":records,"answerability":"not_estimated",
        "ranking":format!("convex combination {ALPHA} dense + {:.2} BM25 (z-scores over the top {POOL}); dense = mean of {n} model(s), exact scan up to {SCAN_LIMIT} entries else over a BM25 shortlist of {SHORTLIST}; scores are not confidence", 1.0 - ALPHA),
        "strategy":if scan {"full_dense_scan"} else {"lexical_shortlist"},"candidates":cands.len(),"with_vectors":with_vectors,
        "freshness_policy":"point-in-time source/evidence hashes; stale citations remain explicitly labeled"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_identity_changed_between_request_and_response_is_refused() {
        let encoded = Encoded {
            vectors: vec![],
            model_sha256: "other-manifest".into(),
            truncated_at_limit: 0,
        };
        assert!(check_identity("pinned-manifest", &encoded).is_err());
        assert!(check_identity("other-manifest", &encoded).is_ok());
    }
    #[test]
    fn int8_round_trip_keeps_the_cosine_within_a_percent() {
        let mut v: Vec<f64> = (0..DIM)
            .map(|i| ((i * 37 % 101) as f64 - 50.0) / 50.0)
            .collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        v.iter_mut().for_each(|x| *x /= norm);
        let blob = quantize(&v);
        assert_eq!(blob.len(), 4 + DIM);
        let cos = dot_q(&blob, &v).unwrap();
        assert!((cos - 1.0).abs() < 0.01, "{cos}");
        assert!(dot_q(&blob[..10], &v).is_err());
        assert!(dot_q(&[0u8; 4 + DIM], &v).is_err()); // zero scale
        let mut zero = quantize(&v);
        zero[4..].fill(0);
        assert!(dot_q(&zero, &v).is_err()); // zero norm
    }
    fn cand(kind: u8, id: i64, lex: Option<f64>, dense: Option<f64>) -> Cand {
        Cand {
            kind,
            id,
            lex,
            dense,
        }
    }
    #[test]
    fn fusion_blends_both_signals_and_breaks_ties_by_id() {
        // doc 1 is best lexically, doc 2 best densely, doc 3 is decent on both: the blend must put 3 first at alpha 0.5.
        let c = vec![
            cand(0, 1, Some(9.0), Some(-1.0)),
            cand(0, 2, Some(1.0), Some(2.0)),
            cand(0, 3, Some(7.0), Some(1.5)),
        ];
        let out: Vec<i64> = fuse(&c, 100, 0.5).iter().map(|(i, _)| c[*i].id).collect();
        assert_eq!(out[0], 3);
        let tie = vec![
            cand(0, 5, Some(1.0), Some(1.0)),
            cand(0, 4, Some(1.0), Some(1.0)),
        ];
        let out: Vec<i64> = fuse(&tie, 100, 0.55)
            .iter()
            .map(|(i, _)| tie[*i].id)
            .collect();
        assert_eq!(out, vec![4, 5]);
    }
    #[test]
    fn fusion_falls_back_to_the_signal_that_exists() {
        let only_dense = vec![cand(0, 1, None, Some(0.1)), cand(0, 2, None, Some(0.9))];
        let out: Vec<i64> = fuse(&only_dense, 100, 0.55)
            .iter()
            .map(|(i, _)| only_dense[*i].id)
            .collect();
        assert_eq!(out, vec![2, 1]);
        let only_lex = vec![cand(0, 1, Some(1.0), None), cand(0, 2, Some(3.0), None)];
        let out: Vec<i64> = fuse(&only_lex, 100, 0.55)
            .iter()
            .map(|(i, _)| only_lex[*i].id)
            .collect();
        assert_eq!(out, vec![2, 1]);
        assert!(fuse(&[], 100, 0.55).is_empty());
    }
    #[test]
    fn zscore_of_a_constant_is_zero_not_nan() {
        assert_eq!(zscore(&[2.0, 2.0, 2.0]), vec![0.0, 0.0, 0.0]);
    }
}
