//! Integration tests for #351 (Lean D2: PreToolUse hook, `footnote gain`, `install --lean --remove-rtk`, `uninstall --lean`).
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

#[test]
fn hook_lean_rewrites_supported_bash_and_passes_through_pipes() {
    let state = TempDir::new().unwrap();
    let payload = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "cargo test --workspace",
            "timeout": 120000
        }
    });
    let out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .env("PATH", "")
        .args(["hook", "lean"])
        .write_stdin(payload.to_string())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let rewritten = v["hookSpecificOutput"]["updatedInput"]["command"]
        .as_str()
        .unwrap();
    assert!(
        rewritten.ends_with(" run -- cargo test --workspace") && rewritten.contains("footnote"),
        "expected absolute-path fallback rewrite when PATH is empty, got {rewritten:?}"
    );
    assert_eq!(v["hookSpecificOutput"]["updatedInput"]["timeout"], 120000);
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "allow");

    // When `footnote` IS on PATH, the hook emits the compact `footnote run -- ...` form.
    let bin_dir = assert_cmd::cargo::cargo_bin!("footnote")
        .parent()
        .unwrap()
        .to_path_buf();
    let out_on_path = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .env("PATH", &bin_dir)
        .args(["hook", "lean"])
        .write_stdin(payload.to_string())
        .output()
        .unwrap();
    assert!(out_on_path.status.success(), "{out_on_path:?}");
    let v_path: Value = serde_json::from_slice(&out_on_path.stdout).unwrap();
    assert_eq!(
        v_path["hookSpecificOutput"]["updatedInput"]["command"],
        "footnote run -- cargo test --workspace"
    );

    // Piped commands must pass through untouched (empty stdout).
    let piped = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "rg foo src | head -n 5"}
    });
    let out2 = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["hook", "lean"])
        .write_stdin(piped.to_string())
        .output()
        .unwrap();
    assert!(out2.status.success(), "{out2:?}");
    assert!(
        String::from_utf8_lossy(&out2.stdout).trim().is_empty(),
        "piped command must not be rewritten"
    );
}

#[test]
fn hook_lean_rewrites_antigravity_tool_call_and_gemini_cli_before_tool() {
    let state = TempDir::new().unwrap();
    let bin_dir = assert_cmd::cargo::cargo_bin!("footnote")
        .parent()
        .unwrap()
        .to_path_buf();

    // 1. Antigravity PreToolUse sends `toolCall: { name: "run_command", args: { CommandLine } }`
    //    and expects `{ "decision": "allow", "overwrite": { "CommandLine": ... } }`.
    let agy_payload = json!({
        "conversationId": "conv-123",
        "hookEventName": "PreToolUse",
        "toolCall": {
            "id": "toolu_01",
            "name": "run_command",
            "args": {
                "CommandLine": "cargo test --workspace",
                "WaitMsBeforeAsync": 5000
            }
        }
    });
    let agy_out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .env("PATH", &bin_dir)
        .args(["hook", "lean"])
        .write_stdin(agy_payload.to_string())
        .output()
        .unwrap();
    assert!(agy_out.status.success(), "{agy_out:?}");
    let agy_v: Value = serde_json::from_slice(&agy_out.stdout).unwrap();
    assert_eq!(agy_v["decision"], "allow", "{agy_v}");
    assert_eq!(
        agy_v["overwrite"]["CommandLine"], "footnote run -- cargo test --workspace",
        "{agy_v}"
    );
    assert!(
        agy_v.get("hookSpecificOutput").is_none(),
        "Antigravity must not receive Claude hookSpecificOutput: {agy_v}"
    );

    // Antigravity passthrough (chained command) must emit valid empty JSON `{}`.
    let agy_chain = json!({
        "conversationId": "conv-123",
        "hookEventName": "PreToolUse",
        "toolCall": {
            "id": "toolu_02",
            "name": "run_command",
            "args": { "CommandLine": "git status && git diff" }
        }
    });
    let agy_pass = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .env("PATH", &bin_dir)
        .args(["hook", "lean"])
        .write_stdin(agy_chain.to_string())
        .output()
        .unwrap();
    assert!(agy_pass.status.success(), "{agy_pass:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&agy_pass.stdout).unwrap(),
        json!({})
    );

    // 2. Gemini CLI BeforeTool sends `tool_name: "run_shell_command"`, `tool_input: { command }`
    //    and expects `{ "hookSpecificOutput": { "tool_input": { "command": ... } } }`.
    let gemini_payload = json!({
        "hook_event_name": "BeforeTool",
        "tool_name": "run_shell_command",
        "tool_input": { "command": "git status" }
    });
    let gemini_out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .env("PATH", &bin_dir)
        .args(["hook", "lean"])
        .write_stdin(gemini_payload.to_string())
        .output()
        .unwrap();
    assert!(gemini_out.status.success(), "{gemini_out:?}");
    let gemini_v: Value = serde_json::from_slice(&gemini_out.stdout).unwrap();
    assert_eq!(
        gemini_v["hookSpecificOutput"]["tool_input"]["command"], "footnote run -- git status",
        "{gemini_v}"
    );

    // Gemini CLI passthrough must also emit valid empty JSON `{}`.
    let gemini_pipe = json!({
        "hook_event_name": "BeforeTool",
        "tool_name": "run_shell_command",
        "tool_input": { "command": "rg foo src | head -n 5" }
    });
    let gemini_pass = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .env("PATH", &bin_dir)
        .args(["hook", "lean"])
        .write_stdin(gemini_pipe.to_string())
        .output()
        .unwrap();
    assert!(gemini_pass.status.success(), "{gemini_pass:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&gemini_pass.stdout).unwrap(),
        json!({})
    );
}

