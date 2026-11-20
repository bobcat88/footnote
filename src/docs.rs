//! `footnote docs`: one documentation index per site and version on this machine, shared by every project.
//!
//! A corpus is searchable once its keyword index is built. Embeddings are computed afterwards by a detached `docs embed`
//! worker (one `docs.embed` job per library, smallest first) and search upgrades to hybrid by itself when the vectors are
//! committed. Embedding is one transaction per library: a crash leaves the keyword corpus intact, and re-running `docs add`
//! starts the embedding again (shortcut: nothing is kept from the killed run, split into commits when a library is huge).
//! Layout: `<state>/docs/<site>/<version>.db`, each registered in the corpora list so Search finds it.
//! Old-version rule: a new version is added beside the old ones and nothing is deleted on its own; `docs prune` deletes,
//! per site, every version older than the newest (numbers compare as numbers, so 10.0 > 9.2; any non-numeric version such
//! as `latest` counts as newer than every numeric one).
use crate::find;
use anyhow::{Context, Result, bail, ensure};
use footnote_core::jobs::{self, Job};
use footnote_library::{crawl, index};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// A path component made of `[a-z0-9._-]` only and never starting with a dot, so it cannot climb out of the docs folder.
fn safe(what: &str, raw: &str) -> Result<String> {
    let s: String = raw
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_start_matches('.').to_string();
    ensure!(!s.is_empty(), "{what} {raw:?} has no usable characters");
    Ok(s)
}

/// Numeric parts sort before text parts, each by value.
fn parts(v: &str) -> Vec<(bool, u64, String)> {
    v.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(|p| match p.parse::<u64>() {
            Ok(n) => (false, n, String::new()),
            Err(_) => (true, 0, p.to_string()),
        })
        .collect()
}

fn compare(a: &str, b: &str) -> Ordering {
    parts(a).cmp(&parts(b))
}

fn root(state: &Path) -> PathBuf {
    state.join("docs")
}

/// Every `(site, version, db)` in the cache, oldest version first within a site.
fn entries(state: &Path) -> Vec<(String, String, PathBuf)> {
    let mut out: Vec<(String, String, PathBuf)> = Vec::new();
    for site in std::fs::read_dir(root(state))
        .into_iter()
        .flatten()
        .flatten()
    {
        for f in std::fs::read_dir(site.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let p = f.path();
            if let Some(v) = p
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".db"))
            {
                out.push((site.file_name().to_string_lossy().into(), v.to_string(), p));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| compare(&a.1, &b.1)));
    out
}

/// The cache entry for a library (site) and an optional version, else its newest.
pub fn find_entry(
    state: &Path,
    library: &str,
    version: Option<&str>,
) -> Result<(String, String, PathBuf)> {
    let site = safe("library", library)?;
    let version = version.map(|v| safe("version", v)).transpose()?;
    entries(state)
        .into_iter()
        .rfind(|e| e.0 == site && version.as_ref().is_none_or(|v| &e.1 == v))
        .with_context(|| format!("{library} is not in the docs cache (see `footnote docs list`)"))
}

pub struct Add<'a> {
    pub url: Option<&'a str>,
    pub from: Option<&'a Path>,
    pub site: Option<&'a str>,
    pub version: Option<&'a str>,
    pub max_pages: usize,
    /// Embedding model folders; empty means the models Footnote Memory uses.
    pub models: &'a [PathBuf],
    /// Keyword index only, no background embedding.
    pub no_embed: bool,
}

/// Keyword index first, then the background embedding when the corpus lacks vectors.
pub fn add(state: &Path, a: &Add) -> Result<Value> {
    let mut v = add_keyword(state, a)?;
    let db = PathBuf::from(v["db"].as_str().unwrap_or_default());
    v["state"] = json!(status(&db, &jobs_now(state)).0);
    v["embedding"] = embed_missing(state, &[db], a)?;
    Ok(v)
}

