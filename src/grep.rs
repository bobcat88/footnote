//! `footnote grep` (#382): native ripgrep-engine search (`ignore` + `grep-searcher` + `grep-regex`).
//!
//! Features designed specifically for AI coding agents:
//! - `.gitignore`-aware parallel traversal with binary skipping and symlink/reparse-point rejection.
//! - Matches grouped by file (eliminating repeated file paths).
//! - Automatic enclosing-symbol annotation (`[fn foo L40-L82]`) when a Footnote Code graph exists.
//! - Hard inline output byte budget with reversible spillover to `footnote output <id>` (fail-closed on secrets).

use anyhow::{Context, Result, bail};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{BinaryDetection, SearcherBuilder, sinks::UTF8};
use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Mutex,
};

const MAX_INLINE_BYTES: usize = 10_000;
const MAX_LINE_CHARS: usize = 200;

pub struct Options<'a> {
    pub pattern: &'a str,
    pub path: &'a Path,
    pub glob: Option<&'a str>,
    pub ignore_case: bool,
    pub fixed_strings: bool,
    pub limit: usize,
    pub json: bool,
}

#[derive(Debug, Clone)]
struct MatchHit {
    file: String,
    line: u64,
    text: String,
    symbol: Option<String>,
}

fn trim_line(s: &str) -> String {
    let trimmed = s.trim_end_matches(['\r', '\n']).trim();
    if trimmed.chars().count() <= MAX_LINE_CHARS {
        trimmed.to_string()
    } else {
        let mut out: String = trimmed.chars().take(MAX_LINE_CHARS).collect();
        out.push('…');
        out
    }
}

#[cfg(feature = "code")]
fn format_symbol(spans: &[footnote_code::SymbolSpan], line: u64) -> Option<String> {
    let ln = line as i64;
    let best = spans
        .iter()
        .filter(|s| s.line <= ln && ln <= s.end_line)
        .min_by_key(|s| (s.end_line - s.line, s.line))?;
    let short_kind = match best.kind.as_str() {
        "function" | "method" => "fn",
        "struct" => "struct",
        "enum" => "enum",
        "trait" => "trait",
        "class" => "class",
        "interface" => "iface",
        other => other,
    };
    Some(format!(
        "{short_kind} {} L{}-L{}",
        best.name, best.line, best.end_line
    ))
}

fn format_grouped(hits: &[MatchHit], include_all: bool, limit: usize) -> (String, usize, bool) {
    let mut by_file: BTreeMap<&str, Vec<&MatchHit>> = BTreeMap::new();
    let cap = if include_all {
        hits.len()
    } else {
        limit.min(hits.len())
    };
    for h in &hits[..cap] {
        by_file.entry(&h.file).or_default().push(h);
    }
    let mut out = String::new();
    let mut shown = 0usize;
    let mut truncated = cap < hits.len();
    for (file, list) in by_file {
        let header = format!("{file} ({})\n", list.len());
        if !include_all && out.len() + header.len() > MAX_INLINE_BYTES && shown > 0 {
            truncated = true;
            break;
        }
        out.push_str(&header);
        for h in list {
            let line = match &h.symbol {
                Some(sym) => format!("  L{} [{}] {}\n", h.line, sym, h.text),
                None => format!("  L{} {}\n", h.line, h.text),
            };
            if !include_all && out.len() + line.len() > MAX_INLINE_BYTES && shown > 0 {
                return (out, shown, true);
            }
            out.push_str(&line);
            shown += 1;
        }
    }
    (out, shown, truncated)
}

