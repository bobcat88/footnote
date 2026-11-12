use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

pub const MIGRATION: &str = "CREATE TABLE embedding_models(identity TEXT PRIMARY KEY,manifest TEXT NOT NULL,helper_sha256 TEXT NOT NULL,dimension INTEGER NOT NULL); CREATE TABLE embeddings(model TEXT NOT NULL REFERENCES embedding_models(identity),kind TEXT NOT NULL,id INTEGER NOT NULL,signature TEXT NOT NULL,vector TEXT NOT NULL,PRIMARY KEY(model,kind,id));";
const MAX_ENTRIES: usize = 10000;
#[derive(Deserialize)]
struct Encoded {
    vectors: Vec<Vec<f64>>,
    model_sha256: String,
    truncated_at_limit: usize,
    runtime: Value,
}
fn check_identity(expected: &str, encoded: &Encoded) -> Result<()> {
    if encoded.model_sha256 != expected {
        bail!("embedding model changed during request; retry with immutable model artifacts");
    }
    Ok(())
}
struct Entry {
    kind: String,
    id: i64,
    signature: String,
    text: String,
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
fn encode(model: &Path, texts: &[String], kind: &str) -> Result<Encoded> {
    if texts.is_empty() || texts.len() > MAX_ENTRIES {
        bail!("embedding batch must contain 1..10000 entries");
    }
    let dir = tempfile::tempdir_in(crate::app::scratch_dir()?)?;
    let input = dir.path().join("input.json");
    let output = dir.path().join("output.json");
    let request = json!({"kind":kind,"texts":texts}).to_string();
    if request.len() > 64_000_000 {
        bail!("embedding request exceeds 64 MB");
    }
    fs::write(&input, request)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let python = std::env::var_os("BORG_KNOWLEDGE_PYTHON")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            root.join(if cfg!(windows) {
                ".venv/Scripts/python.exe"
            } else {
                ".venv/bin/python"
            })
        });
    let mut cmd = Command::new(python);
    cmd.arg("-I")
        .arg(root.join("tools/embed.py"))
        .arg(model)
        .arg(input)
        .stdout(fs::File::create(&output)?)
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    // Embedding helper is also scrubbed: no agent credentials in a text parser.
    crate::app::helper_env(&mut cmd);
    let mut child = cmd.spawn()?;
    let Some(status) = child.wait_timeout(Duration::from_secs(600))? else {
        child.kill()?;
        child.wait()?;
        bail!("embedding helper timed out (600s)");
    };
    if fs::metadata(&output)?.len() > 128_000_000 {
        bail!("embedding helper response exceeds 128 MB");
    }
    let bytes = fs::read(output)?;
    if !status.success() {
        bail!(
            "embedding helper failed: {}",
            String::from_utf8_lossy(&bytes)
                .chars()
                .take(1000)
                .collect::<String>()
        );
    }
    let result: Encoded = serde_json::from_slice(&bytes)?;
    if result.model_sha256 != identity(model)?.0
        || result.vectors.len() != texts.len()
        || result.vectors.iter().any(|v| {
            v.len() != 384
                || v.iter().any(|x| !x.is_finite())
                || (v.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() > 0.01
        })
    {
        bail!("embedding model/dimension/norm mismatch");
    }
    Ok(result)
}
fn entries(c: &Connection) -> Result<Vec<Entry>> {
    let mut result = Vec::new();
    let mut s = c.prepare("SELECT c.id,s.sha256,s.revision,c.heading,c.text FROM chunks c JOIN sources s ON s.id=c.source_id WHERE s.status='indexed' ORDER BY c.id LIMIT 10001")?;
    for row in s.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })? {
        let (id, sha, revision, heading, text) = row?;
        let text = format!("{heading}\n{text}");
        result.push(Entry {
            kind: "chunk".into(),
            id,
            signature: crate::app::hash(format!("{sha}:{revision}:{text}").as_bytes()),
            text,
        });
    }
    let mut s = c.prepare(
        "SELECT id,payload_sha256,revision,payload FROM records ORDER BY id LIMIT 10001",
    )?;
    for row in s.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
        ))
    })? {
        let (id, sha, revision, payload) = row?;
        let p: Value = serde_json::from_str(&payload)?;
        let text = [
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
        .join("\n");
        result.push(Entry {
            kind: "record".into(),
            id,
            signature: format!("{sha}:{revision}"),
            text,
        });
    }
    if result.len() > MAX_ENTRIES {
        bail!("local exact-vector scan limited to 10000 chunks plus records; narrow corpus");
    }
    Ok(result)
}
pub fn rebuild(c: &mut Connection, model: &Path) -> Result<Value> {
    let (identity, manifest) = identity(model)?;
    // Lock corpus/record revisions for the complete rebuild; readers remain read-only.
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let entries = entries(&tx)?;
    let encoded = if entries.is_empty() {
        None
    } else {
        Some(encode(
            model,
            &entries.iter().map(|e| e.text.clone()).collect::<Vec<_>>(),
            "passage",
        )?)
    };
    if let Some(ref encoded) = encoded {
        check_identity(&identity, encoded)?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO embedding_models VALUES(?1,?2,?3,384)",
        params![identity, manifest, helper_hash()?],
    )?;
    tx.execute("DELETE FROM embeddings WHERE model=?1", [&identity])?;
    if let Some(ref encoded) = encoded {
        for (entry, vector) in entries.iter().zip(&encoded.vectors) {
            tx.execute(
                "INSERT INTO embeddings VALUES(?1,?2,?3,?4,?5)",
                params![
                    identity,
                    entry.kind,
                    entry.id,
                    entry.signature,
                    serde_json::to_string(vector)?
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(
        json!({"embedded":entries.len(),"model_sha256":identity,"dimension":384,"truncated_at_limit":encoded.as_ref().map(|e|e.truncated_at_limit).unwrap_or(0),"runtime":encoded.map(|e|e.runtime),"derived_only":true}),
    )
}
pub fn search(
    c: &Connection,
    q: &str,
    limit: usize,
    f: &crate::records::Filters,
    model: &Path,
) -> Result<Value> {
    if q.len() > 512 || q.trim().is_empty() || !(1..=20).contains(&limit) {
        bail!("query must be 1..512 bytes and limit 1..20");
    }
    let (identity, manifest) = identity(model)?;
    let stored: String = c
        .query_row(
            "SELECT helper_sha256 FROM embedding_models WHERE identity=?1",
            [&identity],
            |r| r.get(0),
        )
        .context("model not indexed; run rebuild-embeddings explicitly")?;
    if stored != helper_hash()? {
        bail!("embedding recipe changed; run rebuild-embeddings");
    }
    let eligible = crate::records::eligible(c, f)?;
    let structured = f.technology.is_some()
        || f.project.is_some()
        || f.tested_version.is_some()
        || f.status.is_some();
    let entries = entries(c)?;
    let live: HashMap<_, _> = entries
        .into_iter()
        .filter(|e| {
            if e.kind == "record" {
                eligible.contains(&e.id)
            } else {
                !structured
            }
        })
        .map(|e| ((e.kind, e.id), e.signature))
        .collect();
    let encoded = encode(model, &[q.to_string()], "query")?;
    check_identity(&identity, &encoded)?;
    let vector = &encoded.vectors[0];
    let mut semantic = Vec::new();
    let mut s=c.prepare("SELECT kind,id,signature,vector FROM embeddings WHERE model=?1 ORDER BY kind,id LIMIT 10001")?;
    let mut scanned = 0;
    for row in s.query_map([&identity], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })? {
        scanned += 1;
        let (kind, id, signature, values) = row?;
        if live.get(&(kind.clone(), id)) != Some(&signature) {
            continue;
        }
        let v: Vec<f64> = serde_json::from_str(&values)?;
        if v.len() != vector.len()
            || v.iter().any(|x| !x.is_finite())
            || (v.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() > 0.01
        {
            bail!("corrupt derived vector; rebuild embeddings");
        }
        let score: f64 = v.iter().zip(vector).map(|(a, b)| a * b).sum();
        semantic.push(((kind, id), score));
    }
    if scanned > MAX_ENTRIES {
        bail!("derived vector bound exceeded; rebuild on narrower corpus");
    }
    semantic.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    // No similarity threshold is represented as calibrated answerability.
    semantic.truncate(60);
    let lexical = crate::app::search(c, q, 20, f)?;
    let mut ranks: HashMap<(String, i64), f64> = HashMap::new();
    for (rank, (key, _)) in semantic.iter().enumerate() {
        *ranks.entry(key.clone()).or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
    }
    for (kind, field) in [("record", "records"), ("chunk", "results")] {
        for (rank, v) in lexical[field].as_array().unwrap().iter().enumerate() {
            *ranks
                .entry((kind.into(), v["id"].as_i64().unwrap()))
                .or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
        }
    }
    let mut ranked: Vec<_> = ranks.into_iter().collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut results = Vec::new();
    let mut records = Vec::new();
    let mut seen = HashSet::new();
    for ((kind, id), score) in ranked {
        if results.len() + records.len() >= limit {
            break;
        }
        let mut v = if kind == "record" {
            crate::records::get(c, id, false)?
        } else {
            c.query_row(&format!("{} WHERE c.id=?1", crate::app::HIT), [id], |r| {
                crate::app::hit(r, false)
            })?
        };
        if kind == "chunk" {
            let text: String =
                c.query_row("SELECT text FROM chunks WHERE id=?1", [id], |r| r.get(0))?;
            let key = format!(
                "{}:{}:{}:{}:{}",
                v["sha256"], v["heading"], v["page"], v["line"], text
            );
            if !seen.insert(key) {
                continue;
            }
        }
        v["rrf_score"] = json!(score);
        if kind == "record" {
            records.push(v)
        } else {
            results.push(v)
        }
    }
    Ok(
        json!({"mode":"hybrid_rrf","model":serde_json::from_str::<Value>(&manifest)?,"model_sha256":identity,"results":results,"records":records,"answerability":"not_estimated","ranking":"RRF k=60; lexical top20 + cosine top60; scores are not confidence","eligible_embeddings":semantic.len(),"eligible_items":live.len(),"freshness_policy":"point-in-time source/evidence hashes; stale citations remain explicitly labeled"}),
    )
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
            runtime: json!({}),
        };
        assert!(check_identity("pinned-manifest", &encoded).is_err());
        assert!(check_identity("other-manifest", &encoded).is_ok());
    }
}
