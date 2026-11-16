//! Footnote Code, stage 1: the precise pass of docs/CODE-GRAPH-DESIGN.md (section 5). A compiler-grade indexer writes SCIP
//! (`rust-analyzer scip` for Rust); Footnote keeps the project's definitions, references and who calls whom in one SQLite file.
//! Every edge says how it was made (`origin`: `compiler` here); the tree-sitter pass for other languages comes later.
pub mod scip;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

const SCHEMA: &str = "
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files(id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE, sha256 TEXT NOT NULL);
CREATE TABLE symbols(id INTEGER PRIMARY KEY, scip TEXT NOT NULL UNIQUE, name TEXT NOT NULL, kind TEXT NOT NULL,
  signature TEXT NOT NULL, file_id INTEGER REFERENCES files(id), line INTEGER, end_line INTEGER);
CREATE INDEX symbols_name ON symbols(name);
CREATE TABLE refs(symbol_id INTEGER NOT NULL REFERENCES symbols(id), file_id INTEGER NOT NULL REFERENCES files(id),
  line INTEGER NOT NULL, caller_id INTEGER REFERENCES symbols(id), origin TEXT NOT NULL);
CREATE INDEX refs_symbol ON refs(symbol_id);
CREATE INDEX refs_caller ON refs(caller_id);
CREATE TABLE outside(file_id INTEGER NOT NULL REFERENCES files(id), package TEXT NOT NULL, version TEXT NOT NULL,
  refs INTEGER NOT NULL);";

/// Package and version of a SCIP symbol (`<scheme> <manager> <package> <version> <descriptors>`).
fn package(symbol: &str) -> Option<(&str, &str)> {
    if symbol.starts_with("local ") {
        return None;
    }
    let mut f = symbol.splitn(5, ' ');
    let (_, _, pkg, ver) = (f.next()?, f.next()?, f.next()?, f.next()?);
    Some((pkg, ver))
}

/// What a SCIP symbol is, from its last descriptor (`().` method, `#` type, `/` namespace, `.` term).
fn kind(symbol: &str) -> &'static str {
    if symbol.ends_with(").") {
        "function"
    } else if symbol.ends_with('#') {
        "type"
    } else if symbol.ends_with('/') {
        "module"
    } else if symbol.ends_with('.') {
        "value"
    } else {
        "other"
    }
}

/// Last name in a SCIP symbol, for symbols the indexer gave no display name.
fn short_name(symbol: &str) -> String {
    let mut s = symbol.trim_end_matches(['.', '#', '/']);
    if s.ends_with(')') {
        s = s.rfind('(').map_or(s, |i| &s[..i]); // method disambiguator: `run().`, `run(+1).`
    }
    s.rsplit(['/', '#', '.', ' '])
        .next()
        .unwrap_or(s)
        .trim_matches('`')
        .to_string()
}

fn contains(outer: [i32; 4], at: [i32; 4]) -> bool {
    (outer[0], outer[1]) <= (at[0], at[1]) && (at[2], at[3]) <= (outer[2], outer[3])
}

/// What to tell someone without the Rust indexer.
pub const INSTALL_INDEXER: &str = "Footnote Code needs rust-analyzer to read Rust projects: install it with `rustup component add rust-analyzer`";

