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
    event_in("s1", name, prompt)
}

fn event_in(session: &str, name: &str, prompt: &str) -> Value {
    json!({"hook_event_name": name, "session_id": session, "cwd": env!("CARGO_MANIFEST_DIR"), "prompt": prompt})
}
fn seed(state: &Path, models: &[&str]) {
    let mut c = command(state);
    c.arg("--db").arg(state.join("memory.db")).arg("remember");
    for model in models {
        c.args(["--model", model]);
    }
    c.write_stdin(json!([
        {"text": "Always show relative % gaps in every benchmark table.", "kind": "preference", "scope": "global", "observed_at": "2026-11-14"},
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
    // Already in this session's pack: not repeated (#242). Another session gets it, as a short recall line.
    let out = hook(state, event("UserPromptSubmit", "- benchmark tables gaps"));
    assert!(!out.contains("relative % gaps"), "{out}");
    let other = hook(
        state,
        event_in("s9", "UserPromptSubmit", "- benchmark tables gaps"),
    );
    assert!(
        other.contains("relative % gaps") && other.contains("[id "),
        "{other}"
    );
    let again = hook(
        state,
        event_in("s9", "UserPromptSubmit", "- benchmark tables gaps"),
    );
    assert!(!again.contains("relative % gaps"), "{again}");
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
    let mut st = read(&state.join("memory-hook/s1.json"));
    // Ids of the memories already shown, numbers only, never their text.
    let shown = st.as_object_mut().unwrap().remove("shown").unwrap();
    assert!(
        !shown.as_array().unwrap().is_empty()
            && shown.as_array().unwrap().iter().all(Value::is_i64),
        "{shown}"
    );
    assert_eq!(
        st,
        json!({"turns":3,"asked_at":3,"embed":false,"handoff_asked":false})
    );
    // A build with the `metrics` feature always writes timings (#265); otherwise they are off by default.
    #[cfg(not(feature = "metrics"))]
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
        json!({"turns":3,"asked_at":0,"embed":true,"shown":[],"handoff_asked":false})
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
        hook(
            state,
            event_in("r1", "UserPromptSubmit", "benchmark tables gaps")
        )
        .contains("relative % gaps")
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
                "footnote",
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
            event_in(
                "r2",
                "UserPromptSubmit",
                "how should tables show differences"
            )
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
    let out = command(state)
        .arg("projects")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
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
    assert_eq!(
        p["memories"], 0,
        "the seeded memories belong to `global` and `other`: {list}"
    );
    let first = p["first_seen"].clone();
    hook(state, event("SessionStart", ""));
    let again: Value = serde_json::from_slice(
        &command(state)
            .arg("projects")
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(again["projects"].as_array().unwrap().len(), 1);
    assert_eq!(again["projects"][0]["first_seen"], first);
}

/// Past the context threshold, Stop asks once for a handoff (#250); the saved handoff leads the next session's pack.
#[test]
fn a_long_session_is_asked_once_for_a_handoff_that_leads_the_next_pack() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    seed(state, &[]);
    let transcript = state.join("session.jsonl");
    let usage = json!({"type": "assistant", "message": {"content": [], "usage": {"input_tokens": 5,
        "cache_read_input_tokens": 240_000, "cache_creation_input_tokens": 1_000, "output_tokens": 9}}});
    fs::write(&transcript, format!("{usage}\n")).unwrap();
    let mut stop = event_in("h1", "Stop", "");
    stop["transcript_path"] = json!(transcript);
    let ask: Value = serde_json::from_str(&hook(state, stop.clone())).unwrap();
    assert_eq!(ask["decision"], "block");
    let reason = ask["reason"].as_str().unwrap();
    assert!(
        reason.contains("241k") && reason.contains(r#"tags ["handoff"]"#),
        "{reason}"
    );
    assert_eq!(hook(state, stop), "", "asked once per session");

    let project = reason
        .split("scope \"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let today = time::OffsetDateTime::now_utc().date().to_string();
    command(state)
        .arg("--db")
        .arg(state.join("memory.db"))
        .arg("remember")
        .write_stdin(
            json!([{"text": "Handoff: session handoff built, tests next.", "kind": "session", "scope": project,
                "observed_at": today, "tags": ["handoff"]}])
            .to_string(),
        )
        .assert()
        .success();
    let pack_of = |session: &str, source: &str| -> String {
        let mut start = event_in(session, "SessionStart", "");
        start["source"] = json!(source);
        let out = hook(state, start);
        serde_json::from_str::<Value>(&out).map_or(String::new(), |v| {
            v["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .unwrap_or("")
                .to_string()
        })
    };
    // A compaction replays the handoff without consuming it: the same session may have just written it.
    assert!(pack_of("h1", "compact").contains("Handoff: session handoff built"));
    let pack = pack_of("h2", "clear");
    assert!(
        pack.lines()
            .nth(1)
            .unwrap()
            .contains("Handoff: session handoff built"),
        "{pack}"
    );
    // Delivered once, then retired so it stops bloating later packs and recall.
    assert!(!pack_of("h3", "startup").contains("Handoff: session handoff built"));
}

#[test]
fn five_pending_global_preferences_remind_the_user_at_session_start() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    seed(state, &[]);
    // Agents save global preferences over MCP; each waits for the user's review (#207).
    let propose = |n: usize| {
        let mut input = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string() + "
";
        for i in 0..n {
            input += &(json!({"jsonrpc":"2.0","id":i + 2,"method":"tools/call","params":{"name":"footnote_remember","arguments":{"memories":[
                {"text": format!("Agent-proposed preference {i}."), "kind": "preference", "scope": "global", "observed_at": "2026-11-19"}]}}}).to_string() + "
");
        }
        command(state)
            .arg("--db")
            .arg(state.join("memory.db"))
            .args(["serve", "--mcp"])
            .write_stdin(input)
            .assert()
            .success();
    };
    propose(4);
    let start = hook(state, event("SessionStart", ""));
    assert!(
        !start.contains("waiting for the user's review") && !start.contains("Agent-proposed"),
        "{start}"
    );
    propose(5); // the same four again, plus a fifth
    let start = hook(state, event_in("s2", "SessionStart", ""));
    assert!(
        start.contains("5 global preferences") && !start.contains("Agent-proposed"),
        "{start}"
    );
}

#[test]
fn antigravity_hooks_inject_steps_deduplicate_turn_and_use_continue_on_stop() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    fs::write(state.join("memory.json"), r#"{"every_turns":2}"#).unwrap();
    seed(state, &[]);

    let agy_event = |name: &str, extra: Value| {
        let mut v = json!({
            "hookEventName": name,
            "conversationId": "agy-conv-1",
            "workspacePaths": [env!("CARGO_MANIFEST_DIR")],
            "cwd": env!("CARGO_MANIFEST_DIR"),
        });
        if let Some(obj) = extra.as_object() {
            for (k, val) in obj {
                v[k] = val.clone();
            }
        }
        v
    };

    // SessionStart emits Antigravity `injectSteps` with nested `systemMessage`.
    let start: Value =
        serde_json::from_str(&hook(state, agy_event("SessionStart", json!({})))).unwrap();
    let pack = start["injectSteps"][0]["systemMessage"]["systemMessage"]
        .as_str()
        .unwrap();
    assert!(pack.contains("relative % gaps"), "{start}");

    // Another conversation gets PreInvocation recall once per user turn, not on every tool-loop step.
    let pre1 = json!({
        "hookEventName": "PreInvocation",
        "conversationId": "agy-conv-2",
        "cwd": env!("CARGO_MANIFEST_DIR"),
        "invocationNum": 1,
        "lastUserInput": "benchmark tables gaps secret-phrase",
    });
    let r1: Value = serde_json::from_str(&hook(state, pre1.clone())).unwrap();
    assert!(
        r1["injectSteps"][0]["systemMessage"]["systemMessage"]
            .as_str()
            .unwrap()
            .contains("relative % gaps"),
        "{r1}"
    );
    // Second model step inside the same turn: no-op `{}` and turn count stays 1.
    let mut pre2 = pre1.clone();
    pre2["invocationNum"] = json!(2);
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, pre2)).unwrap(),
        json!({})
    );
    let st_raw = fs::read_to_string(state.join("memory-hook/agy-conv-2.json")).unwrap();
    assert!(!st_raw.contains("secret-phrase"), "{st_raw}");
    assert_eq!(serde_json::from_str::<Value>(&st_raw).unwrap()["turns"], 1);

    // Normal Stop on turn 1 (< every_turns): `{}` and resets turn hash so the next turn counts even with same text.
    let stop_ok = json!({
        "hookEventName": "Stop",
        "conversationId": "agy-conv-2",
        "cwd": env!("CARGO_MANIFEST_DIR"),
        "terminationReason": "NO_TOOL_CALL",
        "continuationActive": false,
    });
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, stop_ok.clone())).unwrap(),
        json!({})
    );

    // Turn 2 with the exact same prompt text: counts as turn 2.
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, pre1.clone())).unwrap(),
        json!({})
    );
    assert_eq!(read(&state.join("memory-hook/agy-conv-2.json"))["turns"], 2);

    // USER_CANCELED never asks to continue.
    let mut stop_cancel = stop_ok.clone();
    stop_cancel["terminationReason"] = json!("USER_CANCELED");
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, stop_cancel)).unwrap(),
        json!({})
    );

    // Turn 3 -> Stop with NO_TOOL_CALL asks Antigravity to `continue` (not `block`).
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, pre1.clone())).unwrap(),
        json!({})
    );
    let ask: Value = serde_json::from_str(&hook(state, stop_ok.clone())).unwrap();
    assert_eq!(ask["decision"], "continue", "{ask}");
    assert!(
        ask["reason"]
            .as_str()
            .unwrap()
            .contains("footnote_remember")
    );

    // Continuation PreInvocation inside the save loop must not double-count turns.
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, pre1)).unwrap(),
        json!({})
    );
    assert_eq!(read(&state.join("memory-hook/agy-conv-2.json"))["turns"], 3);

    // Final Stop with continuationActive=true returns `{}`.
    let mut stop_cont = stop_ok;
    stop_cont["continuationActive"] = json!(true);
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, stop_cont)).unwrap(),
        json!({})
    );
}

