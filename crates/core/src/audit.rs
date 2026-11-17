//! Append-only, hash-chained audit log for MCP tool calls.
//!
//! Every tool call appends one JSON line. Each line names the hash of the
//! previous line, so removing, reordering or editing a line breaks the chain at
//! a detectable point. With a MAC key the chain also binds a keyed HMAC over
//! `seq | result_sha256 | prev`, which a party without the key cannot recompute.
//!
//! What this does and does not buy:
//! * It detects tampering by anything that cannot read the key file: accidental
//!   edits, truncation, reordering, a proxy rewriting evidence mid-flight.
//! * It does NOT defend against someone who can read the key and rewrite the
//!   whole log. Keep the key outside the agent's reach (user-only ACL, 0600).
//! * Argument text is never stored, only its hash: queries can be sensitive.
use crate::hash;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Previous-hash value for the first entry in a log.
pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// A per-server-run label, not a secret: it groups entries from one session.
pub fn session_id() -> String {
    let seed = format!("{}|{}", std::process::id(), now_ms());
    hash(seed.as_bytes())[..16].to_string()
}

/// Load a MAC key. Accepts 64 hex characters or raw bytes of at least 32 bytes.
pub fn load_key(path: &Path) -> Result<Vec<u8>> {
    let raw = std::fs::read(path).with_context(|| format!("read MAC key {}", path.display()))?;
    let trimmed: Vec<u8> = raw
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let key = if trimmed.len() == 64 && trimmed.iter().all(u8::is_ascii_hexdigit) {
        let mut out = Vec::with_capacity(32);
        for pair in trimmed.chunks(2) {
            let s = std::str::from_utf8(pair).context("MAC key hex")?;
            out.push(u8::from_str_radix(s, 16).context("MAC key hex")?);
        }
        out
    } else {
        trimmed
    };
    if key.len() < 32 {
        bail!(
            "MAC key must be at least 32 bytes (got {}); use 64 hex chars or raw bytes",
            key.len()
        );
    }
    Ok(key)
}

/// HMAC-SHA256 (RFC 2104), so keyed integrity needs no extra dependency.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut k = if key.len() > BLOCK {
        Sha256::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    k.resize(BLOCK, 0);
    let mut inner = vec![0x36_u8; BLOCK];
    let mut outer = vec![0x5c_u8; BLOCK];
    for (i, byte) in k.iter().enumerate() {
        inner[i] ^= byte;
        outer[i] ^= byte;
    }
    let mut first = Sha256::new();
    first.update(&inner);
    first.update(data);
    let mut second = Sha256::new();
    second.update(&outer);
    second.update(first.finalize());
    format!("{:x}", second.finalize())
}

/// Canonical encoding of a JSON value. `serde_json` maps are key-sorted here, so
/// this is stable across runs and platforms.
fn canonical(value: &Value) -> String {
    value.to_string()
}

/// What the caller can quote back to a client to bind evidence to a log line.
pub struct Appended {
    pub seq: u64,
    pub result_sha256: String,
    pub mac: Option<String>,
}

impl Appended {
    pub fn to_meta(&self, session: &str) -> Value {
        json!({
            "session": session,
            "seq": self.seq,
            "result_sha256": self.result_sha256,
            "mac": self.mac,
        })
    }
}

pub struct Audit {
    path: PathBuf,
    key: Option<Vec<u8>>,
    seq: u64,
    prev: String,
}