#[test]
fn gain_reports_shell_savings_miss_rate_and_attrib() {
    let state = TempDir::new().unwrap();
    let mut noisy = String::new();
    for i in 0..40 {
        noisy.push_str(&format!("   Compiling dep-{i} v0.1.0\n"));
    }
    for i in 0..25 {
        noisy.push_str(&format!("test m::case_{i} ... ok\n"));
    }
    noisy.push_str("test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n");

    let run_out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["run", "--stdin", "--family", "cargo-test"])
        .write_stdin(noisy)
        .output()
        .unwrap();
    assert!(run_out.status.success());
    let text = String::from_utf8(run_out.stdout).unwrap();
    let handle = text
        .split("footnote output ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();

    let ret = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["output", &handle, "--head", "3"])
        .output()
        .unwrap();
    assert!(ret.status.success());

    let gain = cargo_bin_cmd!("footnote")
        .env("USERPROFILE", state.path())
        .env("HOME", state.path())
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["gain", "--days", "7", "--attrib"])
        .output()
        .unwrap();
    assert!(gain.status.success(), "{gain:?}");
    let v: Value = serde_json::from_slice(&gain.stdout).unwrap();
    assert_eq!(v["shell"]["commands"], 1, "{v}");
    assert!(v["shell"]["saved_bytes"].as_u64().unwrap() > 500, "{v}");
    assert_eq!(v["shell"]["handles_stored"], 1, "{v}");
    assert_eq!(v["shell"]["handles_retrieved"], 1, "{v}");
    assert_eq!(v["shell"]["miss_rate"], 1.0, "{v}");
    assert_eq!(v["shell"]["by_family"][0]["family"], "cargo-test", "{v}");
    assert!(v["attribution"]["totals"].is_object(), "{v}");
}

