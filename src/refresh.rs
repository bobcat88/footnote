//! `footnote docs refresh [library]`: keep cached documentation fresh without crawling it again (#310).
//!
//! A crawled entry keeps its pages beside the database (`<version>.pages/`, with `refresh.json`: per page the fetched URL and
//! its validators). Refresh asks each page with a conditional request (ETag, Last-Modified, else a hash of the body), so an
//! unchanged site costs one small request per page and no re-indexing. A changed page is judged; a material one is rewritten and
//! the folder is indexed again (the indexer already skips unchanged files, drops removed ones and re-chunks changed ones), the
//! keyword index first and the embeddings afterwards through the usual `docs.embed` job, which embeds only chunks without a vector.
//! New pages come from the links of changed pages and the sitemap, up to the entry's original page cap.
//!
//! The judge: `ChangeJudge` decides "material or cosmetic" for a changed page; cosmetic leaves the indexed text and its
//! embeddings alone (the new validators are kept, so the page is not fetched in full again, and the next judgement still
//! compares against what is indexed, so small changes add up). The default, `TokenDiff`, is deterministic and offline. A
//! Haiku-class model can be plugged in by implementing `ChangeJudge` and passing it to `refresh`: it only answers true or false for
//! a pair of texts and never writes index content. It is not wired to any API key or flag, and `model_calls` in the report is 0.
//! Entries made before this existed, from a folder (`--from`) or installed from a bundle have no page data and are skipped.
use crate::docs::{self, Add};
use anyhow::{Result, bail};
use footnote_library::crawl::{self, Client, Recheck, Validators};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

/// Decides whether a changed page is worth re-indexing and re-embedding.
pub trait ChangeJudge {
    fn is_material(&self, old: &str, new: &str) -> bool;
}

/// Most changed words a cosmetic change may have (a typo fix, a renamed link), however long the page: "off" turning into "on by
/// default now" is 5 words of a long page and must not be skipped.
const TYPO_WORDS: i64 = 4;

/// Cosmetic when the share of changed words is below the threshold, at most `TYPO_WORDS` words changed, and no changed word holds a digit (a number, version or
/// limit that moved is never cosmetic). Words compare case- and punctuation-insensitively, so reflowed text is no change.
/// shortcut: word counts, not an ordered diff, so a pure reordering counts as cosmetic; use a real diff if that matters.
pub struct TokenDiff(pub f64);

impl Default for TokenDiff {
    fn default() -> Self {
        TokenDiff(0.01)
    }
}

fn counts(s: &str) -> HashMap<String, i64> {
    let mut m = HashMap::new();
    for w in s.split_whitespace() {
        let w = w
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if !w.is_empty() {
            *m.entry(w).or_insert(0) += 1;
        }
    }
    m
}

impl ChangeJudge for TokenDiff {
    fn is_material(&self, old: &str, new: &str) -> bool {
        let (a, b) = (counts(old), counts(new));
        let total: i64 = a.values().chain(b.values()).sum();
        let mut moved = 0;
        for w in a.keys().chain(b.keys()).collect::<HashSet<_>>() {
            let d = (a.get(w).unwrap_or(&0) - b.get(w).unwrap_or(&0)).abs();
            if d > 0 && w.chars().any(|c| c.is_ascii_digit()) {
                return true;
            }
            moved += d;
        }
        total > 0 && (moved > TYPO_WORDS || moved as f64 / total as f64 >= self.0)
    }
}

#[derive(Serialize, Deserialize)]
struct PageRec {
    url: String,
    validators: Validators,
}

#[derive(Serialize, Deserialize)]
struct Sidecar {
    scope: String,
    max_pages: usize,
    pages: BTreeMap<String, PageRec>,
    #[serde(default)]
    refreshed_unix: u64,
}

