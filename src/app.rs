use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use walkdir::WalkDir;

pub const MAX_FILE: u64 = 16 * 1024 * 1024;
/// Scan bounds per `index` run: a mistake such as pointing at a whole disk aborts and rolls back instead of filling the database.
const DEFAULT_MAX_FILES: usize = 250_000;
const DEFAULT_MAX_MIB: u64 = 4096;
const SCHEMA: i64 = 4;
#[derive(Parser)]
#[command(
    version,
    about = "Local-only retrieval; retrieved text is untrusted data"
)]
struct Cli {
    /// Database path. Required for every command that touches a corpus; audit
    /// verification works on a standalone log file and does not need it.
    #[arg(long)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    Index {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, required = true)]
        allow: Vec<PathBuf>,
        /// Abort and roll back if the scan accepts more files than this
        #[arg(long, default_value_t = DEFAULT_MAX_FILES)]
        max_files: usize,
        /// Abort and roll back if the accepted source bytes exceed this many MiB
        #[arg(long, default_value_t = DEFAULT_MAX_MIB)]
        max_mib: u64,
    },
    RebuildEmbeddings {
        /// Local embedding model directory; repeat for an ensemble (1 to 3 models, blended equally)
        #[arg(long, required = true)]
        model: Vec<PathBuf>,
    },
    Search {
        query: String,
        #[arg(long, default_value = "lexical", value_parser = ["lexical", "hybrid"])]
        mode: String,
        /// Local embedding model directory, hybrid mode only; repeat for an ensemble
        #[arg(long)]
        model: Vec<PathBuf>,
        #[arg(long, default_value_t = 8)]
        limit: usize,
        #[arg(long)]
        technology: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        tested_version: Option<String>,
        #[arg(long)]
        status: Option<String>,
        /// At most this many chunks of one source file in the results
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=20))]
        per_source: Option<u8>,
    },
    Get {
        id: i64,
    },
    Propose {
        #[arg(long)]
        file: PathBuf,
        #[arg(long, required = true)]
        allow: Vec<PathBuf>,
    },
    Validate {
        id: i64,
        #[arg(long)]
        revision: i64,
        #[arg(long)]
        evidence: PathBuf,
        #[arg(long, required = true)]
        allow: Vec<PathBuf>,
    },
    GetRecord {
        id: i64,
    },
    Backup {
        #[arg(long)]
        output: PathBuf,
    },
    Stats,
    /// Serve the knowledge base to an agent; --mcp selects the MCP stdio transport.
    Serve {
        #[arg(long)]
        mcp: bool,
        /// Stay resident and serve every agent over a local socket instead of
        /// stdin/stdout. The `mcp` front-end spawns this automatically.
        #[arg(long)]
        daemon: bool,
        /// Append-only hash-chained log of every tool call. Refuses to serve if
        /// an existing log does not verify. Daemon mode defaults to a managed log.
        #[arg(long)]
        audit: Option<PathBuf>,
        /// Key for keyed HMAC integrity in the audit log (64 hex chars or >=32
        /// raw bytes). Requires --audit. Daemon mode manages this automatically.
        #[arg(long)]
        mac_key: Option<PathBuf>,
        /// Daemon mode: do not record an audit log.
        #[arg(long)]
        no_audit: bool,
    },
    /// Stdio front-end for agents: use the shared daemon when it runs, otherwise
    /// serve in-process. Audit logging, MAC key and token are automatic.
    Mcp,
    /// Register this corpus as an MCP server in every detected agent.
    Install {
        #[arg(long)]
        dry_run: bool,
    },
    /// Verify an append-only audit log (no database needed).
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
}
#[derive(Subcommand)]
pub enum AuditAction {
    /// Recompute the hash chain and report the first break, if any.
    Verify {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        mac_key: Option<PathBuf>,
        /// Pinned head hash. Detects tail truncation, which a bare hash chain
        /// cannot: without an out-of-band head, dropping the last line verifies.
        #[arg(long)]
        expect_last: Option<String>,
    },
}
pub fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
/// Build a child process with a scrubbed environment.
///
/// The MCP host spawns this binary inside the agent's own environment, which
/// typically carries provider credentials (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
/// GitHub tokens, ...). Helper subprocesses parse untrusted documents, so they
/// must never inherit those secrets. Only the variables a helper genuinely needs
/// are passed through; everything else is dropped.
pub(crate) fn helper_env(cmd: &mut std::process::Command) {
    cmd.env_clear();
    for key in [
        "PATH",
        "SYSTEMROOT",
        "WINDIR",
        "TMPDIR",
        "TMP",
        "TEMP",
        "HOME",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
    ] {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
}
pub(crate) fn open(path: &Path, write: bool) -> Result<Connection> {
    no_links(path)?;
    let mut c = Connection::open_with_flags(
        path,
        if write {
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        },
    )?;
    c.busy_timeout(Duration::from_secs(5))?;
    c.pragma_update(None, "foreign_keys", true)?;
    let version: i64 = c.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > SCHEMA {
        bail!("unsupported future schema");
    }
    if version == 0 && write {
        let tx = c.transaction()?;
        tx.execute_batch("CREATE TABLE sources(id INTEGER PRIMARY KEY,root TEXT NOT NULL,path TEXT NOT NULL UNIQUE,sha256 TEXT NOT NULL,revision INTEGER NOT NULL,status TEXT NOT NULL,extractor TEXT NOT NULL);
            CREATE TABLE chunks(id INTEGER PRIMARY KEY AUTOINCREMENT,source_id INTEGER NOT NULL REFERENCES sources(id),heading TEXT NOT NULL,page INTEGER,line INTEGER NOT NULL,text TEXT NOT NULL);
            CREATE VIRTUAL TABLE search_fts USING fts5(text,heading, tokenize='unicode61 remove_diacritics 2');
            PRAGMA user_version=1;")?;
        tx.commit()?;
    }
    if write {
        if version < 2 {
            let tx = c.transaction()?;
            tx.execute_batch(crate::records::MIGRATION)?;
            tx.pragma_update(None, "user_version", 2)?;
            tx.commit()?;
        }
        if version < 3 {
            let tx = c.transaction()?;
            tx.execute_batch("CREATE INDEX source_hash ON sources(sha256,status); CREATE INDEX chunk_source ON chunks(source_id);")?;
            tx.pragma_update(None, "user_version", 3)?;
            tx.commit()?;
        }
        if version < 4 {
            let tx = c.transaction()?;
            tx.execute_batch(crate::semantic::MIGRATION)?;
            tx.pragma_update(None, "user_version", 4)?;
            tx.commit()?;
        }
    } else if version != SCHEMA {
        bail!("schema migration required; run a write command");
    }
    Ok(c)
}
fn is_link(p: &Path) -> bool {
    let Ok(m) = fs::symlink_metadata(p) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
/// Private scratch directory for helper input/output: `TMPDIR`, else a per-user
/// folder (`%LOCALAPPDATA%` on Windows, `~/.cache` elsewhere). Created on demand,
/// never behind a link.
pub(crate) fn scratch_dir() -> Result<std::path::PathBuf> {
    let scratch = std::env::var_os("TMPDIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(|p| std::path::PathBuf::from(p).join("Footnote/scratch"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|p| std::path::PathBuf::from(p).join(".cache/Footnote/scratch"))
        })
        .ok_or_else(|| anyhow::anyhow!("configure TMPDIR for helper scratch space"))?;
    no_links(&scratch)?;
    std::fs::create_dir_all(&scratch)?;
    Ok(scratch)
}

pub fn no_links(p: &Path) -> Result<()> {
    let abs = std::path::absolute(p)?;
    if abs.ancestors().any(is_link) {
        bail!("symlinks/reparse points are not allowed");
    }
    Ok(())
}
fn excluded(p: &Path) -> bool {
    p.components().any(|c| {
        let name = c.as_os_str().to_string_lossy().to_lowercase();
        name.starts_with('.')
            || [
                "node_modules",
                "target",
                "private",
                "secrets",
                "credentials",
                "optimization-registry",
            ]
            .contains(&name.as_str())
            || name.contains("secret")
            || name.contains("credential")
    })
}
pub fn bounded_read(p: &Path) -> Result<Vec<u8>> {
    no_links(p)?;
    if fs::metadata(p)?.len() > MAX_FILE {
        bail!("source exceeds 16 MiB");
    }
    if !fs::metadata(p)?.is_file() {
        bail!("source must be a regular file");
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(p)?
        .take(MAX_FILE + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE {
        bail!("source grew beyond bound");
    }
    Ok(bytes)
}
fn index(
    c: &mut Connection,
    root: &Path,
    allow: &[PathBuf],
    max_files: usize,
    max_mib: u64,
) -> Result<Value> {
    no_links(root)?;
    let root = fs::canonicalize(root)?;
    if !root.is_dir() {
        bail!("root must be a directory");
    }
    let mut allowed = false;
    for p in allow {
        no_links(p)?;
        if root.starts_with(fs::canonicalize(p)?) {
            allowed = true;
        }
    }
    if !allowed {
        bail!("root is outside explicit allowlist");
    }
    let root_s = root.to_string_lossy().to_string();
    let mut seen = std::collections::HashSet::new();
    let mut indexed = 0;
    let mut unchanged = 0;
    let mut failed = 0;
    let mut failures = Vec::new();
    let mut total_bytes = 0_u64;
    let extractor_hash = hash(&fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/extract.py"),
    )?);
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.path() == root
                || (!excluded(e.path().strip_prefix(&root).unwrap_or(e.path()))
                    && !is_link(e.path()))
        })
    {
        let e = entry?;
        if !e.file_type().is_file() {
            continue;
        }
        let ext = e
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ![
            "md", "txt", "pdf", "png", "jpg", "jpeg", "webp", "tiff", "bmp",
        ]
        .contains(&ext.as_str())
        {
            continue;
        }
        let path = e.path().to_string_lossy().to_string();
        seen.insert(path.clone());
        if seen.len() > max_files {
            bail!("index file limit exceeded ({max_files}); raise it with --max-files");
        }
        let bytes = bounded_read(e.path())?;
        total_bytes += bytes.len() as u64;
        if total_bytes > max_mib * 1024 * 1024 {
            bail!("index scan byte limit exceeded ({max_mib} MiB); raise it with --max-mib");
        }
        let sha = hash(&bytes);
        let old: Option<(i64, String, String)> = tx
            .query_row(
                "SELECT id,sha256,extractor FROM sources WHERE path=?1 AND status='indexed'",
                [&path],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if old.as_ref().is_some_and(|(_, h, ex)| {
            h == &sha
                && (if ["md", "txt"].contains(&ext.as_str()) {
                    ex == "utf8-v2"
                } else {
                    ex.contains(&extractor_hash) && ex.contains("chunker-v2")
                })
        }) {
            unchanged += 1;
            continue;
        }
        let extraction = if ["md", "txt"].contains(&ext.as_str()) {
            match String::from_utf8(bytes.clone()) {
                Ok(text) if text.len() <= 1_000_000 => crate::extract::Extraction {
                    status: if text.trim().is_empty() {
                        "empty"
                    } else {
                        "indexed"
                    }
                    .into(),
                    extractor: "utf8-v2".into(),
                    pages: vec![crate::extract::Page { page: 0, text }],
                },
                _ => crate::extract::Extraction {
                    status: "failed".into(),
                    extractor: "utf8-v2".into(),
                    pages: vec![],
                },
            }
        } else {
            crate::extract::extract(&bytes, &ext).unwrap_or_else(|_| crate::extract::Extraction {
                status: "unsupported_or_failed".into(),
                extractor: "local-extractor-unavailable".into(),
                pages: vec![],
            })
        };
        if extraction.status != "indexed" {
            failed += 1;
            if failures.len() < 20 {
                failures.push(json!({"path":path,"status":extraction.status}));
            }
        }
        tx.execute("INSERT INTO sources(root,path,sha256,revision,status,extractor) VALUES(?1,?2,?3,1,?4,?5) ON CONFLICT(path) DO UPDATE SET root=excluded.root,sha256=excluded.sha256,revision=sources.revision+1,status=excluded.status,extractor=excluded.extractor",params![root_s,path,sha,extraction.status,extraction.extractor])?;
        let id: i64 = tx.query_row("SELECT id FROM sources WHERE path=?1", [&path], |r| {
            r.get(0)
        })?;
        tx.execute(
            "DELETE FROM search_fts WHERE rowid IN (SELECT id FROM chunks WHERE source_id=?1)",
            [id],
        )?;
        tx.execute("DELETE FROM chunks WHERE source_id=?1", [id])?;
        if extraction.status == "indexed" {
            for p in extraction.pages {
                text_chunks(
                    &tx,
                    id,
                    &p.text,
                    if p.page == 0 { None } else { Some(p.page) },
                )?;
            }
            indexed += 1;
        }
    }
    let paths: Vec<(i64, String)> = {
        let mut s =
            tx.prepare("SELECT id,path FROM sources WHERE root=?1 AND status!='deleted'")?;
        s.query_map([&root_s], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?
    };
    let mut deleted = 0;
    for (id, p) in paths {
        if !seen.contains(&p) {
            tx.execute(
                "UPDATE sources SET status='deleted',revision=revision+1 WHERE id=?1",
                [id],
            )?;
            tx.execute(
                "DELETE FROM search_fts WHERE rowid IN (SELECT id FROM chunks WHERE source_id=?1)",
                [id],
            )?;
            deleted += 1;
        }
    }
    tx.commit()?;
    Ok(
        json!({"indexed":indexed,"unchanged":unchanged,"deleted":deleted,"failed":failed,"failures":failures}),
    )
}
fn text_chunks(c: &Connection, id: i64, text: &str, page: Option<i64>) -> Result<()> {
    let title = text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("")
        .chars()
        .take(500)
        .collect::<String>();
    let mut heading = String::new();
    let mut chunk = String::new();
    let mut line = 1;
    for (n, s) in text.lines().enumerate() {
        if s.starts_with('#') || chunk.chars().count() + s.chars().count() > 2000 {
            add_chunk(c, id, &heading, page, line, &chunk, &title)?;
            chunk.clear();
            if s.starts_with('#') {
                heading = s.trim_start_matches('#').trim().chars().take(500).collect();
            }
        }
        if chunk.is_empty() {
            line = n as i64 + 1;
        }
        for ch in s.chars().chain(std::iter::once('\n')) {
            if chunk.chars().count() == 2000 {
                add_chunk(c, id, &heading, page, line, &chunk, &title)?;
                chunk.clear();
                line = n as i64 + 1;
            }
            chunk.push(ch);
        }
    }
    add_chunk(c, id, &heading, page, line, &chunk, &title)
}
fn add_chunk(
    c: &Connection,
    id: i64,
    heading: &str,
    page: Option<i64>,
    line: i64,
    text: &str,
    title: &str,
) -> Result<()> {
    if text.trim().is_empty() {
        return Ok(());
    }
    c.execute(
        "INSERT INTO chunks(source_id,heading,page,line,text) VALUES(?1,?2,?3,?4,?5)",
        params![id, heading, page, line, text],
    )?;
    c.execute(
        "INSERT INTO search_fts(rowid,text,heading) VALUES(?1,?2,?3)",
        params![c.last_insert_rowid(), text, format!("{title} {heading}")],
    )?;
    Ok(())
}
fn freshness(path: &str, sha: &str, status: &str) -> &'static str {
    if status == "deleted" {
        return "deleted";
    }
    match bounded_read(Path::new(path)) {
        Ok(b) if hash(&b) == sha => "current",
        Ok(_) => "stale",
        Err(_) => "missing_or_unreadable",
    }
}
pub(crate) fn hit(r: &rusqlite::Row<'_>, full: bool) -> rusqlite::Result<Value> {
    let path: String = r.get(1)?;
    let sha: String = r.get(2)?;
    let status: String = r.get(4)?;
    let text: String = r.get(9)?;
    Ok(
        json!({"id":r.get::<_,i64>(0)?,"path":path,"sha256":sha,"revision":r.get::<_,i64>(3)?,"status":status,"extractor":r.get::<_,String>(5)?,"heading":r.get::<_,String>(6)?,"page":r.get::<_,Option<i64>>(7)?,"line":r.get::<_,i64>(8)?,"text":text.chars().take(if full {2000} else {500}).collect::<String>(),"freshness":freshness(&path,&sha,&status),"trust":"untrusted_source_data"}),
    )
}
pub(crate) const HIT: &str = "SELECT c.id,s.path,s.sha256,s.revision,s.status,s.extractor,c.heading,c.page,c.line,c.text FROM chunks c JOIN sources s ON s.id=c.source_id";
/// Counts the chunks already returned from each source file, so one long file cannot fill the whole result list.
pub(crate) struct PerSource {
    cap: Option<usize>,
    seen: std::collections::HashMap<String, usize>,
}
impl PerSource {
    pub(crate) fn new(cap: Option<usize>) -> Self {
        Self {
            cap,
            seen: Default::default(),
        }
    }
    /// True when another chunk of this source may be returned (and records it).
    pub(crate) fn allow(&mut self, path: &str) -> bool {
        let Some(cap) = self.cap else {
            return true;
        };
        let n = self.seen.entry(path.to_string()).or_default();
        if *n >= cap {
            return false;
        }
        *n += 1;
        true
    }
}
/// Chunk ids with BM25 scores (higher is better) for any-word matching: Tantivy when its derived index is available, else FTS5.
pub(crate) fn lexical_chunks(c: &Connection, q: &str, n: usize) -> Result<Vec<(i64, f64)>> {
    if let Some(hits) = crate::lexical::top_hits(c, q, n).ok().flatten() {
        return Ok(hits.into_iter().map(|(id, s)| (id, s as f64)).collect());
    }
    let mut s = c.prepare("SELECT rowid,-bm25(search_fts) FROM search_fts WHERE search_fts MATCH ?1 ORDER BY bm25(search_fts),rowid LIMIT ?2")?;
    Ok(s.query_map(params![match_expr(q), n as i64], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?
    .collect::<rusqlite::Result<_>>()?)
}
/// FTS5 expression matching any of the query's words, each quoted so punctuation and operators in user text stay literal.
pub(crate) fn match_expr(q: &str) -> String {
    q.split_whitespace()
        .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}
pub(crate) fn search(
    c: &Connection,
    q: &str,
    limit: usize,
    filters: &crate::records::Filters,
) -> Result<Value> {
    if q.len() > 512 || q.trim().is_empty() || !(1..=20).contains(&limit) {
        bail!("query must be 1..512 bytes and limit 1..20");
    }
    // Any query word may match and BM25 ranks the chunks. Requiring every word returned nothing for natural-language questions
    // (bench v1: SciFact nDCG@10 0.017 with AND, 0.663 with OR).
    let expr = match_expr(q);
    let records = crate::records::search(c, &expr, limit, filters)?;
    if filters.technology.is_some()
        || filters.project.is_some()
        || filters.tested_version.is_some()
        || filters.status.is_some()
    {
        return Ok(json!({"mode":"lexical_fts5","results":[],"records":records}));
    }
    let text_limit = limit - records.len();
    // Chunk ranking comes from the derived Tantivy index when there is one (BM25, stemming); FTS5 is the fallback.
    if let Some(ids) = crate::lexical::top_ids(c, q, (limit * 30).max(100))
        .ok()
        .flatten()
    {
        let mut s = c.prepare(&format!(
            "{HIT} WHERE c.id=?1 AND s.status='indexed' AND c.id=(SELECT min(c2.id) FROM chunks c2 JOIN sources s2 ON s2.id=c2.source_id WHERE s2.sha256=s.sha256 AND s2.status='indexed' AND c2.heading=c.heading AND c2.page IS c.page AND c2.line=c.line AND c2.text=c.text)"
        ))?;
        let mut results = Vec::new();
        let mut cap = PerSource::new(filters.per_source);
        for id in ids {
            if results.len() >= text_limit {
                break;
            }
            if let Some(v) = s.query_row([id], |r| hit(r, false)).optional()?
                && cap.allow(v["path"].as_str().unwrap_or(""))
            {
                results.push(v);
            }
        }
        return Ok(json!({"mode":"lexical_bm25","results":results,"records":records}));
    }
    let sql = format!(
        "{HIT} JOIN search_fts f ON f.rowid=c.id WHERE search_fts MATCH ?1 AND s.status='indexed' AND c.id=(SELECT min(c2.id) FROM chunks c2 JOIN sources s2 ON s2.id=c2.source_id WHERE s2.sha256=s.sha256 AND s2.status='indexed' AND c2.heading=c.heading AND c2.page IS c.page AND c2.line=c.line AND c2.text=c.text) ORDER BY bm25(search_fts),c.id LIMIT ?2"
    );
    let mut s = c.prepare(&sql)?;
    let fetch = if filters.per_source.is_some() {
        text_limit * 10
    } else {
        text_limit
    };
    let mut cap = PerSource::new(filters.per_source);
    let results: Vec<Value> = s
        .query_map(params![expr, fetch as i64], |r| hit(r, false))?
        .collect::<rusqlite::Result<Vec<Value>>>()?
        .into_iter()
        .filter(|v| cap.allow(v["path"].as_str().unwrap_or("")))
        .take(text_limit)
        .collect();
    Ok(json!({"mode":"lexical_fts5","results":results,"records":records}))
}
fn require_db(db: &Option<PathBuf>) -> Result<&Path> {
    db.as_deref()
        .ok_or_else(|| anyhow::anyhow!("--db is required for this command"))
}

/// Daemon security defaults: an audit log and a managed MAC key, created on
/// demand, unless the operator opts out. No flag or key file has to be supplied.
fn managed_audit(
    disabled: bool,
    audit: &Option<PathBuf>,
    mac_key: &Option<PathBuf>,
) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    if disabled {
        return Ok((None, None));
    }
    match audit {
        Some(path) => Ok((Some(path.clone()), mac_key.clone())),
        None => {
            let key = crate::state::load_or_create_key()?;
            Ok((Some(crate::state::audit_path()?), Some(key)))
        }
    }
}
fn audit_command(action: &AuditAction) -> Result<Value> {
    match action {
        AuditAction::Verify {
            file,
            mac_key,
            expect_last,
        } => {
            let key = match mac_key.as_deref() {
                Some(path) => Some(crate::audit::load_key(path)?),
                // Default: this machine's managed key, so MACs are checked without a flag.
                None => match crate::state::key_path() {
                    Ok(path) if path.is_file() => Some(crate::audit::load_key(&path)?),
                    _ => None,
                },
            };
            let (entries, last_hash) = crate::audit::verify(file, key.as_deref())?;
            let head = match expect_last.as_deref() {
                Some(expected) if expected != last_hash => {
                    bail!(
                        "audit head mismatch: expected {expected}, found {last_hash} \
                         (log was truncated or extended)"
                    );
                }
                Some(_) => "matched",
                None => "not_checked",
            };
            Ok(json!({
                "entries": entries,
                "ok": true,
                "last_hash": last_hash,
                "mac": if key.is_some() { "verified" } else { "not_checked" },
                "head": head,
            }))
        }
    }
}
fn execute(cli: Cli) -> Result<Value> {
    // MCP stdio opens the database lazily, read-only, per request, so a missing
    // or not-yet-migrated database is reported as a tool error instead of
    // aborting the server before the client can call tools/list. Audit-log
    // verification is a pure file operation and also needs no database.
    match &cli.command {
        Action::Serve {
            mcp,
            daemon,
            audit,
            mac_key,
            no_audit,
        } => {
            let db = require_db(&cli.db)?;
            if *daemon {
                let (audit_path, key_path) = managed_audit(*no_audit, audit, mac_key)?;
                let shared = crate::mcp::Shared::new(
                    db,
                    crate::mcp::open_audit(audit_path.as_deref(), key_path.as_deref())?,
                    Some(crate::state::load_or_create_token()?),
                );
                crate::daemon::run(shared)?;
                return Ok(Value::Null);
            }
            return crate::mcp::serve(db, *mcp, audit.as_deref(), mac_key.as_deref());
        }
        Action::Mcp => {
            let db = require_db(&cli.db)?;
            let (audit_path, key_path) = managed_audit(false, &None, &None)?;
            let shared = crate::mcp::Shared::new(
                db,
                crate::mcp::open_audit(audit_path.as_deref(), key_path.as_deref())?,
                None,
            );
            crate::daemon::proxy(&shared)?;
            return Ok(Value::Null);
        }
        Action::Install { dry_run } => return crate::install::run(&cli.db, *dry_run),
        Action::Audit { action } => return audit_command(action),
        _ => {}
    }
    let db = require_db(&cli.db)?;
    let mut c = open(
        db,
        matches!(
            cli.command,
            Action::Index { .. }
                | Action::Propose { .. }
                | Action::Validate { .. }
                | Action::RebuildEmbeddings { .. }
        ),
    )?;
    match cli.command {
        Action::Index {
            root,
            allow,
            max_files,
            max_mib,
        } => index(&mut c, &root, &allow, max_files, max_mib),
        Action::RebuildEmbeddings { model } => crate::semantic::rebuild(&mut c, &model),
        Action::Search {
            query,
            mode,
            model,
            limit,
            technology,
            project,
            tested_version,
            status,
            per_source,
        } => {
            let filters = crate::records::Filters {
                technology,
                project,
                tested_version,
                status,
                per_source: per_source.map(usize::from),
            };
            if mode == "hybrid" {
                if model.is_empty() {
                    bail!("hybrid requires --model local directory");
                }
                crate::semantic::search(&c, &query, limit, &filters, &model)
            } else {
                if !model.is_empty() {
                    bail!("--model requires --mode hybrid");
                }
                search(&c, &query, limit, &filters)
            }
        }
        Action::Propose { file, allow } => crate::records::propose(&mut c, &file, &allow),
        Action::Validate {
            id,
            revision,
            evidence,
            allow,
        } => crate::records::validate(&mut c, id, revision, &evidence, &allow),
        Action::GetRecord { id } => crate::records::get(&c, id, true),
        Action::Get { id } => {
            Ok(c.query_row(&format!("{HIT} WHERE c.id=?1"), [id], |r| hit(r, true))?)
        }
        Action::Backup { output } => backup(&c, &output),
        Action::Stats => stats(&c),
        // Dispatched above, before the database is opened; MCP serves its
        // read-only tools by opening the database lazily per request.
        Action::Serve { .. } => unreachable!("serve is dispatched before the database is opened"),
        Action::Mcp => unreachable!("mcp is dispatched before the database is opened"),
        Action::Audit { .. } => {
            unreachable!("audit verification is dispatched before the database is opened")
        }
        Action::Install { .. } => {
            unreachable!("install is dispatched before the database is opened")
        }
    }
}
pub(crate) fn stats(c: &Connection) -> Result<Value> {
    let mut statuses = serde_json::Map::new();
    let mut s = c.prepare("SELECT status,count(*) FROM sources GROUP BY status")?;
    for row in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (status, n) = row?;
        statuses.insert(status, json!(n));
    }
    Ok(
        json!({"schema":SCHEMA,"sqlite_version":c.query_row("SELECT sqlite_version()",[],|r|r.get::<_,String>(0))?,"fts5":c.query_row("SELECT sqlite_compileoption_used('ENABLE_FTS5')",[],|r|r.get::<_,bool>(0))?,"sources":c.query_row("SELECT count(*) FROM sources",[],|r|r.get::<_,i64>(0))?,"source_statuses":statuses,"chunks":c.query_row("SELECT count(*) FROM chunks",[],|r|r.get::<_,i64>(0))?,"records":c.query_row("SELECT count(*) FROM records",[],|r|r.get::<_,i64>(0))?,"semantic":"local_onnx_opt_in","embedding_models":c.query_row("SELECT count(*) FROM embedding_models",[],|r|r.get::<_,i64>(0))?,"embeddings":c.query_row("SELECT count(*) FROM embeddings",[],|r|r.get::<_,i64>(0))?}),
    )
}
fn backup(c: &Connection, output: &Path) -> Result<Value> {
    no_links(output)?;
    let f = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    drop(f);
    // SQLite online backup yields a coherent authoritative + derived snapshot.
    if let Err(e) = c.backup(rusqlite::MAIN_DB, output, None) {
        let _ = fs::remove_file(output);
        return Err(e.into());
    }
    use std::io::Read;
    let mut file = fs::File::open(output)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(json!({"backup":output,"sha256":format!("{:x}",digest.finalize())}))
}
pub fn run() {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e)
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            print!("{e}");
            return;
        }
        Err(e) => {
            println!("{}", json!({"error":e.to_string()}));
            std::process::exit(2);
        }
    };
    // In MCP mode stdout belongs to the JSON-RPC transport: a serve-level
    // failure is reported on stderr instead of the CLI's JSON envelope.
    let serving = matches!(cli.command, Action::Serve { .. } | Action::Mcp);
    match execute(cli) {
        Ok(v) => {
            if serving {
                return;
            }
            let encoded = v.to_string();
            if encoded.len() > 256_000 {
                println!(
                    "{}",
                    json!({"error":"response exceeds 256k byte limit; narrow search"})
                );
                std::process::exit(1);
            }
            println!("{encoded}");
        }
        Err(e) => {
            if serving {
                eprintln!("{}", json!({"error":e.to_string()}));
                std::process::exit(1);
            }
            println!("{}", json!({"error":e.to_string()}));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod helper_env_tests {
    /// The scrubbed helper environment must drop everything the agent host
    /// injected (credentials, Cargo/CI variables) while keeping the handful of
    /// system variables a Python/ONNX helper needs to start.
    #[test]
    fn helper_env_clears_inherited_secrets_and_keeps_system_variables() {
        let mut cmd = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(["/c", "set"]);
            c
        } else {
            std::process::Command::new("env")
        };
        crate::app::helper_env(&mut cmd);
        let out = cmd.output().expect("probe command must run");
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        // Set in this test process by Cargo, deliberately NOT in the allowlist.
        assert!(
            !text.contains("cargo_manifest_dir"),
            "helper inherited the agent/cargo environment"
        );
        assert!(
            !text.contains("cargo_pkg_"),
            "helper inherited the agent/cargo environment"
        );
        assert!(text.contains("path"), "helper lost PATH");
    }
}
