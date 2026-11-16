//! The one database schema (corpus and agent memory share it), its migrations, chunk storage and keyword search.
use anyhow::{Result, bail};
use footnote_core::{bounded_read, hash, no_links};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Duration};

pub const SCHEMA: i64 = 5;
pub const MEMORY_MIGRATION: &str = "CREATE TABLE memories(source_id INTEGER PRIMARY KEY REFERENCES sources(id),kind TEXT NOT NULL CHECK(kind IN ('preference','fact','fix','session')),scope TEXT NOT NULL,observed_at TEXT NOT NULL,session TEXT,supersedes INTEGER REFERENCES sources(id));
CREATE INDEX memory_scope ON memories(scope,kind);";
/// Source paths of agent memories start with this marker (no file behind them).
pub const MEMORY_PREFIX: &str = "memory:";

pub fn is_memory_db(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sources WHERE path LIKE 'memory:%')",
        [],
        |r| r.get(0),
    )?)
}

pub fn open(path: &Path, write: bool) -> Result<Connection> {
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
        if version < 5 {
            let tx = c.transaction()?;
            tx.execute_batch(MEMORY_MIGRATION)?;
            tx.pragma_update(None, "user_version", 5)?;
            tx.commit()?;
        }
    } else if version != SCHEMA {
        bail!("schema migration required; run a write command");
    }
    Ok(c)
}
/// Markdown heading level and text (`## Title` -> 2, "Title"); `#hashtag` and `#!/bin/sh` are not headings.
fn heading_of(s: &str) -> Option<(usize, &str)> {
    let level = s.chars().take_while(|&c| c == '#').count();
    let rest = &s[level..];
    ((1..=6).contains(&level) && (rest.is_empty() || rest.starts_with(' ')))
        .then(|| (level, rest.trim()))
}

