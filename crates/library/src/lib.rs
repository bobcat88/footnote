//! Footnote Library: document corpora. Extraction and folder indexing (ADR 0001); documentation crawling comes later.
use anyhow::{Result, bail};
use footnote_core::{bounded_read, hash, is_link, no_links};
use footnote_search::store::{is_memory_db, text_chunks};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub mod crawl;
pub mod extract;
pub mod miss;

use footnote_core::{metrics, state};

/// The repository the binary was built from: the extraction helper lives in its `tools/`.
pub(crate) fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
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
pub fn index(
    c: &mut Connection,
    root: &Path,
    allow: &[PathBuf],
    max_files: usize,
    max_mib: u64,
    progress: &mut dyn FnMut(u64, u64),
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
    if is_memory_db(c)? {
        bail!("this is an agent memory database; index documents into a separate corpus database");
    }
    let root_s = root.to_string_lossy().to_string();
    let mut seen = std::collections::HashSet::new();
    let mut indexed = 0;
    let mut unchanged = 0;
    let mut failed = 0;
    let mut failures = Vec::new();
    let mut total_bytes = 0_u64;
    let extractor_hash = hash(&fs::read(repo().join("tools/extract.py"))?);
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    // Sorted: the same files get the same ids on every platform (NTFS lists by name, Linux file systems do not).
    for entry in WalkDir::new(&root)
        .sort_by_file_name()
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
        progress(seen.len() as u64, total_bytes);
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
            crate::metrics::timed!("index.extract", crate::extract::extract(&bytes, &ext))
                .unwrap_or_else(|_| crate::extract::Extraction {
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