/// Run the indexer on `root` and build the graph into `db` (written beside it, then renamed: a reader never sees half a graph).
pub fn index(root: &Path, db: &Path) -> Result<Value> {
    let started = Instant::now();
    if !root.join("Cargo.toml").is_file() {
        bail!(
            "no Cargo.toml in {}: Footnote Code reads Rust projects for now",
            root.display()
        );
    }
    let parent = db.parent().context("database path")?;
    std::fs::create_dir_all(parent)?;
    let scip_file = db.with_extension("scip");
    let out = std::process::Command::new("rust-analyzer")
        .arg("scip")
        .arg(root)
        .arg("--output")
        .arg(&scip_file)
        .output();
    let out = match out {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(INSTALL_INDEXER),
        r => r.context("run rust-analyzer")?,
    };
    // rustup's proxy exists but the component is missing: it exits with "Unknown binary 'rust-analyzer'".
    if !out.status.success() && String::from_utf8_lossy(&out.stderr).contains("Unknown binary") {
        bail!(INSTALL_INDEXER);
    }
    if !out.status.success() {
        bail!(
            "rust-analyzer scip failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let indexer_ms = started.elapsed().as_millis();
    let bytes = std::fs::read(&scip_file)?;
    let _ = std::fs::remove_file(&scip_file);
    let docs = scip::read(&bytes)?;
    let building = db.with_extension("building");
    let _ = std::fs::remove_file(&building);
    let counts = write(&building, root, &docs)?;
    std::fs::rename(&building, db)?;
    Ok(
        json!({"root": root, "db": db, "indexer": "rust-analyzer scip", "files": counts.0, "symbols": counts.1,
        "references": counts.2, "indexer_ms": indexer_ms, "total_ms": started.elapsed().as_millis()}),
    )
}

fn write(db: &Path, root: &Path, docs: &[scip::Document]) -> Result<(usize, usize, usize)> {
    let mut c = Connection::open(db)?;
    c.execute_batch(SCHEMA)?;
    let tx = c.transaction()?;
    tx.execute(
        "INSERT INTO meta VALUES('root', ?1)",
        [root.to_string_lossy()],
    )?;
    let mut names: HashMap<&str, (&str, &str)> = HashMap::new();
    for d in docs {
        for s in &d.symbols {
            names.insert(&s.symbol, (&s.display_name, &s.signature));
        }
    }
    let (mut ids, mut file_ids) = (HashMap::<&str, i64>::new(), Vec::new());
    for d in docs {
        let sha = std::fs::read(root.join(&d.path))
            .map(|b| footnote_core::hash(&b))
            .unwrap_or_default();
        tx.execute(
            "INSERT INTO files(path, sha256) VALUES(?1, ?2)",
            params![d.path, sha],
        )?;
        let file = tx.last_insert_rowid();
        file_ids.push(file);
        for o in d
            .occurrences
            .iter()
            .filter(|o| o.roles & scip::DEFINITION != 0 && !o.symbol.starts_with("local "))
        {
            if ids.contains_key(o.symbol.as_str()) {
                continue;
            }
            let (display, signature) = names.get(o.symbol.as_str()).copied().unwrap_or(("", ""));
            let name = if display.is_empty() {
                short_name(&o.symbol)
            } else {
                display.to_string()
            };
            let end = o.enclosing.unwrap_or(o.range)[2];
            tx.execute(
                "INSERT INTO symbols(scip, name, kind, signature, file_id, line, end_line) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![o.symbol, name, kind(&o.symbol), signature, file, o.range[0] + 1, end + 1],
            )?;
            ids.insert(&o.symbol, tx.last_insert_rowid());
        }
    }
    let mut refs = 0;
    let ours: std::collections::HashSet<&str> =
        ids.keys().filter_map(|s| package(s).map(|p| p.0)).collect();
    let mut outside: HashMap<(i64, &str, &str), i64> = HashMap::new();
    {
        let mut add = tx.prepare("INSERT INTO refs VALUES(?1, ?2, ?3, ?4, 'compiler')")?;
        for (d, &file) in docs.iter().zip(&file_ids) {
            // Items of this file that can hold a reference (not the module itself), innermost found by the smallest span.
            let items: Vec<(&str, [i32; 4])> = d
                .occurrences
                .iter()
                .filter(|o| {
                    o.roles & scip::DEFINITION != 0
                        && kind(&o.symbol) != "module"
                        && !o.symbol.starts_with("local ")
                })
                .filter_map(|o| o.enclosing.map(|e| (o.symbol.as_str(), e)))
                .collect();
            for o in d
                .occurrences
                .iter()
                .filter(|o| o.roles & scip::DEFINITION == 0)
            {
                let Some(&target) = ids.get(o.symbol.as_str()) else {
                    // A reference into another package (not a bare path to it): kept as a count for the architecture map.
                    if let Some((pkg, ver)) = package(&o.symbol)
                        .filter(|p| !ours.contains(p.0) && kind(&o.symbol) != "module")
                    {
                        *outside.entry((file, pkg, ver)).or_default() += 1;
                    }
                    continue;
                };
                let caller = items
                    .iter()
                    .filter(|(_, e)| contains(*e, o.range))
                    .min_by_key(|(_, e)| (e[2] - e[0], e[3] - e[1]))
                    .and_then(|(s, _)| ids.get(s).copied());
                add.execute(params![target, file, o.range[0] + 1, caller])?;
                refs += 1;
            }
        }
    }
    for ((file, pkg, ver), n) in outside {
        tx.execute(
            "INSERT INTO outside VALUES(?1, ?2, ?3, ?4)",
            params![file, pkg, ver, n],
        )?;
    }
    tx.commit()?;
    Ok((docs.len(), ids.len(), refs))
}

/// Definitions named `name`, each with signature, place, callers and callees. `stale` lists answer files changed since indexing.
pub fn lookup(db: &Path, name: &str, limit: usize) -> Result<Value> {
    let c = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
            format!(
                "no code graph at {}; run `footnote code-index` first",
                db.display()
            )
        })?;
    let root: String = c.query_row("SELECT value FROM meta WHERE key='root'", [], |r| r.get(0))?;
    let mut defs = c.prepare(
        "SELECT s.id, s.name, s.kind, s.signature, f.path, s.line, s.end_line FROM symbols s JOIN files f ON f.id = s.file_id
         WHERE s.name = ?1 AND s.kind != 'module' ORDER BY f.path, s.line LIMIT 10",
    )?;
    let edges = |sql: &str, id: i64| -> Result<Vec<Value>> {
        let mut q = c.prepare(sql)?;
        let rows = q.query_map(params![id, limit as i64], |r| {
            Ok(json!({"name": r.get::<_, String>(0)?, "kind": r.get::<_, String>(1)?, "path": r.get::<_, String>(2)?,
                "lines": r.get::<_, String>(3)?.split(',').filter_map(|l| l.parse::<i64>().ok()).collect::<Vec<_>>()}))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    };
    let mut paths = Vec::new();
    let mut found = Vec::new();
    let rows = defs.query_map([name], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, i64>(6)?,
        ))
    })?;
    for row in rows {
        let (id, name, kind, signature, path, line, end) = row?;
        // Callers: items whose body holds a reference to this one; the lines are the call sites.
        let callers = edges(
            "SELECT s.name, s.kind, f.path, group_concat(r.line) FROM refs r JOIN symbols s ON s.id = r.caller_id
             JOIN files f ON f.id = r.file_id WHERE r.symbol_id = ?1 AND r.caller_id != ?1
             GROUP BY r.caller_id, r.file_id ORDER BY f.path LIMIT ?2",
            id,
        )?;
        // Callees: what its body references, defined in this project; the lines are where it refers to them.
        let callees = edges(
            "SELECT s.name, s.kind, f.path, group_concat(r.line) FROM refs r JOIN symbols s ON s.id = r.symbol_id
             JOIN files f ON f.id = s.file_id WHERE r.caller_id = ?1 AND r.symbol_id != ?1 AND s.kind != 'module'
             GROUP BY r.symbol_id ORDER BY s.name LIMIT ?2",
            id,
        )?;
        let references: i64 = c.query_row(
            "SELECT count(*) FROM refs WHERE symbol_id = ?1",
            [id],
            |r| r.get(0),
        )?;
        paths.push(path.clone());
        paths.extend(
            callers
                .iter()
                .filter_map(|v| v["path"].as_str().map(str::to_string)),
        );
        found.push(json!({"name": name, "kind": kind, "signature": signature, "path": path, "lines": [line, end],
            "references": references, "callers": callers, "callees": callees}));
    }
    paths.sort();
    paths.dedup();
    let mut stale = Vec::new();
    for p in paths {
        let indexed: Option<String> = c
            .query_row("SELECT sha256 FROM files WHERE path = ?1", [&p], |r| {
                r.get(0)
            })
            .optional()?;
        let now = std::fs::read(Path::new(&root).join(&p))
            .map(|b| footnote_core::hash(&b))
            .ok();
        if indexed != now {
            stale.push(p);
        }
    }
    Ok(json!({"root": root, "origin": "compiler", "symbols": found, "stale": stale}))
}

