//! Fetch on miss (#301): a docs search whose results are weak fetches the pages most likely to answer it, live, into the cache
//! entry, and is searched again. The weak rule is `footnote_library::miss::is_weak`; the page choice and the politeness and scope
//! rules are `crawl::Client::fetch_on_miss`. Keyword-first: the new pages are indexed for keywords at once and get their
//! embeddings with the next `docs refresh` or `docs embed`.
use crate::refresh::{self, Sidecar};
use footnote_library::crawl::Client;
use serde_json::Value;
use std::path::Path;

/// Most pages fetched for one query, across all docs entries.
pub const CAP: usize = 3;

/// The text a hit is judged on.
fn hit_text(h: &Value) -> String {
    ["heading", "path", "text", "snippet"]
        .iter()
        .filter_map(|k| h[k].as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn weak(query: &str, hits: &[Value]) -> bool {
    let texts: Vec<String> = hits.iter().map(hit_text).collect();
    footnote_library::miss::is_weak(query, texts.iter().map(String::as_str))
}

/// Fetch up to `room` pages for `query` into the docs entry `db` and index them; the URLs fetched. Empty for a database that is
/// not a crawled docs entry, when nothing was found, and on any error (a failed live fetch must never fail the search).
pub fn rescue(db: &Path, query: &str, room: usize) -> Vec<String> {
    let dir = db.with_extension("pages");
    let Ok(bytes) = std::fs::read(dir.join("refresh.json")) else {
        return Vec::new();
    };
    let Ok(mut side) = serde_json::from_slice::<Sidecar>(&bytes) else {
        return Vec::new();
    };
    let found = Client::default().fetch_on_miss(&side.scope, query, &side.known(), room);
    let urls: Vec<String> = found.iter().map(|f| f.url.to_string()).collect();
    let stored = side.add(&dir, found).is_ok()
        && refresh::reindex(db, &dir).is_ok()
        && std::fs::write(
            dir.join("refresh.json"),
            serde_json::to_vec_pretty(&side).unwrap_or_default(),
        )
        .is_ok();
    if stored { urls } else { Vec::new() }
}