/// Chunks follow the page structure: a new chunk at each heading (outside fenced code) once the current one has 500
/// characters, otherwise the short section joins the next one in the same parent; a chunk ends past 2,000 characters but
/// never inside a code block (hard cap 3,000). Each chunk's heading is its path in the page, `Page title > Section >
/// Subsection`, so a short section ("Default values") still says which page and part it belongs to.
pub fn text_chunks(c: &Connection, id: i64, text: &str, page: Option<i64>) -> Result<()> {
    const FLOOR: usize = 500;
    const TARGET: usize = 2000;
    const MAX: usize = 3000;
    let title = text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("")
        .chars()
        .take(500)
        .collect::<String>();
    let mut path: Vec<String> = Vec::new();
    let mut heading = String::new();
    let mut chunk = String::new();
    let (mut len, mut level0) = (0usize, 0usize); // chunk length in characters, level of the heading it started at
    let mut line = 1;
    let mut fence = false;
    for (n, s) in text.lines().enumerate() {
        let t = s.trim_start();
        let fence_line = t.starts_with("```") || t.starts_with("~~~");
        let head = if fence { None } else { heading_of(s) };
        let flush = match head {
            Some((level, _)) => len >= FLOOR || level < level0,
            None => !fence && len + s.chars().count() > TARGET,
        };
        if flush {
            add_chunk(c, id, &heading, page, line, &chunk, &title)?;
            chunk.clear();
            len = 0;
        }
        if let Some((level, h)) = head {
            path.truncate(level - 1);
            path.resize(level - 1, String::new());
            path.push(h.to_string());
            if len == 0 || level0 == 0 {
                let parts: Vec<&str> = path
                    .iter()
                    .map(String::as_str)
                    .filter(|p| !p.is_empty())
                    .collect();
                let mut joined = parts.join(" > ");
                if !title.is_empty() && level > 1 && !joined.starts_with(title.as_str()) {
                    joined = format!("{title} > {joined}");
                }
                heading = joined.chars().take(500).collect();
                level0 = level;
            }
        }
        if len == 0 {
            line = n as i64 + 1;
        }
        for ch in s.chars().chain(std::iter::once('\n')) {
            if len >= if fence { MAX } else { TARGET } {
                add_chunk(c, id, &heading, page, line, &chunk, &title)?;
                chunk.clear();
                len = 0;
                line = n as i64 + 1;
            }
            chunk.push(ch);
            len += 1;
        }
        if fence_line {
            fence = !fence;
        }
    }
    add_chunk(c, id, &heading, page, line, &chunk, &title)
}
pub fn add_chunk(
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
    if path.starts_with(MEMORY_PREFIX) {
        return "agent_memory"; // no file behind it
    }
    match bounded_read(Path::new(path)) {
        Ok(b) if hash(&b) == sha => "current",
        Ok(_) => "stale",
        Err(_) => "missing_or_unreadable",
    }
}
pub fn hit(r: &rusqlite::Row<'_>, full: bool) -> rusqlite::Result<Value> {
    let path: String = r.get(1)?;
    let sha: String = r.get(2)?;
    let status: String = r.get(4)?;
    let text: String = r.get(9)?;
    Ok(
        json!({"id":r.get::<_,i64>(0)?,"path":path,"sha256":sha,"revision":r.get::<_,i64>(3)?,"status":status,"extractor":r.get::<_,String>(5)?,"heading":r.get::<_,String>(6)?,"page":r.get::<_,Option<i64>>(7)?,"line":r.get::<_,i64>(8)?,"text":text.chars().take(if full {2000} else {500}).collect::<String>(),"freshness":freshness(&path,&sha,&status),"trust":"untrusted_source_data"}),
    )
}
/// What a search returns per hit: where it is and what it says. Hashes, revision, status and extractor stay in `get <id>`;
/// the untrusted-data label is stated once per response (`TRUST`). About a third fewer characters per result list.
pub fn lean(mut hits: Vec<Value>) -> Vec<Value> {
    for v in &mut hits {
        if let Some(o) = v.as_object_mut() {
            for k in ["sha256", "revision", "status", "extractor", "trust"] {
                o.remove(k);
            }
            if o.get("page").is_some_and(Value::is_null) {
                o.remove("page");
            }
            if let Some(p) = o.get("path").and_then(Value::as_str) {
                let plain = plain_path(p);
                o.insert("path".into(), plain.into());
            }
        }
    }
    hits
}
/// Windows canonical paths carry the verbatim prefix `\\?\`, which many tools cannot open; agents get the ordinary form.
fn plain_path(p: &str) -> String {
    match p.strip_prefix(r"\\?\") {
        Some(rest) => rest.strip_prefix(r"UNC\").map_or_else(|| rest.to_string(), |share| format!(r"\\{share}")),
        None => p.to_string(),
    }
}
pub const TRUST: &str = "untrusted_source_data";
pub const HIT: &str = "SELECT c.id,s.path,s.sha256,s.revision,s.status,s.extractor,c.heading,c.page,c.line,c.text FROM chunks c JOIN sources s ON s.id=c.source_id";
/// Counts the chunks already returned from each source file, so one long file cannot fill the whole result list.
pub struct PerSource {
    cap: Option<usize>,
    seen: std::collections::HashMap<String, usize>,
}
impl PerSource {
    pub fn new(cap: Option<usize>) -> Self {
        Self {
            cap,
            seen: Default::default(),
        }
    }
    /// True when another chunk of this source may be returned (and records it).
    pub fn allow(&mut self, path: &str) -> bool {
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
pub fn lexical_chunks(c: &Connection, q: &str, n: usize) -> Result<Vec<(i64, f64)>> {
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
pub fn match_expr(q: &str) -> String {
    q.split_whitespace()
        .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}
pub fn search(
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
    if let Some(ids) = crate::metrics::timed!(
        "search.keywords",
        crate::lexical::top_ids(c, q, (limit * 30).max(100))
    )
    .ok()
    .flatten()
    {
        let mut s = c.prepare(&format!(
            "{HIT} WHERE c.id=?1 AND s.status='indexed' AND c.id=(SELECT min(c2.id) FROM chunks c2 JOIN sources s2 ON s2.id=c2.source_id WHERE s2.sha256=s.sha256 AND s2.status='indexed' AND c2.heading=c.heading AND c2.page IS c.page AND c2.line=c.line AND c2.text=c.text)"
        ))?;
        let mut results = Vec::new();
        let mut cap = PerSource::new(filters.per_source);
        // shortcut: the pre-filter applies to the keyword shortlist (30 x limit, at least 100), enough for memory databases.
        for id in ids
            .into_iter()
            .filter(|id| filters.only.as_ref().is_none_or(|o| o.contains(id)))
        {
            if results.len() >= text_limit {
                break;
            }
            if let Some(v) = s.query_row([id], |r| hit(r, false)).optional()?
                && cap.allow(v["path"].as_str().unwrap_or(""))
            {
                results.push(v);
            }
        }
        return Ok(
            json!({"mode":"lexical_bm25","results":lean(results),"records":records,"trust":TRUST}),
        );
    }
    let sql = format!(
        "{HIT} JOIN search_fts f ON f.rowid=c.id WHERE search_fts MATCH ?1 AND s.status='indexed' AND c.id=(SELECT min(c2.id) FROM chunks c2 JOIN sources s2 ON s2.id=c2.source_id WHERE s2.sha256=s.sha256 AND s2.status='indexed' AND c2.heading=c.heading AND c2.page IS c.page AND c2.line=c.line AND c2.text=c.text) ORDER BY bm25(search_fts),c.id LIMIT ?2"
    );
    let mut s = c.prepare(&sql)?;
    let fetch = if filters.only.is_some() {
        10_000
    } else if filters.per_source.is_some() {
        text_limit * 10
    } else {
        text_limit
    };
    let mut cap = PerSource::new(filters.per_source);
    let results: Vec<Value> = s
        .query_map(params![expr, fetch as i64], |r| hit(r, false))?
        .collect::<rusqlite::Result<Vec<Value>>>()?
        .into_iter()
        .filter(|v| {
            filters
                .only
                .as_ref()
                .is_none_or(|o| v["id"].as_i64().is_some_and(|id| o.contains(&id)))
        })
        .filter(|v| cap.allow(v["path"].as_str().unwrap_or("")))
        .take(text_limit)
        .collect();
    Ok(json!({"mode":"lexical_fts5","results":lean(results),"records":records,"trust":TRUST}))
}
pub fn stats(c: &Connection) -> Result<Value> {
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
pub fn backup(c: &Connection, output: &Path) -> Result<Value> {
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

#[cfg(test)]
mod chunk_tests {
    use super::*;

    #[test]
    fn search_paths_drop_the_windows_verbatim_prefix() {
        assert_eq!(plain_path(r"\\?\D:\docs\a.md"), r"D:\docs\a.md");
        assert_eq!(plain_path(r"\\?\UNC\srv\share\a.md"), r"\\srv\share\a.md");
        assert_eq!(plain_path("/home/u/a.md"), "/home/u/a.md");
        let out = lean(vec![json!({"path": r"\\?\D:\a.md", "sha256": "x", "page": null})]);
        assert_eq!(out[0], json!({"path": r"D:\a.md"}));
    }

    #[test]
    fn chunks_follow_structure_merge_short_sections_and_keep_code_whole() {
        let dir = tempfile::tempdir().unwrap();
        let c = open(&dir.path().join("t.db"), true).unwrap();
        c.execute("INSERT INTO sources(root,path,sha256,revision,status,extractor) VALUES('r','p','s',1,'indexed','md')", [])
            .unwrap();
        let id = c.last_insert_rowid();
        let code = format!(
            "# not a heading\n{}",
            "let x = 1; // some code\n".repeat(90)
        ); // ~2,000 chars
        let doc = format!(
            "# T\nintro\n## A\n{}\n## B\n```sh\n{code}```\n## C\nshort\n### D\nd\n# U\nu\n",
            "a".repeat(600)
        );
        text_chunks(&c, id, &doc, None).unwrap();
        let rows: Vec<(String, String)> = c
            .prepare("SELECT heading, text FROM chunks ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let heads: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
        // A's short intro joins A; B keeps its whole code block; D joins its short parent C; U leaves C's parent.
        assert_eq!(heads, ["T", "T > B", "T > C", "U"]);
        assert_eq!(rows[1].1.matches("```").count(), 2);
        assert!(rows[2].1.contains("### D"));
        assert!(heading_of("#hashtag").is_none() && heading_of("#!/bin/sh").is_none());
    }
}