/// `docs ingest <url>...`: each library's keyword index in turn, then one worker embedding them, largest last. A library
/// that fails does not stop the others.
pub fn ingest(state: &Path, urls: &[String], a: &Add) -> Result<Value> {
    let (mut dbs, mut libraries) = (Vec::new(), Vec::new());
    for u in urls {
        let one = Add {
            url: Some(u),
            from: None,
            site: None,
            version: None,
            max_pages: a.max_pages,
            models: a.models,
            no_embed: a.no_embed,
        };
        match add_keyword(state, &one) {
            Ok(mut v) => {
                let db = PathBuf::from(v["db"].as_str().unwrap_or_default());
                v["state"] = json!(status(&db, &jobs_now(state)).0);
                dbs.push(db);
                libraries.push(v);
            }
            Err(e) => libraries.push(json!({"url": u, "error": format!("{e:#}")})),
        }
    }
    Ok(json!({"libraries": libraries, "embedding": embed_missing(state, &dbs, a)?}))
}

fn jobs_now(state: &Path) -> Vec<Value> {
    jobs::list(state)
        .ok()
        .and_then(|v| v["jobs"].as_array().cloned())
        .unwrap_or_default()
}

/// "embedding" with `(done, total)` while a live `docs.embed` job holds the database, "ready" once every chunk has a vector,
/// otherwise "keyword-only" (a corpus whose worker died is keyword-only again; its job shows as interrupted).
fn status(db: &Path, jobs: &[Value]) -> (&'static str, Option<(u64, u64)>) {
    let label = db.to_string_lossy();
    if let Some(j) = jobs.iter().find(|j| {
        j["kind"] == "docs.embed" && j["state"] == "running" && j["label"] == label.as_ref()
    }) {
        let n = |k: &str| j[k].as_u64().unwrap_or(0);
        return ("embedding", Some((n("done"), n("total"))));
    }
    let Ok(c) = crate::app::open(db, false) else {
        return ("keyword-only", None);
    };
    let n = |sql: &str| c.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0);
    let chunks = n(
        "SELECT count(*) FROM chunks c JOIN sources s ON s.id=c.source_id WHERE s.status='indexed'",
    );
    let embedded = n(
        "SELECT coalesce(max(n),0) FROM (SELECT count(*) n FROM embeddings WHERE kind='chunk' GROUP BY model)",
    );
    (
        if embedded >= chunks {
            "ready"
        } else {
            "keyword-only"
        },
        None,
    )
}

fn chunk_count(db: &Path) -> i64 {
    crate::app::open(db, false)
        .ok()
        .and_then(|c| {
            c.query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
                .ok()
        })
        .unwrap_or(0)
}

/// Start one detached worker for the keyword-only corpora among `dbs`, smallest first so a big one never holds up the rest.
fn embed_missing(state: &Path, dbs: &[PathBuf], a: &Add) -> Result<Value> {
    if a.no_embed {
        return Ok(json!("skipped (--no-embed)"));
    }
    let jobs = jobs_now(state);
    let mut todo: Vec<(i64, &PathBuf)> = dbs
        .iter()
        .filter(|d| status(d, &jobs).0 == "keyword-only")
        .map(|d| (chunk_count(d), d))
        .collect();
    if todo.is_empty() {
        return Ok(json!("nothing to embed"));
    }
    let models: Vec<PathBuf> = if a.models.is_empty() {
        crate::models::in_use(state)?
            .into_iter()
            .map(PathBuf::from)
            .collect()
    } else {
        a.models.to_vec()
    };
    if models.is_empty() {
        return Ok(json!(
            "no embedding models selected: keyword search only (`footnote models install`, `footnote models use`)"
        ));
    }
    todo.sort();
    let mut cmd = std::process::Command::new(std::env::current_exe()?);
    cmd.args(["docs", "embed"]);
    for m in &models {
        cmd.arg("--model").arg(m);
    }
    cmd.args(todo.iter().map(|(_, d)| d.as_os_str()));
    #[cfg(feature = "memory")]
    crate::hook::spawn_background(&mut cmd)?;
    #[cfg(not(feature = "memory"))]
    {
        use std::process::Stdio;
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }
    Ok(
        json!({"started": todo.len(), "note": "search answers by keywords now and upgrades to hybrid as each library finishes; progress in `footnote jobs`"}),
    )
}

/// The detached worker: one `docs.embed` job per database, all registered up front (so they show as queued and a second
/// `add` does not start them twice), then run in order.
pub fn embed(state: &Path, dbs: &[PathBuf], models: &[PathBuf]) -> Result<Value> {
    let mut queue = Vec::new();
    for db in dbs {
        queue.push((
            db,
            Job::start(state, "docs.embed", &db.to_string_lossy(), "chunks")?,
        ));
    }
    let mut done = Vec::new();
    for (db, mut job) in queue {
        let mut total = 0;
        let r = (|| {
            let mut c = crate::app::open(db, true)?;
            footnote_search::semantic::rebuild_with(&mut c, models, &mut |d, t| {
                total = t as u64;
                job.progress(d as u64, Some(total))
            })
        })();
        if r.is_ok() {
            job.progress(total, Some(total)); // the throttled writes may stop short of the end
        }
        done.push(json!({"db": db, "ok": r.is_ok()}));
        job.finish(&r);
    }
    Ok(json!({"embedded": done}))
}