/// What an outside package is for, from a short hand-written list; `None` when not listed.
fn class(package: &str) -> Option<&'static str> {
    Some(match package {
        "rusqlite" | "sqlx" | "diesel" | "postgres" | "tokio-postgres" | "redis" | "mongodb"
        | "sled" | "rocksdb" | "duckdb" | "libsql" | "tantivy" | "qdrant-client" | "lancedb"
        | "sea-orm" => "database",
        "reqwest" | "ureq" | "hyper" | "axum" | "actix-web" | "warp" | "tonic" | "url" | "http"
        | "isahc" | "surf" | "tungstenite" | "tokio-tungstenite" | "rocket" | "poem" | "iroh" => {
            "network"
        }
        "async-openai" | "openai" | "anthropic" | "ollama-rs" | "rig-core" | "genai" | "ort"
        | "llama-cpp-2" | "candle-core" | "tokenizers" | "mistralrs" => "model",
        "std" | "core" | "alloc" => "standard library",
        _ => return None,
    })
}

/// The block a file belongs to: the first `depth` folders of its path (`.` for files at the root).
fn block(path: &str, depth: usize) -> String {
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    let dirs = &parts[..parts.len() - 1];
    if dirs.is_empty() {
        ".".into()
    } else {
        dirs[..depth.min(dirs.len())].join("/")
    }
}

