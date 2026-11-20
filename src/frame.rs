//! `footnote frame` (#311): how ready a new project is to start building. A readiness score from 0 to 100 is the sum of
//! weighted, named checks; each says why it matters and the one action that fixes it. Open questions (goal, users,
//! constraints, how to run tests) are answered with `--set key=value`, stored in project memory as facts and counted on the
//! next run. Local and deterministic: it reads files, the docs cache and project memory; no network, no model.
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

/// Verdict bands: `ready` from 80, `almost` from 50, else `not yet`.
const READY: u32 = 80;
const ALMOST: u32 = 50;

/// `(key, question)`; the answer to each is stored as a memory fact.
const QUESTIONS: &[(&str, &str)] = &[
    ("goal", "What are you building, in one sentence?"),
    ("users", "Who uses it?"),
    (
        "constraints",
        "Any constraints: deadline, platform, performance, licence, things it must not do?",
    ),
    ("tests", "How do you run the tests?"),
];

/// Answers parsed from `key=value`; unknown keys and empty values are errors, not silently dropped.
fn parse(set: &[String]) -> Result<Vec<(&'static str, String)>> {
    set.iter()
        .map(|s| {
            let (k, v) = s.split_once('=').unwrap_or((s, ""));
            let Some((key, _)) = QUESTIONS.iter().find(|(q, _)| *q == k.trim()) else {
                bail!("--set takes goal, users, constraints or tests as key=value, not {s:?}");
            };
            let v = v.trim();
            if v.is_empty() || v.len() > 500 {
                bail!("--set {key}=... needs an answer of 1 to 500 characters");
            }
            Ok((*key, v.to_string()))
        })
        .collect()
}

fn text_of(root: &Path, names: &[&str]) -> Option<String> {
    let e = std::fs::read_dir(root).ok()?.flatten().find(|e| {
        let n = e.file_name().to_string_lossy().to_lowercase();
        e.path().is_file()
            && names
                .iter()
                .any(|w| n == *w || n.starts_with(&format!("{w}.")))
    })?;
    std::fs::read_to_string(e.path()).ok()
}

/// A test command found in the project's own files, else None.
fn test_command(root: &Path) -> Option<&'static str> {
    let has = |f: &str| root.join(f).is_file();
    let mentions =
        |f: &str, w: &str| std::fs::read_to_string(root.join(f)).is_ok_and(|t| t.contains(w));
    let py_tests = std::fs::read_dir(root.join("tests"))
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("test_") && n.ends_with(".py")
        });
    if has("Cargo.toml") {
        Some("cargo test")
    } else if mentions("package.json", "\"test\"") {
        Some("npm test")
    } else if has("go.mod") {
        Some("go test ./...")
    } else if has("pytest.ini")
        || has("tox.ini")
        || mentions("pyproject.toml", "pytest")
        || py_tests
    {
        Some("pytest")
    } else if std::fs::read_to_string(root.join("Makefile"))
        .is_ok_and(|t| t.lines().any(|l| l.starts_with("test:")))
    {
        Some("make test")
    } else {
        None
    }
}

/// How many of the libraries have a cached docs entry (the site name is, or contains, the library name).
fn cached(libs: &[String]) -> usize {
    #[cfg(feature = "library")]
    if let Ok(state) = crate::state::state_dir() {
        let sites: Vec<String> = crate::docs::entries(&state)
            .into_iter()
            .map(|e| e.0)
            .collect();
        // shortcut: substring match on the site name; exact library-to-site mapping would need the docs index to store it.
        return libs
            .iter()
            .filter(|l| {
                let l = l.to_lowercase();
                sites.iter().any(|s| s == &l || s.contains(&l))
            })
            .count();
    }
    let _ = libs;
    0
}

#[cfg(feature = "memory")]
fn head(root: &str, key: &str) -> String {
    format!("Frame answer {key} of {root}: ")
}

/// The stored answers of this folder as `key -> (memory ids, value)`.
#[cfg(feature = "memory")]
fn stored(root: &str) -> Result<BTreeMap<&'static str, (Vec<i64>, String)>> {
    let mut out = BTreeMap::new();
    let db = crate::status::memory_db(&crate::state::state_dir()?)?;
    if !db.exists() {
        return Ok(out);
    }
    let c = crate::app::open(&db, false)?;
    let scope = crate::hook::project_of(root);
    for (key, _) in QUESTIONS {
        let h = head(root, key);
        let mut q = c.prepare(
            "SELECT s.id, ch.text FROM memories m JOIN sources s ON s.id=m.source_id JOIN chunks ch ON ch.source_id=s.id \
             WHERE m.scope=?1 AND s.status='indexed' AND substr(ch.text,1,length(?2))=?2 ORDER BY s.id",
        )?;
        let rows = q.query_map(rusqlite::params![scope, h], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, text) = row?;
            let e = out.entry(*key).or_insert((vec![], String::new()));
            e.0.push(id);
            e.1 = text[h.len()..].to_string();
        }
    }
    Ok(out)
}

