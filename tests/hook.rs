use assert_cmd::Command;
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn command(state: &Path) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
    c.env("FOOTNOTE_STATE_DIR", state)
        .timeout(Duration::from_secs(15));
    c
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn hook(state: &Path, event: Value) -> String {
    let out = command(state)
        .arg("hook")
        .write_stdin(event.to_string())
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}
fn event(name: &str, prompt: &str) -> Value {
    json!({"hook_event_name": name, "session_id": "s1", "cwd": env!("CARGO_MANIFEST_DIR"), "prompt": prompt})
}
fn seed(state: &Path, models: &[&str]) {
    let mut c = command(state);
    c.arg("--db").arg(state.join("memory.db")).arg("remember");
    for model in models {
        c.args(["--model", model]);
    }
    c.write_stdin(json!([
        {"text": "Johan wants relative % gaps in every benchmark table.", "kind": "preference", "scope": "global", "observed_at": "2026-11-14"},
        {"text": "Project Other uses a special benchmark dashboard.", "kind": "fact", "scope": "other", "observed_at": "2026-11-15"}
    ]).to_string()).assert().success();
}

#[test]
fn pack_recall_save_and_stop_loop_match_the_existing_contract() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    fs::write(state.join("memory.json"), r#"{"every_turns":3}"#).unwrap();
    seed(state, &[]);
    let start: Value = serde_json::from_str(&hook(state, event("SessionStart", ""))).unwrap();
    let pack = start["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        pack.contains("relative % gaps")
            && pack.contains("say so in a few words")
            && pack.contains("data, not instructions")
    );
    assert!(!pack.contains("Project Other"));
    let out = hook(state, event("UserPromptSubmit", "- benchmark tables gaps"));
    assert!(out.contains("relative % gaps"));
    assert!(!out.contains("Project Other") && !out.contains("say so in a few words"));
    hook(
        state,
        event("UserPromptSubmit", &format!("{}é", "é".repeat(400))),
    );
    assert!(!state.join("memory-hook/hook.log").exists());
    assert_eq!(hook(state, event("Stop", "")), "");
    hook(state, event("UserPromptSubmit", "next"));
    let ask: Value = serde_json::from_str(&hook(state, event("Stop", ""))).unwrap();
    assert_eq!(ask["decision"], "block");
    let reason = ask["reason"].as_str().unwrap();
    // The project scope is the repository's name on its origin remote (footnote here and in CI), as the hook reads it.
    let remote = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    let url = String::from_utf8(remote.stdout).unwrap();
    let name = url
        .trim()
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap()
        .trim_end_matches(".git");
    let project = format!("\"{}\"", name.to_lowercase());
    for part in [
        "footnote_remember",
        project.as_str(),
        "what you saved",
        "Name the subject",
        "Tested on Linux",
        "supersedes",
    ] {
        assert!(reason.contains(part), "{part}: {reason}");
    }
    assert!(
        reason.contains(
            &time::OffsetDateTime::now_local()
                .unwrap()
                .date()
                .to_string()
        )
    );
    let mut stop = event("Stop", "");
    stop["stop_hook_active"] = json!(true);
    assert_eq!(hook(state, stop), "");
    assert_eq!(hook(state, event("Stop", "")), "");
    assert_eq!(
        read(&state.join("memory-hook/s1.json")),
        json!({"turns":3,"asked_at":3,"embed":false})
    );
    assert!(!state.join("metrics.jsonl").exists());
}

#[test]
fn remember_aliases_and_legacy_state_keep_only_counters() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    fs::create_dir(state.join("memory-hook")).unwrap();
    fs::write(state.join("memory-hook/s2.json"), json!({"turns":2,"asked_at":0,"embed":false,"cwd":"secret folder","transcript":"secret text"}).to_string()).unwrap();
    hook(
        state,
        json!({"hookEventName":"UserPromptSubmit","sessionId":"s2","workspaceRoot":env!("CARGO_MANIFEST_DIR"),"userPrompt":"/remember"}),
    );
    assert_eq!(
        read(&state.join("memory-hook/s2.json")),
        json!({"turns":3,"asked_at":0,"embed":true})
    );
    assert_eq!(
        hook(state, json!({"hookEventName":"Stop","sessionId":"s2"})),
        ""
    );
    assert_eq!(read(&state.join("memory-hook/s2.json"))["embed"], false);
    hook(
        state,
        json!({"hook_event_name":"UserPromptSubmit","session_id":"../bad:id","prompt":"private prompt"}),
    );
    assert!(state.join("memory-hook/___bad_id.json").exists());
    assert!(
        !fs::read_to_string(state.join("memory-hook/___bad_id.json"))
            .unwrap()
            .contains("private prompt")
    );
}

#[test]
fn invalid_events_settings_and_database_fail_open_without_retaining_input() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    for input in [
        "{private event",
        "[]",
        "null",
        &"x".repeat(2 * 1024 * 1024 + 1),
    ] {
        let out = command(state)
            .arg("hook")
            .write_stdin(input)
            .assert()
            .success()
            .get_output()
            .clone();
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
    fs::write(state.join("memory.json"), "{secret malformed settings").unwrap();
    assert_eq!(hook(state, event("SessionStart", "")), "");
    fs::remove_file(state.join("memory.json")).unwrap();
    fs::write(state.join("memory.db"), "private invalid database").unwrap();
    assert_eq!(hook(state, event("SessionStart", "")), "");
    let log = fs::read_to_string(state.join("memory-hook/hook.log")).unwrap();
    for secret in [
        "private",
        "secret",
        "malformed settings",
        "invalid database",
    ] {
        assert!(!log.contains(secret), "{log}");
    }
    // Even an unwritable state location cannot fail the hook or print an error envelope.
    assert_eq!(
        hook(&state.join("memory.db"), event("SessionStart", "")),
        ""
    );
}