impl Audit {
    /// Open (or create) the log, verifying any existing chain first. A broken
    /// chain refuses to serve: an unaccountable session must not start.
    pub fn open(path: &Path, key: Option<Vec<u8>>) -> Result<Self> {
        let (seq, prev) = if path.exists() {
            verify(path, key.as_deref()).with_context(|| {
                format!(
                    "audit log {} does not verify; refusing to serve",
                    path.display()
                )
            })?
        } else {
            (0, GENESIS.to_string())
        };
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create audit directory {}", parent.display()))?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("open audit log {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            key,
            seq,
            prev,
        })
    }

    pub fn append(
        &mut self,
        session: &str,
        tool: &str,
        args: &Value,
        ok: bool,
        result: &Value,
        elapsed_ms: u64,
    ) -> Result<Appended> {
        // Several processes share one log (an MCP server per agent session). Under an exclusive lock, continue from the line that is
        // really last, not from the one this process saw: otherwise two sessions both write the next number and the chain forks.
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&self.path)
            .with_context(|| format!("append audit log {}", self.path.display()))?;
        file.lock()
            .with_context(|| format!("lock audit log {}", self.path.display()))?;
        if let Some((seq, prev)) = last_entry(&mut file)? {
            (self.seq, self.prev) = (seq, prev);
        }
        let seq = self.seq + 1;
        let result_sha256 = hash(canonical(result).as_bytes());
        let mut entry = json!({
            "seq": seq,
            "ts_ms": now_ms(),
            "session": session,
            "tool": tool,
            "args_sha256": hash(canonical(args).as_bytes()),
            "ok": ok,
            "result_sha256": result_sha256,
            "elapsed_ms": elapsed_ms,
            "prev": self.prev,
        });
        let mac = self
            .key
            .as_deref()
            .map(|k| hmac_sha256(k, format!("{seq}|{result_sha256}|{}", self.prev).as_bytes()));
        if let Some(m) = &mac {
            entry["mac"] = json!(m);
        }
        // Hash covers the previous link plus the entry body (mac included).
        let entry_hash = hash(format!("{}|{}", self.prev, canonical(&entry)).as_bytes());
        entry["hash"] = json!(entry_hash);
        let line = canonical(&entry);
        file.write_all(format!("{line}\n").as_bytes())?;
        file.flush()?; // the lock is released when `file` is dropped

        self.seq = seq;
        self.prev = entry_hash;
        Ok(Appended {
            seq,
            result_sha256,
            mac,
        })
    }
}

/// `(seq, hash)` of the last line, or `None` for an empty log. Lines are a few hundred bytes, so the last 64 KiB hold it.
fn last_entry(file: &mut File) -> Result<Option<(u64, String)>> {
    let len = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(len.saturating_sub(65_536)))?;
    let mut tail = String::new();
    file.read_to_string(&mut tail)?;
    let Some(line) = tail.lines().rev().find(|l| !l.trim().is_empty()) else {
        return Ok(None);
    };
    let entry: Value = serde_json::from_str(line).context("last audit line: not JSON")?;
    match (entry["seq"].as_u64(), entry["hash"].as_str()) {
        (Some(seq), Some(h)) => Ok(Some((seq, h.to_string()))),
        _ => bail!("last audit line: no seq or hash"),
    }
}

/// Recompute the chain. Returns `(entry_count, last_hash)` or the first break.
pub fn verify(path: &Path, key: Option<&[u8]>) -> Result<(u64, String)> {
    let file = File::open(path).with_context(|| format!("open audit log {}", path.display()))?;
    // Appends hold an exclusive lock; on Windows that lock is mandatory, so reading during another session's append failed and a
    // server starting at that moment refused to serve (#167). A shared lock waits for the append to finish, then reads whole lines.
    file.lock_shared()
        .with_context(|| format!("lock audit log {}", path.display()))?;
    let mut prev = GENESIS.to_string();
    let mut count = 0_u64;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let number = index + 1;
        let line = line.with_context(|| format!("read audit line {number}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: Value = serde_json::from_str(&line)
            .with_context(|| format!("audit line {number}: not JSON"))?;
        let seq = entry["seq"].as_u64().unwrap_or(0);
        if seq != count + 1 {
            bail!(
                "audit line {number}: sequence gap (expected seq {}, found {seq})",
                count + 1
            );
        }
        if entry["prev"].as_str() != Some(prev.as_str()) {
            bail!("audit line {number}: chain break (prev does not match previous hash)");
        }
        let mut body = entry.clone();
        let declared = body
            .as_object_mut()
            .and_then(|o| o.remove("hash"))
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        let expected = hash(format!("{}|{}", prev, canonical(&body)).as_bytes());
        if expected != declared {
            bail!("audit line {number}: hash mismatch (line was modified)");
        }
        if let Some(k) = key {
            let declared_mac = entry["mac"].as_str().unwrap_or("");
            let result_sha = entry["result_sha256"].as_str().unwrap_or("");
            let expected_mac = hmac_sha256(k, format!("{seq}|{result_sha}|{prev}").as_bytes());
            if declared_mac != expected_mac {
                bail!("audit line {number}: MAC mismatch (wrong key or tampered entry)");
            }
        }
        prev = declared;
        count += 1;
    }
    Ok((count, prev))
}
