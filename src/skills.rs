//! Skill suggestions (#280): one small keyword index per theme of skill names and descriptions, searched on each prompt when
//! `memory.skills` is on. Layout: `<state>/skills/<theme>.db`, its documents in `<state>/skills/<theme>/<name>.md` and the
//! SKILL.md each came from in `<state>/skills/<theme>.json`. Indexing a theme switches it on; removing it switches it off.
//!
//! Rule from bench/experiments/SKILL-RANKING-2026-11-21.md: the top hit is suggested only when it holds at least 40% of the
//! prompt's topic words. Keyword ranking with that rule matched ranking by meaning on theme-sized sets, so no model is loaded.
use anyhow::{Context, Result, ensure};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};

/// Share of the prompt's topic words the top hit must hold.
const COVERAGE: f64 = 0.4;
/// Past this size plain coding prompts start getting skills they do not need (the eval's 238-skill row).
const MAX_SKILLS: usize = 100;
/// Skills suggested per prompt, across all themes.
pub const MAX_SUGGESTED: usize = 2;

fn dir(state: &Path) -> PathBuf {
    state.join("skills")
}

fn check_theme(theme: &str) -> Result<()> {
    ensure!(
        !theme.is_empty()
            && theme.len() <= 40
            && theme
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
        "theme names use a-z, 0-9 and '-' (at most 40 characters)"
    );
    Ok(())
}

/// `name` and `description` of a SKILL.md front matter. A description may continue on indented lines or be a YAML block
/// (`>-`, `|`).
fn front_matter(text: &str) -> Option<(String, String)> {
    let body = text.strip_prefix("---")?.trim_start_matches(['\r', '\n']);
    let front = &body[..body.find("\n---")?];
    let (mut name, mut desc, mut in_desc) = (None, String::new(), false);
    for line in front.lines() {
        let key = !line.starts_with([' ', '\t']) && line.contains(':');
        if key {
            in_desc = false;
            let (k, v) = line.split_once(':')?;
            match k.trim() {
                "name" => name = Some(unquote(v)),
                "description" => {
                    in_desc = true;
                    desc = v.trim().trim_start_matches(['>', '|', '-']).to_string();
                }
                _ => {}
            }
        } else if in_desc {
            desc.push(' ');
            desc.push_str(line.trim());
        }
    }
    let desc = unquote(&desc.split_whitespace().collect::<Vec<_>>().join(" "));
    let name = name.filter(|n| !n.is_empty())?;
    (!desc.is_empty()).then_some((name, desc))
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches(['"', '\'']).to_string()
}

/// A file name for a skill name (plugin names carry ':').
fn file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// `footnote skills index <theme> <folders>`: every SKILL.md under the folders, the first of each name kept.
pub fn index(state: &Path, theme: &str, folders: &[PathBuf]) -> Result<Value> {
    check_theme(theme)?;
    let mut found: Map<String, Value> = Map::new();
    let mut skipped = 0;
    for folder in folders {
        ensure!(folder.is_dir(), "{} is not a folder", folder.display());
        for e in walkdir::WalkDir::new(folder)
            .into_iter()
            .filter_map(Result::ok)
        {
            if e.file_name() != "SKILL.md" {
                continue;
            }
            let Some((name, desc)) = fs::read_to_string(e.path())
                .ok()
                .as_deref()
                .and_then(front_matter)
            else {
                skipped += 1;
                continue;
            };
            let stem = file_stem(&name);
            if !found.contains_key(&stem) {
                found.insert(
                    stem,
                    json!({"name": name, "path": e.path(), "description": desc}),
                );
            }
        }
    }
    ensure!(
        !found.is_empty(),
        "no SKILL.md with a name and a description under those folders"
    );
    ensure!(
        found.len() <= MAX_SKILLS,
        "{} skills: a theme holds at most {MAX_SKILLS}, split them into themes",
        found.len()
    );
    let root = dir(state);
    let docs = root.join(theme);
    remove(state, theme)?;
    fs::create_dir_all(&docs)?;
    for (stem, s) in &found {
        let (name, desc) = (
            s["name"].as_str().unwrap_or_default(),
            s["description"].as_str().unwrap_or_default(),
        );
        fs::write(
            docs.join(format!("{stem}.md")),
            format!("# {name}\n\n{desc}\n"),
        )?;
    }
    let db = root.join(format!("{theme}.db"));
    let mut c = crate::app::open(&db, true)?;
    footnote_library::index(
        &mut c,
        &docs,
        std::slice::from_ref(&docs),
        MAX_SKILLS,
        16,
        &mut |_, _| {},
    )?;
    crate::lexical::sync(&c)?;
    fs::write(
        root.join(format!("{theme}.json")),
        serde_json::to_vec_pretty(&found)?,
    )?;
    Ok(json!({"theme": theme, "skills": found.len(), "skipped": skipped, "db": db}))
}