#[test]
fn gemini_cli_hooks_handle_before_agent_and_after_agent_with_deny_and_empty_json() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    fs::write(state.join("memory.json"), r#"{"every_turns":1}"#).unwrap();
    seed(state, &[]);

    let before = json!({
        "hook_event_name": "BeforeAgent",
        "session_id": "gemini-sess-1",
        "cwd": env!("CARGO_MANIFEST_DIR"),
        "prompt": "benchmark tables gaps",
    });
    let r1: Value = serde_json::from_str(&hook(state, before)).unwrap();
    assert_eq!(r1["hookSpecificOutput"]["hookEventName"], "BeforeAgent");
    assert!(
        r1["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains("relative % gaps"),
        "{r1}"
    );

    // AfterAgent on turn 1 (>= every_turns=1) emits `decision: "deny"` for Gemini CLI retry loop.
    let after = json!({
        "hook_event_name": "AfterAgent",
        "session_id": "gemini-sess-1",
        "cwd": env!("CARGO_MANIFEST_DIR"),
        "stop_hook_active": false,
    });
    let ask: Value = serde_json::from_str(&hook(state, after.clone())).unwrap();
    assert_eq!(ask["decision"], "deny", "{ask}");
    assert!(
        ask["reason"]
            .as_str()
            .unwrap()
            .contains("footnote_remember")
    );

    // Final AfterAgent with stop_hook_active=true emits `{}`.
    let mut after_done = after;
    after_done["stop_hook_active"] = json!(true);
    assert_eq!(
        serde_json::from_str::<Value>(&hook(state, after_done)).unwrap(),
        json!({})
    );
}

/// #280: off by default; on, a theme's skill is suggested once per session when it holds 40% of the prompt's topic words,
/// and a plain prompt gets nothing.
#[test]
fn skills_are_suggested_only_when_on_and_matching() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    let skills = state.join("my-skills");
    for (name, desc) in [
        (
            "css-transitions",
            "Add CSS transitions to dropdowns, modals and toasts.",
        ),
        ("sql-queries", "Write and tune SQL queries for Postgres."),
    ] {
        fs::create_dir_all(skills.join(name)).unwrap();
        fs::write(
            skills.join(name).join("SKILL.md"),
            format!("---\nname: {name}\ndescription: >-\n  {desc}\n---\n# Body\nLong body text.\n"),
        )
        .unwrap();
    }
    let out = command(state)
        .args(["skills", "index", "web"])
        .arg(&skills)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(serde_json::from_slice::<Value>(&out).unwrap()["skills"], 2);
    let prompt = "add a transition to the dropdown";
    assert!(!hook(state, event("UserPromptSubmit", prompt)).contains("css-transitions"));
    fs::write(state.join("memory.json"), r#"{"skills":true}"#).unwrap();
    let out = hook(state, event("UserPromptSubmit", prompt));
    assert!(
        out.contains("css-transitions") && out.contains("SKILL.md"),
        "{out}"
    );
    assert!(!out.contains("sql-queries"), "{out}");
    // Already in this context: not repeated. A plain prompt gets nothing.
    assert!(!hook(state, event("UserPromptSubmit", prompt)).contains("css-transitions"));
    assert!(
        hook(
            state,
            event("UserPromptSubmit", "fix the failing login test")
        )
        .trim()
        .is_empty()
    );
    command(state)
        .args(["skills", "remove", "web"])
        .assert()
        .success();
    assert!(!hook(state, event_in("s2", "UserPromptSubmit", prompt)).contains("css-transitions"));
}

/// #443: a suggested skill is watched in the session file; at Stop its read and script runs are logged with names and
/// counts only, and `footnote skills stats` adds them up. With `activity` off nothing is logged.
#[test]
fn skill_outcomes_log_reads_and_script_runs() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path();
    let skills = state.join("my-skills");
    let dir = skills.join("css-transitions");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: css-transitions\ndescription: Add CSS transitions to dropdowns, modals and toasts.\n---\n# Body\n",
    )
    .unwrap();
    command(state)
        .args(["skills", "index", "web"])
        .arg(&skills)
        .assert()
        .success();
    let transcript = state.join("session.jsonl");
    // Before the suggestion: never counted.
    let early = json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "x0", "name": "Read", "input": {"file_path": dir.join("SKILL.md")}}]}});
    fs::write(&transcript, format!("{early}\n")).unwrap();
    let with_transcript = |name: &str, session: &str| {
        let mut ev = event_in(session, name, "add a transition to the dropdown");
        ev["transcript_path"] = json!(transcript);
        ev
    };
    let log = state.join("memory-hook").join("skill-outcomes.jsonl");

    fs::write(
        state.join("memory.json"),
        r#"{"skills":true,"activity":false}"#,
    )
    .unwrap();
    assert!(hook(state, with_transcript("UserPromptSubmit", "off")).contains("css-transitions"));
    hook(state, with_transcript("Stop", "off"));
    assert!(!log.exists(), "activity off must log nothing");

    fs::write(state.join("memory.json"), r#"{"skills":true}"#).unwrap();
    assert!(hook(state, with_transcript("UserPromptSubmit", "s1")).contains("css-transitions"));
    let script = dir.join("scripts").join("check.mjs");
    let lines = [
        json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "a", "name": "Read", "input": {"file_path": dir.join("SKILL.md")}}]}}),
        json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "b", "name": "Bash", "input": {"command": format!("node {} --secret-flag", script.display())}}]}}),
        json!({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": "b", "is_error": true, "content": "Exit code 1\nboom"}]}}),
        json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "c", "name": "Read", "input": {"file_path": dir.join("SKILL.md")}}]}}),
    ];
    let mut text = fs::read_to_string(&transcript).unwrap();
    for l in &lines {
        text.push_str(&format!("{l}\n"));
    }
    fs::write(&transcript, text).unwrap();
    hook(state, with_transcript("Stop", "s1"));
    // A second Stop rescans nothing old.
    hook(state, with_transcript("Stop", "s1"));

    let rows: Vec<Value> = fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let events: Vec<(&str, Option<bool>)> = rows
        .iter()
        .map(|r| (r["event"].as_str().unwrap(), r["ok"].as_bool()))
        .collect();
    assert_eq!(
        events,
        vec![("suggested", None), ("read", None), ("run", Some(false))]
    );
    let raw = fs::read_to_string(&log).unwrap();
    for leak in [
        "secret-flag",
        "SKILL.md",
        "check.mjs",
        "dropdown",
        "my-skills",
    ] {
        assert!(!raw.contains(leak), "{leak} leaked into {raw}");
    }
    let out = command(state)
        .args(["skills", "stats"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stats: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        stats["skills"][0],
        json!({"skill": "css-transitions", "theme": "web", "suggested": 1, "read": 1, "runs": 1, "failures": 1})
    );
}