/// The architecture map: blocks (folders, `depth` deep) with their symbols, links between blocks counted in
/// references, and the outside packages each block uses.
pub fn map(db: &Path, depth: usize) -> Result<Value> {
    let c = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
            format!(
                "no code graph at {}; run `footnote code-index` first",
                db.display()
            )
        })?;
    let has_outside: bool = c.query_row(
        "SELECT count(*) > 0 FROM sqlite_master WHERE name = 'outside'",
        [],
        |r| r.get(0),
    )?;
    if !has_outside {
        bail!(
            "this code graph was built before the architecture map; run `footnote code-index` again"
        );
    }
    let root: String = c.query_row("SELECT value FROM meta WHERE key='root'", [], |r| r.get(0))?;
    let blocks: HashMap<i64, String> = c
        .prepare("SELECT id, path FROM files")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .map(|r| r.map(|(id, p)| (id, block(&p, depth))))
        .collect::<rusqlite::Result<_>>()?;
    let rows = |sql: &str| -> Result<Vec<(i64, i64, i64)>> {
        Ok(c.prepare(sql)?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?)
    };
    let mut sizes: HashMap<&str, (i64, i64)> = HashMap::new(); // block -> (files, symbols)
    for b in blocks.values() {
        sizes.entry(b).or_default().0 += 1;
    }
    for (file, n, _) in
        rows("SELECT file_id, count(*), 0 FROM symbols WHERE kind != 'module' GROUP BY file_id")?
    {
        sizes.entry(&blocks[&file]).or_default().1 += n;
    }
    let mut links: HashMap<(&str, &str), i64> = HashMap::new();
    for (from, to, n) in rows(
        "SELECT r.file_id, s.file_id, count(*) FROM refs r JOIN symbols s ON s.id = r.symbol_id WHERE s.kind != 'module' GROUP BY 1, 2",
    )? {
        let (a, b) = (blocks[&from].as_str(), blocks[&to].as_str());
        if a != b {
            *links.entry((a, b)).or_default() += n;
        }
    }
    let mut outside: HashMap<(String, String), (i64, std::collections::BTreeSet<&str>)> =
        HashMap::new();
    let mut q = c.prepare("SELECT file_id, package, version, refs FROM outside")?;
    let found = q.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    for r in found {
        let (file, pkg, ver, n) = r?;
        let e = outside.entry((pkg, ver)).or_default();
        e.0 += n;
        e.1.insert(&blocks[&file]);
    }
    let mut blocks: Vec<_> = sizes.into_iter().collect();
    blocks.sort_by(|a, b| b.1.1.cmp(&a.1.1).then(a.0.cmp(b.0)));
    let mut links: Vec<_> = links.into_iter().collect();
    links.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut outside: Vec<_> = outside.into_iter().collect();
    outside.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    Ok(json!({
        "root": root, "origin": "compiler", "depth": depth,
        "blocks": blocks.iter().map(|(b, (f, s))| json!({"name": b, "files": f, "symbols": s})).collect::<Vec<_>>(),
        "links": links.iter().map(|((a, b), n)| json!({"from": a, "to": b, "references": n})).collect::<Vec<_>>(),
        "outside": outside.iter().map(|((p, v), (n, used))| json!({"package": p, "version": v, "references": n,
            "class": class(p), "used_by": used})).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_names_come_from_scip_descriptors() {
        let f = "rust-analyzer cargo footnote 0.1.0 app/run().";
        assert_eq!((kind(f), short_name(f).as_str()), ("function", "run"));
        let t = "rust-analyzer cargo footnote 0.1.0 store/Filters#";
        assert_eq!((kind(t), short_name(t).as_str()), ("type", "Filters"));
        assert!(contains([1, 0, 9, 1], [4, 2, 4, 8]) && !contains([1, 0, 3, 1], [4, 2, 4, 8]));
        assert_eq!(
            package("rust-analyzer cargo serde_json 1.0.1 Value#"),
            Some(("serde_json", "1.0.1"))
        );
        assert_eq!(package("local 3"), None);
    }

    #[test]
    fn blocks_are_the_first_folders_of_a_path() {
        assert_eq!(block("crates\\code\\src\\lib.rs", 2), "crates/code");
        assert_eq!(block("src/app.rs", 2), "src");
        assert_eq!(block("build.rs", 2), ".");
        assert_eq!(block("crates/code/src/lib.rs", 1), "crates");
    }
}