pub fn run(opts: &Options<'_>) -> Result<Value> {
    if opts.pattern.is_empty() {
        bail!("pattern must not be empty");
    }
    if !opts.path.exists() {
        bail!("path not found: {}", opts.path.display());
    }
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(opts.ignore_case)
        .fixed_strings(opts.fixed_strings)
        .build(opts.pattern)
        .with_context(|| format!("invalid search pattern: {:?}", opts.pattern))?;

    #[cfg(feature = "code")]
    let root = if opts.path.is_file() {
        opts.path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        opts.path
    };

    let mut hits: Vec<MatchHit> = Vec::new();
    if opts.path.is_file() {
        if !footnote_core::is_link(opts.path) {
            let rel = opts
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| opts.path.to_string_lossy().replace('\\', "/"));
            let mut searcher = SearcherBuilder::new()
                .binary_detection(BinaryDetection::quit(b'\x00'))
                .line_number(true)
                .build();
            let _ = searcher.search_path(
                &matcher,
                opts.path,
                UTF8(|line_num, line| {
                    hits.push(MatchHit {
                        file: rel.clone(),
                        line: line_num,
                        text: trim_line(line),
                        symbol: None,
                    });
                    Ok(true)
                }),
            );
        }
    } else {
        let mut builder = WalkBuilder::new(opts.path);
        builder
            .follow_links(false)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .require_git(false)
            .hidden(true);

        if let Some(glob) = opts.glob {
            let mut ov = OverrideBuilder::new(opts.path);
            ov.add(glob)
                .with_context(|| format!("invalid --glob pattern: {glob:?}"))?;
            builder.overrides(ov.build()?);
        }

        struct Batch<'a> {
            out: &'a Mutex<Vec<MatchHit>>,
            local: Vec<MatchHit>,
        }
        impl Drop for Batch<'_> {
            fn drop(&mut self) {
                if !self.local.is_empty()
                    && let Ok(mut guard) = self.out.lock()
                {
                    guard.append(&mut self.local);
                }
            }
        }

        let collected: Mutex<Vec<MatchHit>> = Mutex::new(Vec::new());
        let matcher = &matcher;
        builder.build_parallel().run(|| {
            let mut batch = Batch {
                out: &collected,
                local: Vec::new(),
            };
            let mut searcher = SearcherBuilder::new()
                .binary_detection(BinaryDetection::quit(b'\x00'))
                .line_number(true)
                .build();
            Box::new(move |entry| {
                if let Ok(e) = entry
                    && e.file_type().is_some_and(|ft| ft.is_file())
                    && !footnote_core::is_link(e.path())
                {
                    let rel = e
                        .path()
                        .strip_prefix(opts.path)
                        .unwrap_or(e.path())
                        .to_string_lossy()
                        .replace('\\', "/");
                    let _ = searcher.search_path(
                        matcher,
                        e.path(),
                        UTF8(|line_num, line| {
                            batch.local.push(MatchHit {
                                file: rel.clone(),
                                line: line_num,
                                text: trim_line(line),
                                symbol: None,
                            });
                            Ok(true)
                        }),
                    );
                    if batch.local.len() >= 128
                        && let Ok(mut guard) = batch.out.lock()
                    {
                        guard.append(&mut batch.local);
                    }
                }
                WalkState::Continue
            })
        });
        hits = collected.into_inner().unwrap_or_default();
    }

    hits.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.line.cmp(&b.line)));
    let total_files: usize = hits
        .iter()
        .map(|h| h.file.as_str())
        .collect::<BTreeSet<_>>()
        .len();

    // Annotate matches with enclosing symbol + line range when a Footnote Code graph exists (#382).
    #[cfg(feature = "code")]
    let annotated = {
        let mut count = 0usize;
        if let Ok(db) = crate::app::code_db(root)
            && db.is_file()
        {
            let matched_files: Vec<String> = hits
                .iter()
                .map(|h| h.file.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let spans = footnote_code::enclosing_symbols(&db, &matched_files);
            for h in &mut hits {
                if let Some(file_spans) = spans.get(&h.file)
                    && let Some(sym) = format_symbol(file_spans, h.line)
                {
                    h.symbol = Some(sym);
                    count += 1;
                }
            }
        }
        count
    };
    #[cfg(not(feature = "code"))]
    let annotated = 0usize;

    let limit = opts.limit.max(1);
    let (mut inline_text, shown, truncated) = format_grouped(&hits, false, limit);
    let mut handle: Option<String> = None;
    let mut secret_redacted = false;

    if truncated {
        let (full_text, _, _) = format_grouped(&hits, true, hits.len());
        if crate::run::contains_secrets(&full_text) {
            secret_redacted = true;
            inline_text.push_str(&format!(
                "... truncated ({shown}/{} matches across {total_files} files; handle omitted: secret-like token detected)\n",
                hits.len()
            ));
        } else if let Ok(id) = crate::run::store_raw_output(&full_text) {
            inline_text.push_str(&format!(
                "... truncated ({shown}/{} matches across {total_files} files; full output saved to `{id}` — run `footnote output {id} --grep <pat>`)\n",
                hits.len()
            ));
            handle = Some(id);
        } else {
            inline_text.push_str(&format!(
                "... truncated ({shown}/{} matches across {total_files} files)\n",
                hits.len()
            ));
        }
    }

    if opts.json {
        let items: Vec<Value> = hits
            .iter()
            .take(shown)
            .map(|h| {
                json!({
                    "file": h.file,
                    "line": h.line,
                    "symbol": h.symbol,
                    "text": h.text,
                })
            })
            .collect();
        return Ok(json!({
            "pattern": opts.pattern,
            "total_matches": hits.len(),
            "total_files": total_files,
            "shown": shown,
            "annotated_symbols": annotated,
            "truncated": truncated,
            "handle": handle,
            "secret_redacted": secret_redacted,
            "matches": items,
        }));
    }

    if hits.is_empty() {
        println!("no matches for {:?}", opts.pattern);
    } else {
        print!("{inline_text}");
    }
    Ok(Value::Null)
}
