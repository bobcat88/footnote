//! `footnote find <query>`: one search over every product that is in this build and switched on, for the launcher's search
//! screen (goal 13). Each product answers on its own, so one failing (a missing graph, an unreadable corpus) never hides the
//! others; a switched-off product says so instead of being searched.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const LIST: &str = "corpora.json";

/// The Library corpora this machine knows: every database `footnote index` built or `footnote install` registered.
pub fn corpora(state: &Path) -> Vec<PathBuf> {
    std::fs::read(state.join(LIST))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<PathBuf>>(&b).ok())
        .unwrap_or_default()
}

/// Add a corpus database to the list (absolute path, once). A failure here never fails the index or install that called it.
pub fn record_corpus(db: &Path) {
    let Ok(state) = crate::state::state_dir() else {
        return;
    };
    let db = std::path::absolute(db).unwrap_or_else(|_| db.to_path_buf());
    // shortcut: corpora in the system temp folder are not recorded (test runs and scratch indexes vanish there, and the test
    // suite would otherwise fill the user's list); someone who keeps a real corpus in %TEMP% would not see it in search.
    if db.starts_with(std::env::temp_dir()) {
        return;
    }
    let mut list = corpora(&state);
    if list.contains(&db) {
        return;
    }
    list.push(db);
    let tmp = state.join(format!(".{LIST}.{}.tmp", std::process::id()));
    let _ = crate::state::ensure_private_dir(&state)
        .and_then(|()| Ok(std::fs::write(&tmp, serde_json::to_vec_pretty(&list)?)?))
        .and_then(|()| Ok(std::fs::rename(&tmp, state.join(LIST))?));
}

fn on(state: &Path, product: &str) -> bool {
    footnote_core::config::product_settings(state, product)
        .map_or(true, |v| v["enabled"].as_bool().unwrap_or(true))
}

fn text(v: &Value, max: usize) -> String {
    v.as_str().unwrap_or_default().chars().take(max).collect()
}

pub fn find(query: &str, limit: usize) -> Result<Value> {
    let query = query.trim();
    ensure!(
        !query.is_empty() && query.chars().count() <= 500,
        "the query must hold 1 to 500 characters"
    );
    let state = crate::state::state_dir()?;
    Ok(
        json!({"query": query, "products": [memory(&state, query, limit), library(&state, query, limit), code(&state, query, limit)]}),
    )
}

/// One product's answer: its results, or why there are none (off, not in this build, nothing to search, an error).
fn answer(product: &str, run: impl FnOnce() -> Result<Value>) -> Value {
    match run() {
        Ok(v) => {
            let mut v = v;
            v["product"] = json!(product);
            v
        }
        Err(e) => {
            json!({"product": product, "state": "error", "error": format!("{e:#}"), "results": []})
        }
    }
}

#[cfg_attr(
    all(feature = "memory", feature = "library", feature = "code"),
    allow(dead_code)
)]
fn not_built(product: &str) -> Value {
    json!({"product": product, "state": "not_built", "results": []})
}

/// Search one Footnote database by words and, when the database has vectors from installed models, by meaning too (what
/// `footnote search` does by default).
#[cfg(any(feature = "memory", feature = "library"))]
fn search_db(db: &Path, query: &str, limit: usize) -> Result<Vec<Value>> {
    let c = crate::app::open(db, false)?;
    let filters = crate::records::Filters::default();
    let models = crate::app::search_models(&c, None, vec![])?;
    let hits = if models.is_empty() {
        crate::app::search(&c, query, limit, &filters)?
    } else {
        crate::semantic::search(&c, query, limit, &filters, &models)?
    };
    Ok(hits["results"].as_array().cloned().unwrap_or_default())
}

fn off(state: &Path, product: &str) -> Option<Value> {
    (!on(state, product)).then(|| json!({"product": product, "state": "off", "results": []}))
}

