//! Footnote Code, stage 1: the precise pass of docs/CODE-GRAPH-DESIGN.md (section 5). A compiler-grade indexer writes SCIP
//! (`rust-analyzer scip` for Rust); Footnote keeps the project's definitions, references and who calls whom in one SQLite file.
//! Every edge says how it was made (`origin`: `compiler` here); the tree-sitter pass for other languages comes later.
pub mod scip;
mod ts;
pub use ts::{import_counts, imports};

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
CREATE TABLE guesses(symbol_id INTEGER NOT NULL REFERENCES symbols(id), file_id INTEGER NOT NULL REFERENCES files(id),
  line INTEGER NOT NULL, caller_id INTEGER REFERENCES symbols(id), rule TEXT NOT NULL);
CREATE INDEX guesses_symbol ON guesses(symbol_id);
CREATE TABLE unresolved(file_id INTEGER NOT NULL REFERENCES files(id), line INTEGER NOT NULL, caller_id INTEGER REFERENCES symbols(id),
  name TEXT NOT NULL, candidates TEXT NOT NULL);
CREATE INDEX unresolved_name ON unresolved(name);
CREATE TABLE outside(file_id INTEGER NOT NULL REFERENCES files(id), package TEXT NOT NULL, version TEXT NOT NULL,
  refs INTEGER NOT NULL);";

/// How the graph's links were made: `compiler` (a precise indexer) or `resolved` (tree-sitter, through scopes and imports).
/// Graphs built before the grade was stored are compiler graphs.
fn origin(c: &Connection) -> Result<String> {
    Ok(
        c.query_row("SELECT value FROM meta WHERE key='origin'", [], |r| {
            r.get(0)
        })
        .optional()?
        .unwrap_or_else(|| "compiler".into()),
    )
}

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

/// Calls inside macro arguments, which rust-analyzer's index leaves out (#197): the parser's link, moved onto the compiler's
/// own definition (same file, line and name) and kept graded `resolved` through `scip::MACRO_ARG`.
fn merge_macro_calls(docs: &mut [scip::Document], parsed: &[scip::Document]) {
    let norm = |p: &str| p.replace('\\', "/");
    let mut defs: HashMap<(String, i32, String), String> = HashMap::new();
    for d in docs.iter() {
        for o in d
            .occurrences
            .iter()
            .filter(|o| o.roles & scip::DEFINITION != 0 && !o.symbol.starts_with("local "))
        {
            defs.entry((norm(&d.path), o.range[0], short_name(&o.symbol)))
                .or_insert_with(|| o.symbol.clone());
        }
    }
    let at: HashMap<String, usize> = docs
        .iter()
        .enumerate()
        .map(|(i, d)| (norm(&d.path), i))
        .collect();
    for p in parsed {
        let Some(&i) = at.get(&norm(&p.path)) else {
            continue;
        };
        for o in p
            .occurrences
            .iter()
            .filter(|o| o.roles & scip::MACRO_ARG != 0)
        {
            let Some(target) = treesitter_target(&o.symbol).and_then(|k| defs.get(&k)) else {
                continue;
            };
            let doc = &mut docs[i];
            if doc
                .occurrences
                .iter()
                .any(|x| x.range[0] == o.range[0] && &x.symbol == target)
            {
                continue; // the compiler has this one already
            }
            doc.occurrences.push(scip::Occurrence {
                range: o.range,
                symbol: target.clone(),
                roles: scip::MACRO_ARG,
                enclosing: None,
                ..Default::default()
            });
        }
    }
}

/// `treesitter local <project> . <path>/<name>:<line><suffix>` -> (path, line, name).
fn treesitter_target(s: &str) -> Option<(String, i32, String)> {
    let (path, tail) = s.split_once(" . ")?.1.rsplit_once('/')?;
    let (name, line) = tail.split_once(':')?;
    let line = line
        .trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()?;
    Some((path.to_string(), line, name.to_string()))
}

