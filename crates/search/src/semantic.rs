//! Hybrid search: lexical shortlist, dense re-scoring with one or two local models, convex-combination fusion.
//!
//! Design (bench/experiments/RESULTS.md): a Tantivy BM25 shortlist (top 1000 chunks) is re-scored by the dense model(s) from int8 vectors
//! stored in SQLite, so a query never scans the corpus and memory does not grow with it. Two models are blended 50/50 (z-scores over the
//! shortlist), then fused with the BM25 scores: `0.55 * dense + 0.45 * lexical`, each z-scored over its own top-100. Query encoding goes
//! through a resident helper process (one per model), so the model is loaded once and a query costs milliseconds.
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

pub const MIGRATION: &str = "CREATE TABLE embedding_models(identity TEXT PRIMARY KEY,manifest TEXT NOT NULL,helper_sha256 TEXT NOT NULL,dimension INTEGER NOT NULL); CREATE TABLE embeddings(model TEXT NOT NULL REFERENCES embedding_models(identity),kind TEXT NOT NULL,id INTEGER NOT NULL,signature TEXT NOT NULL,vector TEXT NOT NULL,PRIMARY KEY(model,kind,id));";
/// The shipped models' width; every model's own width comes from its manifest and is stored in `embedding_models.dimension`.
#[cfg(test)]
const DIM: usize = 384;
const SHORTLIST: usize = 1000;
const POOL: usize = 100;
const SCAN_LIMIT: usize = 50_000;
/// Entries up to which every stored vector is scored. `FOOTNOTE_SCAN_LIMIT` lowers it (tests, very small machines).
fn scan_limit() -> usize {
    std::env::var("FOOTNOTE_SCAN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .map_or(SCAN_LIMIT, |n: usize| n.min(SCAN_LIMIT))
}
const ALPHA: f64 = 0.55;
const BATCH: usize = 128;
const MAX_MODELS: usize = 3;

struct Encoded {
    vectors: Vec<Vec<f64>>,
    model_sha256: String,
    truncated_at_limit: usize,
    dimension: usize,
}
fn check_identity(expected: &str, encoded: &Encoded) -> Result<()> {
    if encoded.model_sha256 != expected {
        bail!("embedding model changed during request; retry with immutable model artifacts");
    }
    Ok(())
}
fn identity(model: &Path) -> Result<(String, String)> {
    footnote_core::no_links(model)?;
    let manifest = String::from_utf8(footnote_core::bounded_read(&model.join("manifest.json"))?)?;
    let runtime = serde_json::from_str::<Value>(&manifest)?["runtime"]
        .as_str()
        .unwrap_or("onnx")
        .to_string();
    for name in crate::encoder::files(&runtime) {
        footnote_core::no_links(&model.join(name))?;
    }
    Ok((footnote_core::hash(manifest.as_bytes()), manifest))
}
/// Identity of the embedding recipe (code, not model), stored with each model's vectors: when it changes, stored vectors are rebuilt.
/// It is still the SHA-256 of the last Python helper (`tools/embed.py`): the Rust encoder gives the same vectors (cosine 1.0) and the
/// same rankings (bench/experiments/BUILD-SPEED.md, test 4), so databases built with the helper stay valid.
const RECIPE: &str = "d69ec768e02c6700472d7d8fed979fa67f576b276f939eee792a9d392ecfcccb";
fn helper_hash() -> Result<String> {
    Ok(RECIPE.to_string())
}

// ------------------------------------------------------------------ resident encoders
/// One loaded encoder per model directory, for the life of the process (the MCP server, the daemon): loading costs about a second,
/// a query a few milliseconds.
static ENCODERS: OnceLock<Mutex<HashMap<PathBuf, crate::encoder::Encoder>>> = OnceLock::new();

fn encode_batch(model: &Path, expected: &str, texts: &[String], kind: &str) -> Result<Encoded> {
    let pool = ENCODERS.get_or_init(Default::default);
    let mut guard = pool
        .lock()
        .map_err(|_| anyhow::anyhow!("encoder pool poisoned"))?;
    if guard.get(model).is_some_and(|e| e.model_sha256 != expected) {
        guard.remove(model); // the manifest changed on disk since it was loaded: load the new model
    }
    if !guard.contains_key(model) {
        let e = crate::metrics::timed!("encoder.load", crate::encoder::Encoder::load(model))?;
        guard.insert(model.to_path_buf(), e);
    }
    let encoder = guard.get_mut(model).expect("inserted above");
    let e = crate::metrics::timed!(
        if kind == "query" {
            "encoder.query"
        } else {
            "encoder.passages"
        },
        encoder.encode(texts, kind)
    )?;
    Ok(Encoded {
        vectors: e.vectors,
        model_sha256: encoder.model_sha256.clone(),
        truncated_at_limit: e.truncated_at_limit,
        dimension: encoder.dimension(),
    })
}
fn encode(model: &Path, texts: &[String], kind: &str) -> Result<Encoded> {
    if texts.is_empty() {
        bail!("embedding batch must not be empty");
    }
    let (expected, _) = crate::metrics::timed!("model.identity", identity(model))?;
    let mut merged: Option<Encoded> = None;
    for part in texts.chunks(BATCH) {
        let mut e = encode_batch(model, &expected, part, kind)?;
        check_identity(&expected, &e)?;
        if e.vectors.len() != part.len()
            || e.vectors.iter().any(|v| {
                v.len() != e.dimension
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

/// The `embed` command: the full-precision vectors the index would store, before int8 quantization.
pub fn embed(model: &Path, texts: &[String], kind: &str) -> Result<Value> {
    let t = std::time::Instant::now();
    let e = encode(model, texts, kind)?;
    Ok(
        json!({"vectors": e.vectors, "model_sha256": e.model_sha256, "truncated_at_limit": e.truncated_at_limit,
              "seconds": t.elapsed().as_secs_f64()}),
    )
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
/// The scale of a stored vector, or `Err` when it is not a plausible unit vector (wrong length, non-finite or zero scale, norm far
/// from 1): derived data that was damaged is refused, never ranked.
fn check_blob(blob: &[u8]) -> Result<f32> {
    if blob.len() < 5 {
        bail!("corrupt derived vector; rebuild embeddings");
    }
    let scale = f32::from_le_bytes(blob[..4].try_into()?);
    let norm = blob[4..]
        .iter()
        .map(|b| ((*b as i8) as f64).powi(2))
        .sum::<f64>()
        .sqrt()
        * scale as f64;
    if !scale.is_finite() || scale <= 0.0 || (norm - 1.0).abs() > 0.05 {
        bail!("corrupt derived vector; rebuild embeddings");
    }
    Ok(scale)
}
/// Cosine of a stored vector with a unit query vector (see `check_blob` for what is refused).
fn dot_q(blob: &[u8], q: &[f64]) -> Result<f64> {
    let scale = check_blob(blob)? as f64;
    if blob.len() != 4 + q.len() {
        bail!("query vector has the wrong dimension");
    }
    Ok(blob[4..]
        .iter()
        .zip(q)
        .map(|(b, x)| (*b as i8) as f64 * x)
        .sum::<f64>()
        * scale)
}

// ------------------------------------------------------------------ resident vector matrix
/// Every stored vector of one model, decoded once per process: a long-lived session (the MCP front-end, the daemon) pays the load on the
/// first hybrid query and then scores 50,000 entries in a few milliseconds instead of re-reading them from SQLite on every query.
struct Matrix {
    dim: usize,
    keys: Vec<(u8, i64)>,
    sigs: Vec<Arc<str>>,
    scale: Vec<f32>,
    q: Vec<i8>,
}
type MatrixKey = (String, String);
type MatrixEntry = ((i64, i64), Arc<Matrix>);
static MATRICES: OnceLock<Mutex<HashMap<MatrixKey, MatrixEntry>>> = OnceLock::new();

/// The vectors of `identity`, from the cache when its fingerprint (row count, highest rowid) still matches. Every rebuild that adds,
/// replaces or removes a vector changes one of the two; damaged vectors are refused while loading, never ranked.
fn matrix(c: &Connection, identity: &str) -> Result<Arc<Matrix>> {
    let fingerprint: (i64, i64) = c.query_row(
        "SELECT count(*),COALESCE(max(rowid),0) FROM embeddings WHERE model=?1",
        [identity],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let key = (c.path().unwrap_or("").to_string(), identity.to_string());
    let cache = MATRICES.get_or_init(|| Mutex::new(HashMap::new()));
    if !key.0.is_empty()
        && let Some((fp, m)) = cache
            .lock()
            .map_err(|_| anyhow::anyhow!("vector cache poisoned"))?
            .get(&key)
        && *fp == fingerprint
    {
        return Ok(m.clone());
    }
    let mut m = Matrix {
        dim: 0,
        keys: Vec::new(),
        sigs: Vec::new(),
        scale: Vec::new(),
        q: Vec::new(),
    };
    let mut s = c.prepare(
        "SELECT kind,id,signature,vector FROM embeddings WHERE model=?1 ORDER BY kind,id",
    )?;
    let mut rows = s.query([identity])?;
    while let Some(r) = rows.next()? {
        let (kind, id, sig, blob): (String, i64, String, Vec<u8>) =
            (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?);
        let scale = check_blob(&blob)?;
        if m.dim == 0 {
            m.dim = blob.len() - 4;
        } else if blob.len() - 4 != m.dim {
            bail!("corrupt derived vector; rebuild embeddings");
        }
        m.keys.push((u8::from(kind == "record"), id));
        m.sigs.push(Arc::from(sig));
        m.scale.push(scale);
        m.q.extend(blob[4..].iter().map(|b| *b as i8));
    }
    let m = Arc::new(m);
    if !key.0.is_empty() {
        cache
            .lock()
            .map_err(|_| anyhow::anyhow!("vector cache poisoned"))?
            .insert(key, (fingerprint, m.clone()));
    }
    Ok(m)
}

// ------------------------------------------------------------------ entries
fn chunk_text(heading: &str, text: &str) -> String {
    format!("{heading}\n{text}")
}
fn chunk_signature(sha: &str, revision: i64, text: &str) -> String {
    footnote_core::hash(format!("{sha}:{revision}:{text}").as_bytes())
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
    let mut dimensions = Vec::new();
    for model in models {
        let (identity, manifest) = identity(model)?;
        // Start (or reuse) the helper first: it verifies the model files against the manifest, even when nothing needs embedding.
        let probe = encode(model, &["verify".to_string()], "query")?;
        check_identity(&identity, &probe)?;
        dimensions.push(probe.dimension);
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
            params![identity, manifest, helper, probe.dimension as i64],
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
        json!({"embedded":embedded_total,"models":report,"dimension":dimensions[0],"dimensions":dimensions,"storage":"int8","derived_only":true}),
    )
}

// ------------------------------------------------------------------ dense scores
/// The blended dense score of every entry that has a current vector in every model: per model the cosines are z-scored over those
/// entries, then averaged. `z` is NaN where an entry has no usable vector (missing from a model, other signature, filtered out).
/// Keys are sorted, so a lookup is a binary search and a query never builds a hash map over the corpus.
struct Dense {
    keys: Vec<(u8, i64)>,
    sigs: Vec<Arc<str>>,
    z: Vec<f64>,
}
impl Dense {
    fn blend(keys: Vec<(u8, i64)>, sigs: Vec<Arc<str>>, cos: Vec<Vec<f32>>) -> Dense {
        let mut z = vec![0.0; keys.len()];
        for per_model in &cos {
            let ok: Vec<f64> = per_model
                .iter()
                .filter(|c| !c.is_nan())
                .map(|c| *c as f64)
                .collect();
            let n = ok.len().max(1) as f64;
            let mean = ok.iter().sum::<f64>() / n;
            let var = ok.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            let sd = if var > 0.0 { var.sqrt() } else { 1.0 };
            for (zi, c) in z.iter_mut().zip(per_model) {
                *zi += (*c as f64 - mean) / sd / cos.len() as f64; // NaN stays NaN
            }
        }
        Dense { keys, sigs, z }
    }
    /// Exact: score every stored vector of every model (eligible entries only).
    fn scan(mats: &[Arc<Matrix>], qvecs: &[Vec<f64>], keep: impl Fn(&str, i64) -> bool) -> Dense {
        let m0 = &mats[0];
        let mut cos: Vec<Vec<f32>> = Vec::with_capacity(mats.len());
        let mut valid: Vec<bool> = m0
            .keys
            .iter()
            .map(|(kind, id)| keep(if *kind == 1 { "record" } else { "chunk" }, *id))
            .collect();
        for (mi, (m, qv)) in mats.iter().zip(qvecs).enumerate() {
            let qf: Vec<f32> = qv.iter().map(|x| *x as f32).collect();
            let mut v = vec![f32::NAN; m0.keys.len()];
            for i in 0..m0.keys.len() {
                if !valid[i] {
                    continue;
                }
                let j = if mi == 0 {
                    Some(i)
                } else {
                    m.keys.binary_search(&m0.keys[i]).ok()
                };
                match j {
                    Some(j) if m.sigs[j] == m0.sigs[i] => {
                        let row = &m.q[j * m.dim..(j + 1) * m.dim];
                        v[i] = row.iter().zip(&qf).map(|(b, x)| *b as f32 * x).sum::<f32>()
                            * m.scale[j];
                    }
                    _ => valid[i] = false,
                }
            }
            cos.push(v);
        }
        for v in &mut cos {
            for (c, ok) in v.iter_mut().zip(&valid) {
                if !ok {
                    *c = f32::NAN;
                }
            }
        }
        Dense::blend(m0.keys.clone(), m0.sigs.clone(), cos)
    }
    /// Large corpora: only the lexical shortlist is scored, from vectors fetched by id.
    fn shortlist(
        c: &Connection,
        recipes: &[(PathBuf, String, String)],
        qvecs: &[Vec<f64>],
        lex: &HashMap<(u8, i64), f64>,
    ) -> Result<Dense> {
        let mut keys: Vec<(u8, i64)> = lex.keys().cloned().collect();
        keys.sort();
        let mut s = c.prepare(
            "SELECT signature,vector FROM embeddings WHERE model=?1 AND kind=?2 AND id=?3",
        )?;
        let mut sigs: Vec<Arc<str>> = vec![Arc::from(""); keys.len()];
        let mut cos: Vec<Vec<f32>> = vec![vec![f32::NAN; keys.len()]; recipes.len()];
        let mut ok = vec![true; keys.len()];
        for (mi, ((_, identity, _), qv)) in recipes.iter().zip(qvecs).enumerate() {
            for (i, (kind, id)) in keys.iter().enumerate() {
                if !ok[i] {
                    continue;
                }
                let name = if *kind == 1 { "record" } else { "chunk" };
                let row: Option<(String, Vec<u8>)> = s
                    .query_row(params![identity, name, id], |r| Ok((r.get(0)?, r.get(1)?)))
                    .optional()?;
                match row {
                    Some((sig, blob)) if mi == 0 || *sigs[i] == *sig => {
                        if mi == 0 {
                            sigs[i] = Arc::from(sig);
                        }
                        cos[mi][i] = dot_q(&blob, qv)? as f32;
                    }
                    _ => ok[i] = false,
                }
            }
        }
        for v in &mut cos {
            for (c, good) in v.iter_mut().zip(&ok) {
                if !good {
                    *c = f32::NAN;
                }
            }
        }
        Ok(Dense::blend(keys, sigs, cos))
    }
    fn get(&self, key: &(u8, i64)) -> Option<(f64, &Arc<str>)> {
        let i = self.keys.binary_search(key).ok()?;
        (!self.z[i].is_nan()).then(|| (self.z[i], &self.sigs[i]))
    }
    /// The `n` best keys by blended score (ties by key).
    fn top(&self, n: usize) -> Vec<(u8, i64)> {
        let mut idx: Vec<usize> = (0..self.keys.len())
            .filter(|i| !self.z[*i].is_nan())
            .collect();
        let cmp = |a: &usize, b: &usize| {
            self.z[*b]
                .total_cmp(&self.z[*a])
                .then(self.keys[*a].cmp(&self.keys[*b]))
        };
        if idx.len() > n {
            idx.select_nth_unstable_by(n, cmp);
            idx.truncate(n);
        }
        idx.sort_by(cmp);
        idx.into_iter().map(|i| self.keys[i]).collect()
    }
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

/// The model directories among `dirs` whose vectors this database holds (same manifest identity), at most three, in the given
/// order: what hybrid search uses when the caller names no model.
pub fn embedded_with(c: &Connection, dirs: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let has = |id: &str| {
        c.query_row(
            "SELECT EXISTS(SELECT 1 FROM embeddings WHERE model=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )
        .unwrap_or(false) // a database without the embeddings table has no vectors
    };
    dirs.into_iter()
        .filter(|d| identity(d).is_ok_and(|(id, _)| has(&id)))
        .take(MAX_MODELS)
        .collect()
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
    let expr = crate::store::match_expr(q);
    let allowed = |id: &i64| f.only.as_ref().is_none_or(|o| o.contains(id));
    let chunk_lex: Vec<(i64, f64)> = if structured {
        vec![]
    } else {
        crate::metrics::timed!(
            "search.keywords",
            crate::store::lexical_chunks(c, q, SHORTLIST)
        )?
        .into_iter()
        .filter(|(id, _)| allowed(id))
        .collect()
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
        let e =
            crate::metrics::timed!("search.query_vector", encode(m, &[q.to_string()], "query"))?;
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
        _ => !structured && allowed(&id),
    };
    let dense = crate::metrics::timed!(
        "search.vectors",
        (|| -> Result<_> {
            Ok(if scan {
                let mats = recipes
                    .iter()
                    .map(|(_, identity, _)| matrix(c, identity))
                    .collect::<Result<Vec<_>>>()?;
                if mats
                    .iter()
                    .zip(&qvecs)
                    .any(|(m, qv)| !m.keys.is_empty() && m.dim != qv.len())
                {
                    bail!("stored vectors do not match the model's width; rebuild embeddings");
                }
                Dense::scan(&mats, &qvecs, |kind, id| keep(kind, id))
            } else {
                Dense::shortlist(c, &recipes, &qvecs, &lex)?
            })
        })()
    )?;
    // 4. candidates: the lexical shortlist plus the best dense entries, each checked against the live source and its signature
    // Only the best lexical entries can reach the fusion (it keeps the top POOL by score); twice that many are checked so that
    // entries dropped as stale or deleted are replaced by the next ones. Validating the whole shortlist cost one query and one hash each.
    let mut lex_order: Vec<((u8, i64), f64)> = lex.iter().map(|(k, v)| (*k, *v)).collect();
    lex_order.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut wanted: Vec<(u8, i64)> = lex_order.iter().take(2 * POOL).map(|(k, _)| *k).collect();
    wanted.extend(dense.top(POOL));
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
            .filter(|(_, sig)| ***sig == *signature)
            .map(|(b, _)| b);
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
    let mut cap = crate::store::PerSource::new(f.per_source);
    for (i, score) in &ranked {
        if results.len() + records.len() >= limit {
            break;
        }
        let cand = &cands[*i];
        let mut v = if cand.kind == 1 {
            crate::records::get(c, cand.id, false)?
        } else {
            c.query_row(
                &format!("{} WHERE c.id=?1", crate::store::HIT),
                [cand.id],
                |r| crate::store::hit(r, false),
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
            if !cap.allow(v["path"].as_str().unwrap_or("")) {
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
    Ok(json!({
        "mode":"hybrid_cc","results":crate::store::lean(results),"records":records,"trust":crate::store::TRUST,
        "strategy":if scan {"full_dense_scan"} else {"lexical_shortlist"},"candidates":cands.len(),"with_vectors":with_vectors
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
            dimension: DIM,
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
    fn the_vector_matrix_is_cached_and_follows_rebuilds() {
        let t = tempfile::tempdir().unwrap();
        let c = Connection::open(t.path().join("k.db")).unwrap();
        c.execute_batch(MIGRATION).unwrap();
        c.execute("INSERT INTO embedding_models VALUES('m','{}','h',384)", [])
            .unwrap();
        let unit: Vec<f64> = (0..DIM).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
        let add = |id: i64| {
            c.execute(
                "INSERT INTO embeddings VALUES('m','chunk',?1,'sig',?2)",
                params![id, quantize(&unit)],
            )
            .unwrap();
        };
        add(1);
        let first = matrix(&c, "m").unwrap();
        assert!(
            Arc::ptr_eq(&first, &matrix(&c, "m").unwrap()),
            "same fingerprint, same matrix"
        );
        assert_eq!(first.keys, vec![(0, 1)]);
        add(2);
        let second = matrix(&c, "m").unwrap();
        assert!(
            !Arc::ptr_eq(&first, &second),
            "a new vector must invalidate the cache"
        );
        assert_eq!(second.keys.len(), 2);
        c.execute("UPDATE embeddings SET vector=zeroblob(388) WHERE id=2", [])
            .unwrap();
        c.execute("DELETE FROM embeddings WHERE id=1", []).unwrap();
        assert!(
            matrix(&c, "m").is_err(),
            "a damaged vector is refused while loading"
        );
    }
    #[test]
    fn a_model_takes_its_width_from_its_vectors_and_mixed_widths_are_refused() {
        let t = tempfile::tempdir().unwrap();
        let c = Connection::open(t.path().join("k.db")).unwrap();
        c.execute_batch(MIGRATION).unwrap();
        c.execute("INSERT INTO embedding_models VALUES('m','{}','h',768)", [])
            .unwrap();
        let unit =
            |dim: usize| -> Vec<f64> { (0..dim).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect() };
        c.execute(
            "INSERT INTO embeddings VALUES('m','chunk',1,'sig',?1)",
            params![quantize(&unit(768))],
        )
        .unwrap();
        assert_eq!(matrix(&c, "m").unwrap().dim, 768);
        assert!((dot_q(&quantize(&unit(768)), &unit(768)).unwrap() - 1.0).abs() < 1e-3);
        assert!(
            dot_q(&quantize(&unit(768)), &unit(384)).is_err(),
            "a query of another width is refused"
        );
        c.execute(
            "INSERT INTO embeddings VALUES('m','chunk',2,'sig',?1)",
            params![quantize(&unit(384))],
        )
        .unwrap();
        assert!(matrix(&c, "m").is_err(), "one model, two widths: refused");
    }
    #[test]
    fn zscore_of_a_constant_is_zero_not_nan() {
        assert_eq!(zscore(&[2.0, 2.0, 2.0]), vec![0.0, 0.0, 0.0]);
    }
}