#[cfg(feature = "memory")]
fn memory(state: &Path, query: &str, limit: usize) -> Value {
    if let Some(v) = off(state, "memory") {
        return v;
    }
    answer("memory", || {
        let db = crate::status::memory_db(state)?;
        if !db.exists() {
            return Ok(json!({"state": "empty", "results": []}));
        }
        // The whole memory database, every project's scope, as the MCP recall tool searches it: someone searching from the
        // launcher wants all their memories, not only the global ones the hook adds to a project's prompt.
        let results: Vec<Value> = search_db(&db, query, limit)?.into_iter().map(|h| json!({
            "id": h["id"], "title": text(&h["text"], 300), "detail": text(&h["heading"], 120),
        })).collect();
        Ok(
            json!({"state": if results.is_empty() { "nothing_found" } else { "ok" }, "results": results}),
        )
    })
}

#[cfg(not(feature = "memory"))]
fn memory(_: &Path, _: &str, _: usize) -> Value {
    not_built("memory")
}

#[cfg(feature = "library")]
fn library(state: &Path, query: &str, limit: usize) -> Value {
    if let Some(v) = off(state, "library") {
        return v;
    }
    answer("library", || {
        // A corpus deleted since it was indexed is skipped and counted, not reported as an error.
        let (list, missing): (Vec<PathBuf>, Vec<PathBuf>) =
            corpora(state).into_iter().partition(|p| p.exists());
        if list.is_empty() {
            return Ok(json!({"state": "empty", "results": [], "missing": missing.len()}));
        }
        let (mut results, mut problems) = (Vec::new(), Vec::new());
        for db in &list {
            let found = search_db(db, query, limit);
            match found {
                Ok(hits) => results.extend(hits.into_iter().map(|h| json!({
                    "id": h["id"], "title": text(if h["heading"].is_string() { &h["heading"] } else { &h["path"] }, 200),
                    "detail": format!("{}:{}", text(&h["path"], 400), h["line"].as_i64().unwrap_or(0)),
                    "snippet": text(if h["text"].is_string() { &h["text"] } else { &h["snippet"] }, 300),
                    "source": db,
                }))),
                Err(e) => problems.push(json!({"source": db, "error": format!("{e:#}")})),
            }
        }
        let state = if !results.is_empty() {
            "ok"
        } else if problems.len() == list.len() {
            "error"
        } else {
            "nothing_found"
        };
        Ok(
            json!({"state": state, "results": results, "corpora": list.len(), "missing": missing.len(), "problems": problems}),
        )
    })
}

#[cfg(not(feature = "library"))]
fn library(_: &Path, _: &str, _: usize) -> Value {
    not_built("library")
}

#[cfg(feature = "code")]
fn code(state: &Path, query: &str, limit: usize) -> Value {
    if let Some(v) = off(state, "code") {
        return v;
    }
    answer("code", || {
        // Code looks symbols up by exact name, so a phrase cannot match one.
        if !query.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Ok(json!({"state": "not_a_symbol", "results": []}));
        }
        let graphs: Vec<PathBuf> = std::fs::read_dir(state.join("code"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "db"))
            .collect();
        if graphs.is_empty() {
            return Ok(json!({"state": "empty", "results": []}));
        }
        let mut results = Vec::new();
        for g in &graphs {
            // A graph that cannot be read is skipped: one stale project must not hide the others.
            let Ok(v) = footnote_code::lookup(g, query, limit) else {
                continue;
            };
            for s in v["symbols"].as_array().into_iter().flatten() {
                results.push(json!({
                    "title": format!("{} {}", text(&s["kind"], 20), text(&s["name"], 200)),
                    "detail": format!("{}:{}", text(&s["path"], 400), s["lines"][0].as_i64().unwrap_or(0)),
                    "snippet": text(&s["signature"], 300), "source": v["root"],
                    "references": s["references"],
                }));
            }
        }
        Ok(
            json!({"state": if results.is_empty() { "nothing_found" } else { "ok" }, "results": results}),
        )
    })
}

#[cfg(not(feature = "code"))]
fn code(_: &Path, _: &str, _: usize) -> Value {
    not_built("code")
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_corpus_list_tolerates_a_missing_or_broken_file() {
        let t = tempfile::tempdir().unwrap();
        assert!(super::corpora(t.path()).is_empty());
        std::fs::write(t.path().join(super::LIST), "not json").unwrap();
        assert!(super::corpora(t.path()).is_empty());
    }
}
