use crate::app::{bounded_read, hash};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub const MIGRATION: &str = "CREATE TABLE records(id INTEGER PRIMARY KEY AUTOINCREMENT,fingerprint TEXT NOT NULL UNIQUE,payload TEXT NOT NULL,payload_sha256 TEXT NOT NULL,status TEXT NOT NULL CHECK(status IN ('proposal','validated','superseded')),revision INTEGER NOT NULL,validation TEXT);
CREATE TABLE record_history(record_id INTEGER NOT NULL,revision INTEGER NOT NULL,snapshot TEXT NOT NULL,PRIMARY KEY(record_id,revision));
CREATE VIRTUAL TABLE records_fts USING fts5(body,tokenize='unicode61 remove_diacritics 2');";

fn str_field<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .filter(|s| !s.trim().is_empty())
        .with_context(|| format!("missing nonempty field: {k}"))
}
fn checked_path(p: &Path, allow: &[PathBuf]) -> Result<PathBuf> {
    crate::app::no_links(p)?;
    let path = std::fs::canonicalize(p)?;
    for a in allow {
        crate::app::no_links(a)?;
        if path.starts_with(std::fs::canonicalize(a)?) {
            return Ok(path);
        }
    }
    bail!("evidence/input is outside explicit allowlist")
}
fn verify_artifacts(v: &Value, allow: Option<&[PathBuf]>) -> Result<bool> {
    let refs = v
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 16)
        .context("require 1..16 evidence artifacts")?;
    let mut current = true;
    for a in refs {
        let path = Path::new(str_field(a, "path")?);
        if !path.is_absolute() {
            bail!("evidence paths must be absolute");
        }
        let sha = str_field(a, "sha256")?;
        if sha.len() != 64
            || !sha
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            bail!("sha256 must be 64 lowercase hex characters");
        }
        if let Some(allow) = allow {
            checked_path(path, allow)?;
        }
        current &= bounded_read(path).map(|b| hash(&b) == sha).unwrap_or(false);
    }
    Ok(current)
}
pub fn propose(c: &mut Connection, file: &Path, allow: &[PathBuf]) -> Result<Value> {
    let bytes = bounded_read(&checked_path(file, allow)?)?;
    if bytes.len() > 64_000 {
        bail!("record exceeds 64k bytes");
    }
    let payload: Value = serde_json::from_slice(&bytes)?;
    for key in [
        "technology",
        "project",
        "environment",
        "symptom",
        "root_cause",
        "root_cause_status",
        "remediation",
        "applicability",
        "exclusions",
        "protocol",
        "tradeoffs",
        "rollback",
        "last_verified",
    ] {
        str_field(&payload, key)?;
    }
    let versions = payload["tested_versions"]
        .as_object()
        .filter(|m| !m.is_empty())
        .context("exact tested_versions map required")?;
    for v in versions.values() {
        if v.as_str().is_none_or(|s| {
            s.trim().is_empty() || s.contains('*') || s.contains('>') || s == "latest"
        }) {
            bail!("tested versions must be exact strings");
        }
    }
    if !verify_artifacts(&payload["evidence"], Some(allow))? {
        bail!("evidence hash mismatch or missing artifact");
    }
    let fingerprint = hash(
        serde_json::to_string(&json!([
            payload["technology"],
            payload["project"],
            payload["tested_versions"],
            payload["environment"],
            payload["root_cause"]
        ]))?
        .as_bytes(),
    );
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if let Some((id, old_sha)) = tx
        .query_row(
            "SELECT id,payload_sha256 FROM records WHERE fingerprint=?1",
            [&fingerprint],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?
    {
        let candidate = hash(serde_json::to_string(&payload)?.as_bytes());
        if candidate != old_sha {
            bail!(
                "deduplication conflict: same cause/context with different payload; propose distinct context or supersede explicitly"
            );
        }
        return get(&tx, id, true);
    }
    let encoded = serde_json::to_string(&payload)?;
    let sha = hash(encoded.as_bytes());
    tx.execute("INSERT INTO records(fingerprint,payload,payload_sha256,status,revision) VALUES(?1,?2,?3,'proposal',1)",params![fingerprint,encoded,sha])?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO records_fts(rowid,body) VALUES(?1,?2)",
        params![id, encoded],
    )?;
    let result = get(&tx, id, true)?;
    tx.execute(
        "INSERT INTO record_history VALUES(?1,1,?2)",
        params![id, result.to_string()],
    )?;
    tx.commit()?;
    Ok(result)
}
pub fn get(c: &Connection, id: i64, full: bool) -> Result<Value> {
    let (payload, sha, status, revision, validation): (
        String,
        String,
        String,
        i64,
        Option<String>,
    ) = c.query_row(
        "SELECT payload,payload_sha256,status,revision,validation FROM records WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let p: Value = serde_json::from_str(&payload)?;
    let fresh = verify_artifacts(&p["evidence"], None).unwrap_or(false)
        && validation.as_ref().is_none_or(|v| {
            serde_json::from_str::<Value>(v)
                .ok()
                .is_some_and(|v| verify_artifacts(&v["artifacts"], None).unwrap_or(false))
        });
    let mut result = json!({"id":id,"payload_sha256":sha,"status":status,"revision":revision,"technology":p["technology"],"project":p["project"],"tested_versions":p["tested_versions"],"applicability":p["applicability"],"exclusions":p["exclusions"],"evidence_freshness":if fresh {"current"} else {"stale"},"trust":"untrusted_scoped_finding"});
    if full {
        result["payload"] = p;
        result["validation"] = validation
            .map(|v| serde_json::from_str::<Value>(&v).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
    } else {
        result["root_cause"] = json!(
            str_field(&p, "root_cause")?
                .chars()
                .take(500)
                .collect::<String>()
        );
    }
    Ok(result)
}
pub fn validate(
    c: &mut Connection,
    id: i64,
    revision: i64,
    evidence: &Path,
    allow: &[PathBuf],
) -> Result<Value> {
    let bytes = bounded_read(&checked_path(evidence, allow)?)?;
    if bytes.len() > 64_000 {
        bail!("validation exceeds 64k bytes");
    }
    let v: Value = serde_json::from_slice(&bytes)?;
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let current = get(&tx, id, true)?;
    if current["status"] != "proposal" || current["revision"] != revision {
        bail!("revision or status conflict");
    }
    if current["payload"]["root_cause_status"] != "proven"
        || current["evidence_freshness"] != "current"
    {
        bail!("proven root cause and current evidence required");
    }
    if v["record_id"] != id
        || v["record_revision"] != revision
        || v["record_sha256"] != current["payload_sha256"]
        || v["verdict"] != "passed"
    {
        bail!("validation does not bind this record/revision/hash with a passed verdict");
    }
    let checks = v["checks"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 64)
        .context("explicit checks required")?;
    for check in checks {
        str_field(check, "name")?;
        if check["passed"] != true {
            bail!("all checks must pass");
        }
    }
    if !verify_artifacts(&v["artifacts"], Some(allow))? {
        bail!("validation artifact mismatch");
    }
    tx.execute("UPDATE records SET status='validated',revision=revision+1,validation=?1 WHERE id=?2 AND revision=?3",params![v.to_string(),id,revision])?;
    let out = get(&tx, id, true)?;
    tx.execute(
        "INSERT INTO record_history VALUES(?1,?2,?3)",
        params![id, revision + 1, out.to_string()],
    )?;
    tx.commit()?;
    Ok(out)
}
#[derive(Default)]
pub struct Filters {
    pub technology: Option<String>,
    pub project: Option<String>,
    pub tested_version: Option<String>,
    pub status: Option<String>,
    /// At most this many chunks of one source file in the results (None: no limit)
    pub per_source: Option<usize>,
}
pub fn eligible(c: &Connection, f: &Filters) -> Result<std::collections::HashSet<i64>> {
    let version = f
        .tested_version
        .as_deref()
        .map(|s| {
            s.split_once('=')
                .context("tested-version requires component=exact-version")
        })
        .transpose()?;
    let mut s=c.prepare("SELECT r.id FROM records r WHERE (?1 IS NULL OR json_extract(r.payload,'$.technology')=?1) AND (?2 IS NULL OR json_extract(r.payload,'$.project')=?2) AND (?3 IS NULL OR r.status=?3) AND (?4 IS NULL OR EXISTS(SELECT 1 FROM json_each(r.payload,'$.tested_versions') WHERE key=?4 AND value=?5))")?;
    Ok(s.query_map(
        params![
            f.technology,
            f.project,
            f.status,
            version.map(|(k, _)| k),
            version.map(|(_, v)| v)
        ],
        |r| r.get(0),
    )?
    .collect::<rusqlite::Result<_>>()?)
}
/// Record ids and BM25 scores (higher is better) for the hybrid shortlist.
pub fn lexical_hits(
    c: &Connection,
    query: &str,
    limit: usize,
    f: &Filters,
) -> Result<Vec<(i64, f64)>> {
    if limit == 0 {
        return Ok(vec![]);
    }
    let version = f
        .tested_version
        .as_deref()
        .map(|s| {
            s.split_once('=')
                .context("tested-version requires component=exact-version")
        })
        .transpose()?;
    let mut s=c.prepare("SELECT r.id,-bm25(records_fts) FROM records r JOIN records_fts f ON f.rowid=r.id WHERE records_fts MATCH ?1 AND (?2 IS NULL OR json_extract(r.payload,'$.technology')=?2) AND (?3 IS NULL OR json_extract(r.payload,'$.project')=?3) AND (?4 IS NULL OR r.status=?4) AND (?5 IS NULL OR EXISTS(SELECT 1 FROM json_each(r.payload,'$.tested_versions') WHERE key=?5 AND value=?6)) ORDER BY bm25(records_fts),r.id LIMIT ?7")?;
    Ok(s.query_map(
        params![
            query,
            f.technology,
            f.project,
            f.status,
            version.map(|(k, _)| k),
            version.map(|(_, v)| v),
            limit as i64
        ],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?
    .collect::<rusqlite::Result<_>>()?)
}
pub fn search(c: &Connection, query: &str, limit: usize, f: &Filters) -> Result<Vec<Value>> {
    if limit == 0 {
        return Ok(vec![]);
    }
    let version = f
        .tested_version
        .as_deref()
        .map(|s| {
            s.split_once('=')
                .context("tested-version requires component=exact-version")
        })
        .transpose()?;
    let mut s=c.prepare("SELECT r.id FROM records r JOIN records_fts f ON f.rowid=r.id WHERE records_fts MATCH ?1 AND (?2 IS NULL OR json_extract(r.payload,'$.technology')=?2) AND (?3 IS NULL OR json_extract(r.payload,'$.project')=?3) AND (?4 IS NULL OR r.status=?4) AND (?5 IS NULL OR EXISTS(SELECT 1 FROM json_each(r.payload,'$.tested_versions') WHERE key=?5 AND value=?6)) ORDER BY bm25(records_fts),r.id LIMIT ?7")?;
    let ids: Vec<i64> = s
        .query_map(
            params![
                query,
                f.technology,
                f.project,
                f.status,
                version.map(|(k, _)| k),
                version.map(|(_, v)| v),
                limit as i64
            ],
            |r| r.get(0),
        )?
        .collect::<rusqlite::Result<_>>()?;
    ids.into_iter().map(|id| get(c, id, false)).collect()
}