fn add_keyword(state: &Path, a: &Add) -> Result<Value> {
    ensure!(
        a.url.is_some() != a.from.is_some(),
        "give a start URL or --from <folder>, not both"
    );
    let site = match (a.site, a.url) {
        (Some(s), _) => s.to_string(),
        (None, Some(u)) => url::Url::parse(u)?
            .host_str()
            .unwrap_or_default()
            .to_string(),
        (None, None) => bail!("give --site when indexing a folder with --from"),
    };
    let site = safe("site", &site)?;
    let version = safe("version", a.version.unwrap_or("latest"))?;
    let db = root(state).join(&site).join(format!("{version}.db"));
    if db.is_file() {
        find::register(state, &db)?;
        return Ok(json!({"site": site, "version": version, "db": db, "reused": true}));
    }
    std::fs::create_dir_all(db.parent().unwrap())?;
    // Build beside the final name and rename at the end, so a failed run never leaves a half database that counts as cached.
    let part = db.with_extension("db.part");
    let _ = std::fs::remove_file(&part);
    let pages = root(state).join(format!(".pages-{}", std::process::id()));
    let folder = match (a.url, a.from) {
        (Some(u), _) => {
            let options = crawl::Options {
                start: u,
                max_pages: a.max_pages,
                exclude: None,
                concurrency: 2,
            };
            crawl::crawl(&pages, &options, &mut |_, _| {})?;
            pages.clone()
        }
        (_, f) => f.unwrap().to_path_buf(),
    };
    let built = (|| {
        let mut c = crate::app::open(&part, true)?;
        let report = index(
            &mut c,
            &folder,
            std::slice::from_ref(&folder),
            100_000,
            2_048,
            &mut |_, _| {},
        )?;
        // A failure only means searches fall back to FTS5, as for `index`.
        let _ = crate::lexical::sync(&c);
        Ok::<_, anyhow::Error>(report)
    })();
    let _ = std::fs::remove_dir_all(&pages);
    // An unreachable site or empty folder must not be cached as an empty corpus that later reads as ready.
    let built = built.and_then(|r| {
        ensure!(
            r["indexed"].as_u64().unwrap_or(0) > 0,
            "no pages found to index"
        );
        Ok(r)
    });
    let report = match built {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
    };
    std::fs::rename(&part, &db)?;
    find::register(state, &db)?;
    Ok(json!({"site": site, "version": version, "db": db, "reused": false, "index": report}))
}

pub fn list(state: &Path) -> Result<Value> {
    let jobs = jobs_now(state);
    let rows: Vec<Value> = entries(state)
        .into_iter()
        .map(|(site, version, db)| {
            let pages: Option<i64> = crate::app::open(&db, false).ok().and_then(|c| {
                c.query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
                    .ok()
            });
            let meta = std::fs::metadata(&db).ok();
            let date = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            let state_text = match status(&db, &jobs) {
                (s, Some((d, t))) => format!("{s} {d}/{t}"),
                (s, None) => s.to_string(),
            };
            json!({"site": site, "version": version, "state": state_text, "pages": pages, "bytes": meta.map(|m| m.len()), "modified_unix": date, "db": db})
        })
        .collect();
    Ok(json!({"docs": rows}))
}

/// Delete every version older than the newest of its site, and drop it from the corpora list.
pub fn prune(state: &Path) -> Result<Value> {
    let all = entries(state);
    let mut removed = Vec::new();
    let mut list = find::corpora(state);
    for (i, (site, version, db)) in all.iter().enumerate() {
        if all.get(i + 1).is_some_and(|n| &n.0 == site) {
            for ext in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{ext}", db.display()));
            }
            let _ = std::fs::remove_file(db.with_extension("licence.json"));
            list.retain(|p| p != db);
            removed.push(json!({"site": site, "version": version}));
        }
    }
    find::write(state, &list)?;
    Ok(json!({"removed": removed}))
}