/// `footnote frame`. `set`: answers to store (the CLI); the MCP tool passes none and only reads.
pub fn run(root: &Path, set: &[String], _record: bool) -> Result<Value> {
    let root = std::path::absolute(root)?;
    if !root.is_dir() {
        bail!("{} is not a folder", root.display());
    }
    let root_s = root.to_string_lossy().to_string();
    let answers = parse(set)?;
    #[cfg(feature = "memory")]
    let mut known = stored(&root_s)?;
    #[cfg(not(feature = "memory"))]
    let mut known: BTreeMap<&'static str, (Vec<i64>, String)> = BTreeMap::new();
    #[cfg(feature = "memory")]
    if !answers.is_empty() {
        let state = crate::state::state_dir()?;
        std::fs::create_dir_all(&state)?;
        let mut c = crate::app::open(&crate::status::memory_db(&state)?, true)?;
        let scope = crate::hook::project_of(&root_s);
        let today = time::OffsetDateTime::now_utc().date().to_string();
        for (key, value) in &answers {
            for id in known.get(key).map(|k| k.0.clone()).unwrap_or_default() {
                crate::memory::forget(&mut c, id)?;
            }
            crate::memory::remember(
                &mut c,
                &json!([{"text": format!("{}{value}", head(&root_s, key)), "kind": "fact", "scope": scope, "observed_at": today}]),
                &[],
                "cli",
            )?;
            known.insert(key, (vec![], value.clone()));
        }
    }
    #[cfg(not(feature = "memory"))]
    if !answers.is_empty() {
        bail!("--set needs a build with the memory feature");
    }

    let libs = crate::stack::detected(&root)?;
    let names: Vec<String> = libs
        .iter()
        .filter_map(|l| l["name"].as_str().map(String::from))
        .collect();
    let have_manifest = libs.iter().any(|l| l["source"] == "manifest");
    let docs = cached(&names);
    let tests = test_command(&root);
    let readme =
        text_of(&root, &["readme", "goal", "product"]).is_some_and(|t| t.trim().len() >= 40);
    let answered = |k: &str| known.contains_key(k);
    let docs_next = match names.first() {
        Some(n) => format!("footnote docs add <docs URL of {n}> --site {n}"),
        None => "footnote stack <path> --library <ecosystem>:<name>, then footnote docs add <docs URL> --site <name>".into(),
    };
    // (name, weight, earned, why it matters, next action)
    let checks: Vec<(&str, u32, u32, &str, String)> = vec![
        (
            "stack",
            15,
            if names.is_empty() { 0 } else { 15 },
            "Knowing the libraries lets Footnote fetch the right docs and lets you pick compatible versions.",
            "footnote stack <path> --library <ecosystem>:<name>".into(),
        ),
        (
            "dependency_file",
            10,
            if have_manifest { 10 } else { 0 },
            "A dependency file makes the project reproducible and gives Footnote exact versions.",
            "footnote stack write <path> --yes".into(),
        ),
        (
            "docs_cached",
            20,
            if names.is_empty() {
                0
            } else {
                (20 * docs / names.len()) as u32
            },
            "Cached docs let the agent answer from the real documentation of your versions, not from memory.",
            docs_next,
        ),
        (
            "readme",
            5,
            if readme { 5 } else { 0 },
            "A README or goal file tells a new reader or agent what the project is.",
            "write a README.md with the goal, how to install and how to run it".into(),
        ),
        (
            "goal",
            20,
            if answered("goal") { 20 } else { 0 },
            "Without a one-sentence goal the work has no way to be judged finished.",
            "footnote frame <path> --set goal=\"...\"".into(),
        ),
        (
            "users",
            10,
            if answered("users") { 10 } else { 0 },
            "Who uses it decides what to build first and what can wait.",
            "footnote frame <path> --set users=\"...\"".into(),
        ),
        (
            "constraints",
            5,
            if answered("constraints") { 5 } else { 0 },
            "Deadlines, platform limits or things it must not do change the design early and are costly later.",
            "footnote frame <path> --set constraints=\"...\"".into(),
        ),
        (
            "tests",
            10,
            if tests.is_some() || answered("tests") {
                10
            } else {
                0
            },
            "A test command is how the agent checks its own work.",
            "footnote frame <path> --set tests=\"<command>\"".into(),
        ),
        (
            "git",
            5,
            if root.join(".git").exists() { 5 } else { 0 },
            "A git repository gives every change a history you can review and undo.",
            "git init".into(),
        ),
    ];
    let score: u32 = checks.iter().map(|c| c.2).sum();
    let band = if score >= READY {
        "ready"
    } else if score >= ALMOST {
        "almost"
    } else {
        "not yet"
    };
    let rows: Vec<Value> = checks
        .iter()
        .map(|(name, weight, earned, why, next)| {
            json!({"name": name, "weight": weight, "earned": earned, "passed": earned == weight, "why": why,
                "next": if earned == weight { Value::Null } else { json!(next) }})
        })
        .collect();
    let questions: Vec<Value> = QUESTIONS
        .iter()
        .filter(|(k, _)| !answered(k) && !(*k == "tests" && tests.is_some()))
        .map(|(k, q)| json!({"key": k, "question": q}))
        .collect();
    let given: BTreeMap<&str, &String> = known.iter().map(|(k, v)| (*k, &v.1)).collect();
    let open = checks
        .iter()
        .filter(|c| c.2 < c.1)
        .max_by_key(|c| c.1 - c.2);
    let mut text =
        format!("Readiness {score}/100: {band} (ready from {READY}, almost from {ALMOST}).");
    for (name, weight, earned, ..) in &checks {
        text += &format!(
            "\n  [{}] {name} {earned}/{weight}",
            if earned == weight { "x" } else { " " }
        );
    }
    if !questions.is_empty() {
        text += "\nStill to answer:";
        for q in &questions {
            text += &format!(
                "\n  {}: {}",
                q["key"].as_str().unwrap_or_default(),
                q["question"].as_str().unwrap_or_default()
            );
        }
    }
    if let Some((name, _, _, _, next)) = open {
        text += &format!("\nNext ({name}): {next}");
    }
    Ok(json!({"root": root_s, "score": score, "band": band,
        "thresholds": {"ready": READY, "almost": ALMOST}, "checks": rows, "questions": questions,
        "answers": given, "next": open.map(|c| c.4.clone()), "text": text}))
}