/// `footnote skills list`: the themes switched on and their size.
pub fn list(state: &Path) -> Result<Value> {
    let mut themes = vec![];
    for (theme, map) in themes_on(state) {
        themes.push(json!({"theme": theme, "skills": map.len()}));
    }
    Ok(json!({"themes": themes}))
}

/// `footnote skills remove <theme>`: deletes the theme's index and documents (never the skills themselves).
pub fn remove(state: &Path, theme: &str) -> Result<Value> {
    check_theme(theme)?;
    let root = dir(state);
    let mut removed = false;
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(json!({"removed": false}));
    };
    for e in entries.filter_map(Result::ok) {
        let n = e.file_name().to_string_lossy().into_owned();
        let ours =
            n == theme || n == format!("{theme}.json") || n.starts_with(&format!("{theme}.db"));
        if ours {
            let p = e.path();
            if p.is_dir() {
                fs::remove_dir_all(&p)
            } else {
                fs::remove_file(&p)
            }
            .with_context(|| format!("removing {}", p.display()))?;
            removed = true;
        }
    }
    Ok(json!({"theme": theme, "removed": removed}))
}

/// Themes with both an index and a source map, sorted by name.
fn themes_on(state: &Path) -> Vec<(String, Map<String, Value>)> {
    let root = dir(state);
    let mut out: Vec<_> = fs::read_dir(&root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let theme = e
                .file_name()
                .to_string_lossy()
                .strip_suffix(".json")?
                .to_string();
            root.join(format!("{theme}.db")).is_file().then_some(())?;
            let map = serde_json::from_slice(&fs::read(e.path()).ok()?).ok()?;
            Some((theme, map))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Share of the query's topic words found in `text` (the fetch-on-miss term rule).
fn coverage(query: &str, text: &str) -> f64 {
    let want = footnote_library::miss::terms(query);
    if want.is_empty() {
        return 0.0;
    }
    let text = text.to_lowercase();
    want.iter().filter(|t| text.contains(t.as_str())).count() as f64 / want.len() as f64
}

/// Skills for a prompt: each theme's top hit that passes the rule, best coverage first, at most `MAX_SUGGESTED`, minus `seen`.
/// Each is `{name, path, line}`. An unreadable theme is skipped: a suggestion must never block the prompt.
pub fn suggest(state: &Path, query: &str, seen: &[String]) -> Vec<Value> {
    let mut picks: Vec<(f64, Value)> = vec![];
    for (theme, map) in themes_on(state) {
        let db = dir(state).join(format!("{theme}.db"));
        let Ok(hits) = crate::app::open(&db, false)
            .and_then(|c| crate::app::search(&c, query, 1, &crate::records::Filters::default()))
        else {
            continue;
        };
        let Some(top) = hits["results"].get(0) else {
            continue;
        };
        let text = top["text"].as_str().unwrap_or_default();
        let c = coverage(query, text);
        let stem = Path::new(top["path"].as_str().unwrap_or_default())
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(skill) = map.get(&stem) else {
            continue;
        };
        let name = skill["name"].as_str().unwrap_or_default().to_string();
        if c >= COVERAGE && !seen.contains(&name) && !picks.iter().any(|(_, p)| p["name"] == name) {
            let line: String = skill["description"]
                .as_str()
                .unwrap_or_default()
                .chars()
                .take(160)
                .collect();
            picks.push((
                c,
                json!({"name": name, "path": skill["path"], "line": line, "theme": theme}),
            ));
        }
    }
    picks.sort_by(|a, b| b.0.total_cmp(&a.0));
    picks
        .into_iter()
        .take(MAX_SUGGESTED)
        .map(|(_, v)| v)
        .collect()
}

/// The hook's lines for the suggested skills; empty when none.
pub fn render(skills: &[Value]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut out = String::from("Skills that may fit this prompt (load one only if it does):");
    for s in skills {
        out.push_str(&format!(
            "\n- {} ({}): {}",
            s["name"].as_str().unwrap_or_default(),
            s["path"].as_str().unwrap_or_default(),
            s["line"].as_str().unwrap_or_default()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_reads_folded_and_quoted_descriptions() {
        let t = "---\nname: css-transitions\ndescription: >-\n  Add CSS transitions\n  to dropdowns.\nlicense: MIT\n---\nbody";
        assert_eq!(
            front_matter(t),
            Some((
                "css-transitions".into(),
                "Add CSS transitions to dropdowns.".into()
            ))
        );
        let t = "---\r\nname: \"sql\"\r\ndescription: 'Write SQL queries.'\r\n---\r\n";
        assert_eq!(
            front_matter(t),
            Some(("sql".into(), "Write SQL queries.".into()))
        );
        assert_eq!(front_matter("---\nname: x\n---\n"), None);
        assert_eq!(front_matter("no front matter"), None);
    }

    #[test]
    fn coverage_counts_topic_words() {
        assert_eq!(
            coverage(
                "add a dropdown transition",
                "Add CSS transitions to a dropdown"
            ),
            1.0
        );
        assert_eq!(coverage("fix the login bug", "CSS transitions"), 0.0);
    }
}
