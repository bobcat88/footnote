//! `footnote docs`: one documentation index per site and version on this machine, shared by every project.
//!
//! Layout: `<state>/docs/<site>/<version>.db`, each registered in the corpora list so Search finds it.
//! Old-version rule: a new version is added beside the old ones and nothing is deleted on its own; `docs prune` deletes,
//! per site, every version older than the newest (numbers compare as numbers, so 10.0 > 9.2; any non-numeric version such
//! as `latest` counts as newer than every numeric one).
use crate::find;
use anyhow::{Result, bail, ensure};
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

pub struct Add<'a> {
    pub url: Option<&'a str>,
    pub from: Option<&'a Path>,
    pub site: Option<&'a str>,
    pub version: Option<&'a str>,
    pub max_pages: usize,
}

pub fn add(state: &Path, a: &Add) -> Result<Value> {
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
            json!({"site": site, "version": version, "pages": pages, "bytes": meta.map(|m| m.len()), "modified_unix": date, "db": db})
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
            list.retain(|p| p != db);
            removed.push(json!({"site": site, "version": version}));
        }
    }
    find::write(state, &list)?;
    Ok(json!({"removed": removed}))
}