/// Last name in a SCIP symbol, for symbols the indexer gave no display name.
fn short_name(symbol: &str) -> String {
    // scip-typescript names object-literal members `spawnSession0:`, the name plus a counter per object in the file (#216).
    // shortcut: a member whose own name ends in a digit loses it, rare; split on the counter if lookups miss such names.
    if let Some(m) = symbol.strip_suffix(':') {
        let m = m.trim_end_matches(|c: char| c.is_ascii_digit());
        return short_name(&format!("{m}."));
    }
    let mut s = symbol.trim_end_matches(['.', '#', '/']);
    if s.ends_with(')') {
        s = s.rfind('(').map_or(s, |i| &s[..i]); // method disambiguator: `run().`, `run(+1).`
    }
    // `:` too: a member of an inline type is `typeLiteral10:entry_date.` (scip-typescript).
    s.rsplit(['/', '#', '.', ' ', ':'])
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
const INSTALL_TS: &str = "Footnote Code needs scip-typescript to read TypeScript projects: install it with `npm install -g @sourcegraph/scip-typescript`";
const INSTALL_PY: &str = "Footnote Code needs scip-python to read Python projects: install it with `npm install -g @sourcegraph/scip-python`";

/// The compiler-grade indexer for a project, chosen from its marker files (#110). Footnote never writes into the project: a
/// TypeScript or JavaScript project needs its own tsconfig.json (scip-typescript's --infer-tsconfig would write one there).
struct Indexer {
    name: &'static str,
    program: &'static str,
    args: Vec<std::ffi::OsString>,
    install: &'static str,
}

fn indexer(root: &Path, out: &Path) -> Result<Indexer> {
    let has = |f: &str| root.join(f).is_file();
    if has("Cargo.toml") {
        return Ok(Indexer {
            name: "rust-analyzer scip",
            program: "rust-analyzer",
            args: vec!["scip".into(), root.into(), "--output".into(), out.into()],
            install: INSTALL_INDEXER,
        });
    }
    if has("tsconfig.json") {
        return Ok(Indexer {
            name: "scip-typescript",
            // npm installs a .cmd launcher on Windows, which Command does not find by name alone.
            program: if cfg!(windows) {
                "scip-typescript.cmd"
            } else {
                "scip-typescript"
            },
            args: vec![
                "index".into(),
                "--no-progress-bar".into(),
                "--output".into(),
                out.into(),
            ],
            install: INSTALL_TS,
        });
    }
    if [
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "requirements.txt",
    ]
    .iter()
    .any(|f| has(f))
    {
        let name = root
            .file_name()
            .map_or_else(|| "project".into(), |n| n.to_string_lossy().into_owned());
        let mut args = Vec::new();
        if cfg!(windows) {
            // Resolve the global npm package beside its PATH shim, never from the project being indexed.
            let entry = std::env::var_os("PATH")
                .into_iter()
                .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
                .filter(|p| p.join("scip-python.cmd").is_file())
                .map(|p| p.join("node_modules/@sourcegraph/scip-python/index.js"))
                .find(|p| p.is_file())
                .with_context(|| INSTALL_PY)?;
            args.extend([
                "-e".into(),
                include_str!("scip_python_windows.cjs").into(),
                entry.into_os_string(),
            ]);
        }
        args.extend([
            "index".into(),
            ".".into(),
            "--project-name".into(),
            name.into(),
            // Without it scip-python asks git for a version and, outside a repository, crashes on the first module (#216).
            "--project-version".into(),
            "HEAD".into(),
            "--output".into(),
            out.into(),
        ]);
        return Ok(Indexer {
            name: "scip-python",
            program: if cfg!(windows) { "node" } else { "scip-python" },
            args,
            install: INSTALL_PY,
        });
    }
    if has("package.json") {
        bail!(
            "{}: a JavaScript project needs a tsconfig.json (with allowJs) for Footnote Code; Footnote never writes one into your project",
            root.display()
        );
    }
    bail!(
        "{}: no Cargo.toml, tsconfig.json or Python project file; Footnote Code reads Rust, TypeScript/JavaScript and Python",
        root.display()
    )
}

/// Build the graph of `root` into `db` (written beside it, then renamed: a reader never sees half a graph). The precise pass runs
/// when the project's compiler indexer is installed and succeeds; otherwise the tree-sitter base layer builds the graph and the
/// answer's `upgrade` says what would make it precise (CODE-GRAPH-DESIGN.md section 12).
pub fn index(root: &Path, db: &Path) -> Result<Value> {
    let started = Instant::now();
    let parent = db.parent().context("database path")?;
    std::fs::create_dir_all(parent)?;
    let scip_file = std::path::absolute(db.with_extension("scip"))?;
    let project = root.file_name().map_or_else(
        || "project".into(),
        |n| n.to_string_lossy().replace(' ', "_"),
    );
    // Opt-in overlay: a SCIP index that is already there (rust-analyzer, scip-typescript, scip-python, scip-java) is read as the
    // compiler graph instead of running an indexer; the parser still fills the files it does not cover. A bad file is an error.
    let ready = match std::env::var_os("FOOTNOTE_CODE_SCIP") {
        Some(f) => {
            let docs = scip::read(
                &std::fs::read(&f)
                    .with_context(|| format!("FOOTNOTE_CODE_SCIP {}", Path::new(&f).display()))?,
            )?;
            Ok(("existing SCIP index".to_string(), docs, vec![]))
        }
        None => precise(root, &scip_file),
    };
    let (name, mut docs, origin, upgrade, report) = match ready {
        Ok((name, docs, report)) => (name, docs, "compiler", None, report),
        Err(e) => {
            let (docs, _) = ts::index(root, &project)?;
            (
                "tree-sitter".into(),
                docs,
                "resolved",
                Some(format!("{e:#}")),
                vec![],
            )
        }
    };
    // A mixed project: the precise indexer reads one language; the parser adds every other file, its links graded `resolved`.
    let split = docs.len();
    if origin == "compiler" {
        let covered: std::collections::HashSet<String> =
            docs.iter().map(|d| d.path.replace('\\', "/")).collect();
        if let Ok((extra, _)) = ts::index(root, &project) {
            let (inside, outside): (Vec<_>, Vec<_>) =
                extra.into_iter().partition(|d| covered.contains(&d.path));
            merge_macro_calls(&mut docs, &inside);
            docs.extend(outside);
        }
    }
    let also_parsed = docs.len() - split;
    let indexer_ms = started.elapsed().as_millis();
    let building = db.with_extension("building");
    let _ = std::fs::remove_file(&building);
    let counts = write(&building, root, &docs, origin, split)?;
    std::fs::rename(&building, db)?;
    let mut out = json!({"root": root, "db": db, "indexer": name, "origin": origin, "also_parsed": also_parsed, "files": counts.0, "symbols": counts.1,
        "references": counts.2, "indexer_ms": indexer_ms, "total_ms": started.elapsed().as_millis()});
    if let Some(u) = upgrade {
        out["upgrade"] = json!(u);
    }
    if !report.is_empty() {
        out["roots"] = json!(report);
    }
    Ok(out)
}

/// Where compiler indexers run: the root when it has a marker file, plus every top-level Rust project in a subfolder (a Tauri
/// app's `src-tauri`, `crates/<name>`), so a project whose Rust is not at the top still gets a compiler graph (#218).
fn roots(root: &Path) -> Vec<String> {
    const VENDORED: &[&str] = &[
        "vendor",
        "third_party",
        "dependencies",
        "node_modules",
        "target",
    ];
    let mut cargo: Vec<String> = ts::candidates(root)
        .into_iter()
        .filter_map(|p| {
            Some(
                p.strip_suffix("Cargo.toml")?
                    .trim_end_matches('/')
                    .to_string(),
            )
        })
        .filter(|d| {
            d.split('/')
                .all(|s| !s.starts_with('.') && !VENDORED.contains(&s))
        })
        .collect();
    cargo.sort();
    let mut out: Vec<String> = Vec::new();
    for d in cargo {
        // Keep the topmost folder only: a workspace's member crates are indexed with it.
        if !out
            .iter()
            .any(|o| o.is_empty() || d.starts_with(&format!("{o}/")))
        {
            out.push(d);
        }
    }
    if !out.iter().any(String::is_empty) && indexer(root, Path::new("x")).is_ok() {
        out.insert(0, String::new());
    }
    out.truncate(8); // shortcut: 8 roots at most, each a full compiler run; raise when a monorepo needs more.
    out
}

/// The precise pass over every root: each indexer's documents, paths made relative to `root`. Fails only when no root worked.
fn precise(root: &Path, scip_file: &Path) -> Result<(String, Vec<scip::Document>, Vec<Value>)> {
    let (mut names, mut docs, mut report, mut first_err) =
        (Vec::new(), Vec::new(), Vec::new(), None);
    let all = roots(root);
    if all.is_empty() {
        indexer(root, scip_file)?; // no marker anywhere: its message says which files Footnote looks for
    }
    for sub in all {
        let dir = if sub.is_empty() {
            root.to_path_buf()
        } else {
            root.join(&sub)
        };
        match precise_at(&dir, scip_file) {
            Ok((name, found)) => {
                report.push(json!({"root": sub, "indexer": name, "files": found.len()}));
                if !names.contains(&name) {
                    names.push(name);
                }
                docs.extend(found.into_iter().map(|mut d| {
                    if !sub.is_empty() {
                        d.path = format!("{sub}/{}", d.path.replace('\\', "/"));
                    }
                    d
                }));
            }
            Err(e) => {
                report.push(json!({"root": sub, "error": format!("{e:#}")}));
                first_err.get_or_insert(e);
            }
        }
    }
    if names.is_empty() {
        return Err(first_err.unwrap_or_else(|| anyhow::anyhow!("no compiler indexer ran")));
    }
    Ok((names.join(" + "), docs, report))
}

/// One compiler indexer run in `root`, its SCIP output read back.
fn precise_at(root: &Path, scip_file: &Path) -> Result<(&'static str, Vec<scip::Document>)> {
    let ix = indexer(root, scip_file)?;
    let out = std::process::Command::new(ix.program)
        .args(&ix.args)
        .current_dir(root)
        .output();
    let out = match out {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(ix.install),
        r => r.with_context(|| format!("run {}", ix.name))?,
    };
    // rustup's proxy exists but the component is missing: it exits with "Unknown binary 'rust-analyzer'".
    if !out.status.success() && String::from_utf8_lossy(&out.stderr).contains("Unknown binary") {
        bail!(ix.install);
    }
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let msg = if err.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout).into_owned()
        } else {
            err.into_owned()
        };
        bail!("{} failed: {}", ix.name, msg.trim());
    }
    let bytes = std::fs::read(scip_file)?;
    let _ = std::fs::remove_file(scip_file);
    Ok((ix.name, scip::read(&bytes)?))
}
fn write(
    db: &Path,
    root: &Path,
    docs: &[scip::Document],
    origin: &str,
    split: usize, // documents from here on come from the parser: their links are graded `resolved`
) -> Result<(usize, usize, usize)> {
    let mut c = Connection::open(db)?;
    c.execute_batch(SCHEMA)?;
    let tx = c.transaction()?;
    tx.execute(
        "INSERT INTO meta VALUES('root', ?1)",
        [root.to_string_lossy()],
    )?;
    tx.execute("INSERT INTO meta VALUES('origin', ?1)", [origin])?;
    let mut names: HashMap<&str, (&str, &str)> = HashMap::new();
    for d in docs {
        for s in &d.symbols {
            names.insert(&s.symbol, (&s.display_name, &s.signature));
        }
    }
    let (mut ids, mut file_ids) = (HashMap::<&str, i64>::new(), Vec::new());
    // Each file in `tests/` is its own crate, and rust-analyzer names its items without the file (`demo 0.1.0 run().`), so
    // two test files with a `fn run` define the same symbol (#197). Each definition is kept, per file, and a reference goes to
    // the one in its own file first.
    let mut own = HashMap::<(usize, &str), i64>::new();
    for (di, d) in docs.iter().enumerate() {
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
            if own.contains_key(&(di, o.symbol.as_str())) {
                continue;
            }
            // Already defined in another file: stored again under a name that carries this file.
            let scip = if ids.contains_key(o.symbol.as_str()) {
                format!("{} @{}", o.symbol, d.path)
            } else {
                o.symbol.clone()
            };
            let (display, signature) = names.get(o.symbol.as_str()).copied().unwrap_or(("", ""));
            let name = if display.is_empty() {
                short_name(&o.symbol)
            } else {
                display.to_string()
            };
            let end = o.enclosing.unwrap_or(o.range)[2];
            tx.execute(
                "INSERT INTO symbols(scip, name, kind, signature, file_id, line, end_line) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![scip, name, kind(&o.symbol), signature, file, o.range[0] + 1, end + 1],
            )?;
            let id = tx.last_insert_rowid();
            ids.entry(&o.symbol).or_insert(id);
            own.insert((di, &o.symbol), id);
        }
    }
    let mut refs = 0;
    let ours: std::collections::HashSet<&str> =
        ids.keys().filter_map(|s| package(s).map(|p| p.0)).collect();
    let mut outside: HashMap<(i64, &str, &str), i64> = HashMap::new();
    {
        let mut add = tx.prepare("INSERT INTO refs VALUES(?1, ?2, ?3, ?4, ?5)")?;
        let mut guess = tx.prepare("INSERT INTO guesses VALUES(?1, ?2, ?3, ?4, ?5)")?;
        let mut miss = tx.prepare("INSERT INTO unresolved VALUES(?1, ?2, ?3, ?4, ?5)")?;
        for (i, (d, &file)) in docs.iter().zip(&file_ids).enumerate() {
            let origin = if i < split { origin } else { "resolved" };
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
                let Some(&target) = own
                    .get(&(i, o.symbol.as_str()))
                    .or_else(|| ids.get(o.symbol.as_str()))
                else {
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
                    .and_then(|(s, _)| own.get(&(i, *s)).or_else(|| ids.get(s)).copied());
                // A lower-tier guess: stored apart, so every default query is unchanged by it.
                if o.roles & scip::GUESS != 0 {
                    guess.execute(params![target, file, o.range[0] + 1, caller, o.rule])?;
                    continue;
                }
                // A call inside macro arguments came from the parser: graded `resolved` even in a compiler file (#197).
                let origin = if o.roles & scip::MACRO_ARG != 0 {
                    "resolved"
                } else {
                    origin
                };
                add.execute(params![target, file, o.range[0] + 1, caller, origin])?;
                refs += 1;
            }
            // Calls the parser could not resolve, each with the item that holds it (by line) and same-named project definitions.
            for u in &d.unresolved {
                let at = u.line - 1;
                let caller = items
                    .iter()
                    .filter(|(_, e)| e[0] <= at && at <= e[2])
                    .min_by_key(|(_, e)| e[2] - e[0])
                    .and_then(|(s, _)| own.get(&(i, *s)).or_else(|| ids.get(s)).copied());
                miss.execute(params![
                    file,
                    u.line,
                    caller,
                    u.name,
                    u.candidates.join(",")
                ])?;
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

/// A file's definitions in line order, each with kind, signature and line range (#240): what an agent reads before
/// opening a large file, so it can read one function instead of the whole file.
pub fn outline(db: &Path, path: &str) -> Result<Value> {
    let c = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
            format!(
                "no code graph at {}; run `footnote code-index` first",
                db.display()
            )
        })?;
    let path = path.replace('\\', "/");
    let mut s = c.prepare(
        "SELECT s.name, s.kind, s.signature, s.line, s.end_line FROM symbols s JOIN files f ON f.id = s.file_id
         WHERE replace(f.path, '\\', '/') = ?1 AND s.kind != 'module' ORDER BY s.line",
    )?;
    let symbols: Vec<Value> = s
        .query_map([&path], |r| {
            Ok(json!({"name": r.get::<_, String>(0)?, "kind": r.get::<_, String>(1)?,
                "signature": r.get::<_, String>(2)?, "lines": [r.get::<_, i64>(3)?, r.get::<_, i64>(4)?]}))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(json!({"path": path, "symbols": symbols}))
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
        // Lower-tier guesses (graphs built with FOOTNOTE_CODE_GUESSES=1 only): listed apart, each with its rule.
        let guessed = edges_rule(&c, id, limit);
        paths.push(path.clone());
        paths.extend(
            callers
                .iter()
                .filter_map(|v| v["path"].as_str().map(str::to_string)),
        );
        let mut one = json!({"name": name, "kind": kind, "signature": signature, "path": path, "lines": [line, end],
            "references": references, "callers": callers, "callees": callees});
        if !guessed.is_empty() {
            one["guessed_callers"] = json!(guessed);
        }
        found.push(one);
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
    Ok(json!({"root": root, "origin": origin(&c)?, "symbols": found, "stale": stale}))
}

/// Guessed callers of one symbol (the `guesses` table; empty when the graph was built without guesses or before they existed).
fn edges_rule(c: &Connection, id: i64, limit: usize) -> Vec<Value> {
    let Ok(mut q) = c.prepare(
        "SELECT coalesce(s.name, ''), f.path, g.line, g.rule FROM guesses g JOIN files f ON f.id = g.file_id
         LEFT JOIN symbols s ON s.id = g.caller_id WHERE g.symbol_id = ?1 ORDER BY f.path, g.line LIMIT ?2",
    ) else {
        return vec![];
    };
    q.query_map(params![id, limit as i64], |r| {
        Ok(json!({"name": r.get::<_, String>(0)?, "path": r.get::<_, String>(1)?, "line": r.get::<_, i64>(2)?,
            "rule": r.get::<_, String>(3)?}))
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

/// Calls the parser could not resolve, for a file (its path) or a symbol (the item holding the calls): the call site and the
/// project definitions with the same name, none when the callee lives outside the project. Where an agent falls back to grep.
pub fn unresolved(db: &Path, target: &str, limit: usize) -> Result<Value> {
    let c = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
            format!(
                "no code graph at {}; run `footnote code-index` first",
                db.display()
            )
        })?;
    let path = target.replace('\\', "/");
    let mut q = c
        .prepare(
            "SELECT f.path, u.line, u.name, coalesce(s.name, ''), u.candidates FROM unresolved u JOIN files f ON f.id = u.file_id
             LEFT JOIN symbols s ON s.id = u.caller_id
             WHERE replace(f.path, '\\', '/') = ?1 OR s.name = ?2 ORDER BY f.path, u.line LIMIT ?3",
        )
        .context("this graph has no unresolved-call report; run `footnote code-index` again")?;
    let total: i64 = c.query_row(
        "SELECT count(*) FROM unresolved u JOIN files f ON f.id = u.file_id LEFT JOIN symbols s ON s.id = u.caller_id
         WHERE replace(f.path, '\\', '/') = ?1 OR s.name = ?2",
        params![path, target],
        |r| r.get(0),
    )?;
    let calls: Vec<Value> = q
        .query_map(params![path, target, limit as i64], |r| {
            let cands: String = r.get(4)?;
            Ok(json!({"path": r.get::<_, String>(0)?, "line": r.get::<_, i64>(1)?, "name": r.get::<_, String>(2)?,
                "in": r.get::<_, String>(3)?,
                "candidates": cands.split(',').filter(|x| !x.is_empty()).collect::<Vec<_>>()}))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(
        json!({"target": target, "origin": origin(&c)?, "total": total, "unresolved": calls,
        "note": "calls the graph could not link; `candidates` are project definitions with that name (grep them), none means the callee is outside the project"}),
    )
}

/// The impact radius of changing `name`: everything that calls it, then what calls those, up to `depth` levels, with the
/// call's line and how the link was made (`compiler` or `resolved`). Tests on the way (a caller in a test file or named
/// `test_*`) are listed apart, and the affected files and blocks are counted, so an agent sees what to re-check first.
pub fn impact(db: &Path, name: &str, depth: usize, limit: usize) -> Result<Value> {
    let c = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
            format!(
                "no code graph at {}; run `footnote code-index` first",
                db.display()
            )
        })?;
    let start: Vec<(i64, String, String)> = c
        .prepare(
            "SELECT s.id, s.kind, f.path FROM symbols s JOIN files f ON f.id = s.file_id
             WHERE s.name = ?1 AND s.kind != 'module' ORDER BY f.path, s.line LIMIT 10",
        )?
        .query_map([name], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut q = c.prepare(
        "SELECT r.caller_id, cs.name, cs.kind, cf.path, cs.line, min(r.line), r.origin FROM refs r
         JOIN symbols cs ON cs.id = r.caller_id JOIN files cf ON cf.id = cs.file_id
         WHERE r.symbol_id = ?1 AND r.caller_id != ?1 GROUP BY r.caller_id",
    )?;
    let mut seen: std::collections::HashSet<i64> = start.iter().map(|s| s.0).collect();
    let mut frontier: Vec<i64> = seen.iter().copied().collect();
    let mut callers = Vec::new();
    let mut graded = (0, 0);
    for level in 1..=depth.max(1) {
        let mut next = Vec::new();
        for id in &frontier {
            let rows = q.query_map([id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })?;
            for row in rows {
                let (caller, cname, kind, path, line, call_line, origin) = row?;
                if !seen.insert(caller) {
                    continue;
                }
                if origin == "compiler" {
                    graded.0 += 1
                } else {
                    graded.1 += 1
                }
                let file = path.replace('\\', "/");
                let test = file
                    .split('/')
                    .any(|p| p == "tests" || p == "test" || p == "__tests__")
                    || file.contains(".test.")
                    || file.contains("_test.")
                    || cname.starts_with("test_");
                callers.push((level, cname, kind, file, line, call_line, origin, test));
                next.push(caller);
            }
        }
        if next.is_empty() || callers.len() >= limit * 4 {
            break;
        }
        frontier = next;
    }
    let files: std::collections::BTreeSet<&str> = callers.iter().map(|c| c.3.as_str()).collect();
    let blocks: std::collections::BTreeSet<String> = files.iter().map(|f| block(f, 2)).collect();
    let row = |c: &(usize, String, String, String, i64, i64, String, bool)| json!({"level": c.0, "name": c.1, "kind": c.2, "path": c.3, "line": c.4, "call_line": c.5, "origin": c.6});
    Ok(json!({
        "symbol": name,
        "found": start.iter().map(|s| json!({"kind": s.1, "path": s.2.replace('\\', "/")})).collect::<Vec<_>>(),
        "depth": depth.max(1),
        "callers": callers.iter().filter(|c| !c.7).take(limit).map(row).collect::<Vec<_>>(),
        "tests": callers.iter().filter(|c| c.7).take(limit).map(row).collect::<Vec<_>>(),
        "counts": {"callers": callers.iter().filter(|c| !c.7).count(), "tests": callers.iter().filter(|c| c.7).count(),
                   "files": files.len(), "compiler_links": graded.0, "resolved_links": graded.1},
        "files": files,
        "blocks": blocks,
        "origin": origin(&c)?,
    }))
}

/// What an outside package is for, from a hand-written list across Rust, JavaScript and Python; `None` when not listed.
/// `server` and `mcp` mean the project is called in (it serves HTTP or MCP); the other kinds are what it calls out to.
/// shortcut: a fixed list; unknown packages stay unclassed. Grow it when real projects show common misses.
fn class(package: &str) -> Option<&'static str> {
    Some(match package {
        "rusqlite"
        | "sqlx"
        | "diesel"
        | "postgres"
        | "tokio-postgres"
        | "redis"
        | "mongodb"
        | "sled"
        | "rocksdb"
        | "duckdb"
        | "libsql"
        | "tantivy"
        | "qdrant-client"
        | "lancedb"
        | "sea-orm"
        | "pg"
        | "mysql2"
        | "better-sqlite3"
        | "sqlite3"
        | "@prisma/client"
        | "prisma"
        | "mongoose"
        | "ioredis"
        | "typeorm"
        | "drizzle-orm"
        | "knex"
        | "sqlalchemy"
        | "psycopg"
        | "psycopg2"
        | "pymongo"
        | "asyncpg"
        | "chromadb"
        | "pinecone"
        | "@supabase/supabase-js"
        | "firebase-admin" => "database",
        "axum" | "actix-web" | "warp" | "rocket" | "poem" | "tonic" | "express" | "fastify"
        | "koa" | "hono" | "@nestjs/core" | "next" | "fastapi" | "flask" | "django"
        | "starlette" | "aiohttp" => "server",
        "rmcp" | "mcp-sdk" | "@modelcontextprotocol/sdk" | "mcp" | "fastmcp" => "mcp",
        "reqwest" | "ureq" | "hyper" | "url" | "http" | "isahc" | "surf" | "tungstenite"
        | "tokio-tungstenite" | "iroh" | "axios" | "node-fetch" | "undici" | "got" | "ky"
        | "requests" | "httpx" | "urllib3" | "websockets" | "stripe" | "@aws-sdk/client-s3"
        | "boto3" | "googleapis" => "network",
        "async-openai" | "openai" | "anthropic" | "@anthropic-ai/sdk" | "ollama-rs" | "ollama"
        | "rig-core" | "genai" | "ort" | "llama-cpp-2" | "candle-core" | "tokenizers"
        | "mistralrs" | "langchain" | "@langchain/core" | "transformers" | "torch"
        | "onnxruntime" | "onnxruntime-node" | "ai" => "model",
        "windows-sys" | "windows" | "libc" | "nix" | "walkdir" | "notify" | "dirs" | "keyring"
        | "fs-extra" | "chokidar" | "watchdog" | "psutil" | "zstd" => "system",
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
        "root": root, "origin": origin(&c)?, "depth": depth,
        // An entry block is one no other block references: where a run, a request or a test starts.
        "blocks": blocks.iter().map(|(b, (f, s))| json!({"name": b, "files": f, "symbols": s,
            "entry": !links.iter().any(|((_, to), _)| to == b)})).collect::<Vec<_>>(),
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
        let m = "scip-typescript npm app HEAD src/`ipc.ts`/spawnSession0:";
        assert_eq!(short_name(m), "spawnSession");
        let l = "scip-typescript npm app HEAD src/`chart.tsx`/typeLiteral10:entry_date.";
        assert_eq!(short_name(l), "entry_date");
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