#[test]
fn install_lean_remove_rtk_and_uninstall_lean_preserves_memory_hooks() {
    let home = TempDir::new().unwrap();
    let state = TempDir::new().unwrap();
    fs::create_dir_all(home.path().join(".claude")).unwrap();
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    fs::create_dir_all(home.path().join(".gemini/config")).unwrap();
    let t3_cfg = home
        .path()
        .join(".t3/userdata/providers/antigravity/abc123/config");
    fs::create_dir_all(&t3_cfg).unwrap();

    // Seed Claude settings with an existing Footnote Memory hook AND an RTK hook.
    let claude_settings = home.path().join(".claude/settings.json");
    fs::write(
        &claude_settings,
        serde_json::to_string_pretty(&json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Bash", "hooks": [{"type": "command", "command": "rtk hook claude", "timeout": 10}]}
                ],
                "SessionStart": [
                    {"hooks": [{"type": "command", "command": "footnote hook", "timeout": 15}]}
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    // 1. --dry-run must not mutate files.
    let dry = cargo_bin_cmd!("footnote")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["install", "--lean", "--remove-rtk", "--dry-run"])
        .output()
        .unwrap();
    assert!(dry.status.success(), "{dry:?}");
    let after_dry = fs::read_to_string(&claude_settings).unwrap();
    assert!(
        after_dry.contains("rtk hook claude"),
        "dry-run modified file"
    );
    assert!(!after_dry.contains("hook lean"), "dry-run modified file");

    // 2. Real install --lean --remove-rtk installs Lean hook and removes RTK hook.
    let ins = cargo_bin_cmd!("footnote")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["install", "--lean", "--remove-rtk"])
        .output()
        .unwrap();
    assert!(ins.status.success(), "{ins:?}");
    let after_ins = fs::read_to_string(&claude_settings).unwrap();
    assert!(
        !after_ins.contains("rtk hook claude"),
        "rtk hook remained: {after_ins}"
    );
    assert!(
        after_ins.contains("hook lean"),
        "lean hook missing: {after_ins}"
    );
    assert!(
        after_ins.contains("footnote hook"),
        "memory hook should stay intact: {after_ins}"
    );
    // Antigravity PreToolUse must use the grouped `{matcher: "run_command", hooks: [...]}` schema
    // in both `~/.gemini/config/hooks.json` and T3 Code's isolated provider dir.
    for hooks_path in [
        home.path().join(".gemini/config/hooks.json"),
        t3_cfg.join("hooks.json"),
    ] {
        let agy_hooks: Value =
            serde_json::from_str(&fs::read_to_string(&hooks_path).unwrap()).unwrap();
        assert_eq!(
            agy_hooks["footnote-lean"]["PreToolUse"][0]["matcher"],
            "run_command",
            "{}: {agy_hooks}",
            hooks_path.display()
        );
        assert!(
            agy_hooks["footnote-lean"]["PreToolUse"][0]["hooks"][0]["command"]
                .as_str()
                .unwrap()
                .contains("hook lean"),
            "{}: {agy_hooks}",
            hooks_path.display()
        );
    }
    // Gemini CLI must use `BeforeTool` with `matcher: "run_shell_command"` and ms timeout (5000).
    let gemini_settings: Value = serde_json::from_str(
        &fs::read_to_string(home.path().join(".gemini/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        gemini_settings["hooks"]["BeforeTool"][0]["matcher"], "run_shell_command",
        "{gemini_settings}"
    );
    assert_eq!(
        gemini_settings["hooks"]["BeforeTool"][0]["hooks"][0]["timeout"], 5000,
        "{gemini_settings}"
    );

    // 3. Second install is idempotent ("already registered").
    let ins2 = cargo_bin_cmd!("footnote")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["install", "--lean"])
        .output()
        .unwrap();
    assert!(ins2.status.success());
    let v2: Value = serde_json::from_slice(&ins2.stdout).unwrap();
    assert!(
        v2["targets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["status"] == "already registered"),
        "{v2}"
    );

    // 4. uninstall --lean removes only Lean hooks, keeping Memory hooks intact.
    let un = cargo_bin_cmd!("footnote")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["uninstall", "--lean"])
        .output()
        .unwrap();
    assert!(un.status.success(), "{un:?}");
    let after_un = fs::read_to_string(&claude_settings).unwrap();
    assert!(
        !after_un.contains("hook lean"),
        "lean hook remained: {after_un}"
    );
    assert!(
        after_un.contains("footnote hook"),
        "memory hook was accidentally removed: {after_un}"
    );
    let t3_after: Value =
        serde_json::from_str(&fs::read_to_string(t3_cfg.join("hooks.json")).unwrap()).unwrap();
    assert!(
        t3_after.get("footnote-lean").is_none(),
        "t3-code lean hook remained: {t3_after}"
    );
}

#[test]
fn gain_missed_ranks_uncovered_prefixes_without_leaking_arguments_or_payloads() {
    let home = TempDir::new().unwrap();
    let state = TempDir::new().unwrap();
    let claude_proj = home.path().join(".claude/projects/demo");
    let codex_sess = home.path().join(".codex/sessions/2026/11/21");
    fs::create_dir_all(&claude_proj).unwrap();
    fs::create_dir_all(&codex_sess).unwrap();

    let today = time::OffsetDateTime::now_utc().date().to_string();
    let ts = format!("{today}T08:00:00Z");

    // Claude transcript: 1 covered (`cargo test`) + 2 uncovered (`gh api`, `curl`) containing sensitive args/payloads.
    let claude_lines = [
        json!({
            "type": "assistant",
            "timestamp": ts,
            "message": {"content": [
                {"type": "tool_use", "id": "t1", "name": "Bash", "input": {"command": "cargo test --workspace"}},
                {"type": "tool_use", "id": "t2", "name": "Bash", "input": {"command": "cd /private/secret-repo && gh api repos/org/priv/issues?secret=LEAK_ARG_1"}},
                {"type": "tool_use", "id": "t3", "name": "Bash", "input": {"command": "curl -H 'X-Token: LEAK_HEADER' https://internal.example/v1"}}
            ]}
        }),
        json!({
            "type": "user",
            "timestamp": ts,
            "message": {"content": [
                {"type": "tool_result", "tool_use_id": "t1", "content": "x".repeat(9000)},
                {"type": "tool_result", "tool_use_id": "t2", "content": format!("SECRET_PAYLOAD_BODY_{}", "a".repeat(6000))},
                {"type": "tool_result", "tool_use_id": "t3", "content": format!("RAW_HTTP_BODY_{}", "b".repeat(1500))}
            ]}
        }),
    ];
    fs::write(
        claude_proj.join("session.jsonl"),
        claude_lines
            .iter()
            .map(|v| v.to_string() + "\n")
            .collect::<String>(),
    )
    .unwrap();

    // Codex transcript: 1 covered (`git status`) + 1 uncovered (`docker ps`).
    let codex_lines = [
        json!({
            "timestamp": ts,
            "type": "response_item",
            "payload": {"type": "function_call", "call_id": "c1", "name": "exec_command", "arguments": "{\"cmd\":\"git status\"}"}
        }),
        json!({
            "timestamp": ts,
            "type": "response_item",
            "payload": {"type": "function_call_output", "call_id": "c1", "output": "On branch main\n"}
        }),
        json!({
            "timestamp": ts,
            "type": "response_item",
            "payload": {"type": "function_call", "call_id": "c2", "name": "exec_command", "arguments": "{\"cmd\":\"docker ps --filter name=PRIVATE_CONTAINER_99\"}"}
        }),
        json!({
            "timestamp": ts,
            "type": "response_item",
            "payload": {"type": "function_call_output", "call_id": "c2", "output": format!("CONTAINER_ROW_{}", "c".repeat(3200))}
        }),
    ];
    fs::write(
        codex_sess.join("rollout.jsonl"),
        codex_lines
            .iter()
            .map(|v| v.to_string() + "\n")
            .collect::<String>(),
    )
    .unwrap();

    let out = cargo_bin_cmd!("footnote")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .env("CLAUDE_CONFIG_DIR", home.path().join(".claude"))
        .env("CODEX_HOME", home.path().join(".codex"))
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["gain", "--missed", "--days", "7"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let raw_out = String::from_utf8(out.stdout).unwrap();
    let v: Value = serde_json::from_str(&raw_out).unwrap();
    let m = &v["missed"];
    assert_eq!(m["total_shell_calls"], 5, "{v}");
    assert_eq!(m["covered_calls"], 2, "{v}");
    assert_eq!(m["uncovered_calls"], 3, "{v}");

    let top = m["top_uncovered"].as_array().unwrap();
    assert_eq!(top.len(), 3, "{top:?}");
    assert_eq!(top[0]["prefix"], "gh api");
    assert_eq!(top[1]["prefix"], "docker ps");
    assert_eq!(top[2]["prefix"], "curl");

    // Privacy invariant: zero arguments, paths, headers, or raw payloads may appear in the report.
    for forbidden in [
        "LEAK_ARG_1",
        "LEAK_HEADER",
        "PRIVATE_CONTAINER_99",
        "SECRET_PAYLOAD_BODY",
        "RAW_HTTP_BODY",
        "CONTAINER_ROW",
        "/private/secret-repo",
        "internal.example",
    ] {
        assert!(
            !raw_out.contains(forbidden),
            "forbidden token `{forbidden}` leaked into gain --missed output:\n{raw_out}"
        );
    }
}
