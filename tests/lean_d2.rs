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
        .args(["hook", "lean"])
        .write_stdin(payload.to_string())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["hookSpecificOutput"]["updatedInput"]["command"],
        "footnote run -- cargo test --workspace"
    );
    assert_eq!(v["hookSpecificOutput"]["updatedInput"]["timeout"], 120000);
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "allow");

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
}
