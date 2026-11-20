use assert_cmd::Command;
use serde_json::{Value, json};
use std::{fs, path::Path, time::Duration};

fn footnote(state: &Path, args: &[&str], stdin: Option<Value>) -> Value {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
    c.env("FOOTNOTE_STATE_DIR", state)
        .env("FOOTNOTE_POLICY_FILE", state.join("no-policy.json"))
        .args(args)
        .timeout(Duration::from_secs(30));
    if let Some(v) = stdin {
        c.write_stdin(v.to_string());
    }
    let out = c.output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap()
    }
}

#[test]
fn browse_activity_and_jobs_show_what_memory_did_without_prompts() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    let db = state.join("memory.db");
    let db_arg = db.to_str().unwrap();
    let today = time::OffsetDateTime::now_utc().date().to_string();
    let m = |text: &str, scope: &str| json!({"text": text, "kind": "fact", "scope": scope, "observed_at": today});
    footnote(
        state,
        &["--db", db_arg, "remember"],
        Some(json!([
            m("Releases are tagged on Fridays.", "alpha"),
            m("Alpha deploys with blue-green.", "alpha"),
            m("Beta uses SQLite.", "beta"),
            m("Deploys happen after releases are tagged.", "global")
        ])),
    );

    // Browse: one project, paged, ids usable by forget.
    let list = footnote(
        state,
        &[
            "--db",
            db_arg,
            "memory-list",
            "--project",
            "alpha",
            "--limit",
            "1",
        ],
        None,
    );
    assert_eq!(list["total"], 2, "{list}");
    assert_eq!(list["memories"].as_array().unwrap().len(), 1);
    assert_eq!(list["scopes"].as_array().unwrap().len(), 3);
    let id = list["memories"][0]["id"].as_i64().unwrap().to_string();
    footnote(state, &["--db", db_arg, "forget", &id], None);
    assert_eq!(
        footnote(
            state,
            &["--db", db_arg, "memory-list", "--project", "alpha"],
            None
        )["total"],
        1
    );

    // The hook records ids and projects, never the prompt.
    let secret = "deploys tagged releases zebra-secret-phrase";
    footnote(
        state,
        &["hook"],
        Some(
            json!({"hook_event_name": "UserPromptSubmit", "session_id": "s", "cwd": state, "prompt": secret}),
        ),
    );
    let log = fs::read_to_string(state.join("memory-hook/activity.jsonl")).unwrap();
    assert!(
        log.contains("\"recall\"") && !log.contains("zebra"),
        "{log}"
    );
    let st = footnote(state, &["status"], None);
    assert_eq!(st["products"]["memory"]["recalls_last_7_days"], 1, "{st}");
    let act = footnote(state, &["activity", "--days", "3"], None);
    let days = act["days"].as_array().unwrap();
    assert_eq!(days.len(), 3);
    let day = days.iter().find(|d| d["date"] == today.as_str()).unwrap();
    assert_eq!(day["saved"], 3, "{act}"); // four saved, one forgotten
    assert_eq!(day["prompts_given_memories"], 1, "{act}");
    assert!(act["most_used"][0]["memory"]["text"].is_string(), "{act}");
    // Recent recalls, newest first, name the memories that reached the agent (never the prompt).
    let recent = act["recent"].as_array().unwrap();
    assert!(
        !recent.is_empty() && recent[0]["memories"][0]["id"].is_i64(),
        "{act}"
    );
    assert!(
        recent[0]["memories"][0]["scope"].is_string() || recent[0]["memories"][0]["gone"] == true,
        "{act}"
    );
    assert!(
        !act["recent"].to_string().contains("prompt\":"),
        "no prompt text: {act}"
    );

    // Switched off: nothing more is logged.
    footnote(state, &["config", "set", "memory.activity", "false"], None);
    footnote(
        state,
        &["hook"],
        Some(
            json!({"hook_event_name": "UserPromptSubmit", "session_id": "s", "cwd": state, "prompt": "deploys releases"}),
        ),
    );
    assert_eq!(
        fs::read_to_string(state.join("memory-hook/activity.jsonl")).unwrap(),
        log
    );

    // Jobs: an indexing run is listed, finished, with its count.
    let docs = state.join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(docs.join("a.md"), "# A\nalpha\n").unwrap();
    fs::write(docs.join("b.md"), "# B\nbeta\n").unwrap();
    let corpus = state.join("corpus.db");
    let d = docs.to_str().unwrap();
    footnote(
        state,
        &[
            "--db",
            corpus.to_str().unwrap(),
            "index",
            "--root",
            d,
            "--allow",
            d,
        ],
        None,
    );
    let jobs = footnote(state, &["jobs"], None);
    let job = &jobs["jobs"][0];
    assert_eq!(
        (job["kind"].as_str(), job["state"].as_str()),
        (Some("library.index"), Some("done")),
        "{jobs}"
    );
    assert_eq!(job["done"], 2, "{jobs}");
}