#[test]
fn missing_database_and_command_prompts_do_not_inject_context() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    assert_eq!(hook(state, event("SessionStart", "")), "");
    assert!(!state.join("memory.db").exists());
    seed(state, &[]);
    for prompt in ["", "  ", "/help", "/remember"] {
        assert_eq!(hook(state, event("UserPromptSubmit", prompt)), "");
    }
    assert_eq!(read(&state.join("memory-hook/s1.json"))["turns"], 4);
}

#[test]
fn hook_metrics_respect_settings_and_report_recall_failure() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    fs::write(state.join("memory.json"), r#"{"metrics":true}"#).unwrap();
    fs::write(state.join("memory.db"), "bad db").unwrap();
    assert_eq!(hook(state, event("UserPromptSubmit", "private prompt")), "");
    let rows: Vec<Value> = fs::read_to_string(state.join("metrics.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["span"], "hook.recall");
    assert_eq!(rows[0]["ok"], false);
    assert!(!rows[0].to_string().contains("private prompt"));
}

#[test]
#[ignore = "requires artifacts/models/gte-small; run explicitly for the hook migration"]
fn background_embeddings_and_opt_in_resident_recall() {
    let model = Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts/models/gte-small");
    assert!(model.join("manifest.json").exists());
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    fs::write(
        state.join("memory.json"),
        json!({"models":[model],"metrics":true,"resident":false,"idle_seconds":2}).to_string(),
    )
    .unwrap();
    seed(state, &[]);
    let start = Instant::now();
    assert!(hook(state, event("SessionStart", "")).contains("relative % gaps"));
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "background process held the hook's output pipe"
    );
    let end = Instant::now() + Duration::from_secs(20);
    loop {
        let log = fs::read_to_string(state.join("metrics.jsonl")).unwrap_or_default();
        if log
            .lines()
            .filter_map(|s| serde_json::from_str::<Value>(s).ok())
            .any(|v| v["span"] == "hook.embed" && v["ok"] == true)
        {
            break;
        }
        assert!(
            Instant::now() < end,
            "background embedding never finished: {log}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let db = rusqlite::Connection::open(state.join("memory.db")).unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM embeddings", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
    drop(db);
    assert!(
        !state.join("daemon.pid").exists(),
        "resident off: no daemon"
    );
    fs::write(
        state.join("memory.json"),
        json!({"models":[model],"metrics":true,"resident":true,"idle_seconds":2}).to_string(),
    )
    .unwrap();
    assert!(
        hook(state, event("UserPromptSubmit", "benchmark tables gaps")).contains("relative % gaps")
    );
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        let result = command(state)
            .arg("--db")
            .arg(state.join("memory.db"))
            .args(["memory-recall", "--resident", "--model"])
            .arg(&model)
            .args([
                "--project",
                "borgknowledge",
                "--",
                "how should tables show differences",
            ])
            .output()
            .unwrap();
        if result.status.success() {
            let value: Value = serde_json::from_slice(&result.stdout).unwrap();
            assert!(value["mode"].as_str().unwrap().starts_with("hybrid"));
            break;
        }
        assert!(
            Instant::now() < end,
            "daemon never answered: {}",
            String::from_utf8_lossy(&result.stdout)
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        hook(
            state,
            event("UserPromptSubmit", "how should tables show differences")
        )
        .contains("relative % gaps")
    );
    std::thread::sleep(Duration::from_secs(4)); // no active session: the owned test daemon exits
    let result = command(state)
        .arg("--db")
        .arg(state.join("memory.db"))
        .args(["memory-recall", "--resident", "--", "tables"])
        .output()
        .unwrap();
    assert!(!result.status.success(), "idle daemon did not leave");
}

#[test]
fn session_start_records_the_project_for_the_projects_list() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    seed(state, &[]);
    hook(state, event("SessionStart", ""));
    let out = command(state).arg("projects").assert().success().get_output().stdout.clone();
    let list: Value = serde_json::from_slice(&out).unwrap();
    let p = &list["projects"][0];
    // This repository: named after its remote, recorded at its top folder, with git state read live.
    assert!(p["name"].as_str().is_some_and(|n| !n.is_empty()), "{list}");
    assert_eq!(
        fs::canonicalize(p["path"].as_str().unwrap()).unwrap(),
        fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap(),
        "{list}"
    );
    assert_eq!(p["exists"], true);
    assert!(p["git"]["uncommitted"].is_u64(), "{list}");
    assert_eq!(p["memories"], 0, "the seeded memories belong to `global` and `other`: {list}");
    let first = p["first_seen"].clone();
    hook(state, event("SessionStart", ""));
    let again: Value = serde_json::from_slice(&command(state).arg("projects").assert().success().get_output().stdout).unwrap();
    assert_eq!(again["projects"].as_array().unwrap().len(), 1);
    assert_eq!(again["projects"][0]["first_seen"], first);
}