pub fn run(state: &Path, library: Option<&str>, judge: &dyn ChangeJudge, a: &Add) -> Result<Value> {
    let site = library.map(|l| docs::safe("library", l)).transpose()?;
    let wanted: Vec<_> = docs::entries(state)
        .into_iter()
        .filter(|e| site.as_ref().is_none_or(|s| &e.0 == s))
        .collect();
    if let Some(l) = library
        && wanted.is_empty()
    {
        bail!("{l} is not in the docs cache (see `footnote docs list`)");
    }
    let client = Client::default();
    let (mut rows, mut touched) = (Vec::new(), Vec::new());
    for (site, version, db) in wanted {
        match one(&client, judge, &db) {
            Ok(mut v) => {
                if v["changed"].as_u64().unwrap_or(0)
                    + v["added"].as_u64().unwrap_or(0)
                    + v["removed"].as_u64().unwrap_or(0)
                    > 0
                {
                    touched.push(db);
                }
                v["site"] = json!(site);
                v["version"] = json!(version);
                rows.push(v);
            }
            Err(e) => {
                rows.push(json!({"site": site, "version": version, "error": format!("{e:#}")}))
            }
        }
    }
    let embedding = docs::embed_missing(state, &touched, a)?;
    Ok(json!({"refreshed": rows, "embedding": embedding, "model_calls": 0}))
}

fn one(client: &Client, judge: &dyn ChangeJudge, db: &Path) -> Result<Value> {
    let dir = db.with_extension("pages");
    let Ok(bytes) = std::fs::read(dir.join("refresh.json")) else {
        return Ok(json!({
            "skipped": "no page data (added before refresh existed, from a folder, or installed from a bundle); run `docs add` for it again"
        }));
    };
    let mut side: Sidecar = serde_json::from_slice(&bytes)?;
    let (mut same, mut changed, mut cosmetic, mut removed, mut unreachable) = (0, 0, 0, 0, 0);
    let mut seeds = Vec::new();
    for file in side.pages.keys().cloned().collect::<Vec<_>>() {
        let rec = &side.pages[&file];
        match client.recheck(&rec.url, &rec.validators) {
            Recheck::Same(v) => {
                same += 1;
                side.pages.get_mut(&file).unwrap().validators = v;
            }
            Recheck::Changed(text, v, links) => {
                seeds.extend(links);
                let old = std::fs::read_to_string(dir.join(&file)).unwrap_or_default();
                side.pages.get_mut(&file).unwrap().validators = v;
                if judge.is_material(&old, &text) {
                    std::fs::write(dir.join(&file), text)?;
                    changed += 1;
                } else {
                    cosmetic += 1;
                }
            }
            Recheck::Gone => {
                let _ = std::fs::remove_file(dir.join(&file));
                side.pages.remove(&file);
                removed += 1;
            }
            Recheck::Unreachable => unreachable += 1,
        }
    }
    let known: HashSet<String> = side
        .pages
        .values()
        .filter_map(|p| crawl::key_of(&p.url))
        .collect();
    let room = side.max_pages.saturating_sub(side.pages.len());
    let found = if room > 0 {
        client.find_new(&side.scope, &known, seeds, room)
    } else {
        Vec::new()
    };
    let mut n = side
        .pages
        .keys()
        .filter_map(|f| f.strip_suffix(".md")?.parse::<usize>().ok())
        .max()
        .unwrap_or(0);
    let added = found.len();
    for f in found {
        n += 1;
        std::fs::write(dir.join(format!("{n}.md")), &f.text)?;
        side.pages.insert(
            format!("{n}.md"),
            PageRec {
                url: f.url.to_string(),
                validators: f.validators,
            },
        );
    }
    if changed + removed + added > 0 {
        let mut c = crate::app::open(db, true)?;
        footnote_library::index(
            &mut c,
            &dir,
            std::slice::from_ref(&dir),
            100_000,
            2_048,
            &mut |_, _| {},
        )?;
        let _ = crate::lexical::sync(&c); // a failure only means searches fall back to FTS5, as for `add`
    }
    side.refreshed_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    std::fs::write(dir.join("refresh.json"), serde_json::to_vec_pretty(&side)?)?;
    Ok(json!({
        "summary": format!("{same} unchanged, {changed} changed, {added} added, {removed} removed"),
        "unchanged": same, "changed": changed, "added": added, "removed": removed,
        "cosmetic_skipped": cosmetic, "unreachable": unreachable, "pages": side.pages.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_judge_skips_typos_and_reflow_but_not_numbers_or_new_text() {
        let base = "Connections are reused from the pool. ".repeat(30)
            + "The colour of the badge is blue.";
        let judge = TokenDiff::default();
        assert!(!judge.is_material(&base, &base.replace("colour", "color")));
        assert!(!judge.is_material(&base, &base.replace(". ", ".\n").to_uppercase()));
        assert!(judge.is_material(&base, &base.replace("blue", "blue 42")));
        assert!(judge.is_material(
            &base,
            &(base.clone() + " A whole new paragraph about timeouts and retries follows.")
        ));
    }
}
