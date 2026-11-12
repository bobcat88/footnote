//! End-to-end checks for the zero-configuration agent path.
//!
//! These deliberately avoid starting a child daemon: on Windows a spawned child
//! inherits the test binary's stdout pipe and keeps it open, which hangs the test
//! harness. The shared-daemon protocol is covered by unit tests inside
//! `src/daemon.rs`, which bind the listener in-process.
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::PathBuf};

fn bin() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
}

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap()
}

fn fixture(t: &tempfile::TempDir) -> PathBuf {
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("sqlite.md"),
        "# SQLite\n## WAL\nCheckpoint latency is amortized by batching writes.\n",
    )
    .unwrap();
    let db = t.path().join("k.db");
    bin()
        .arg("--db")
        .arg(&db)
        .args(["index", "--root"])
        .arg(&root)
        .arg("--allow")
        .arg(&root)
        .assert()
        .success();
    db
}

#[test]
fn an_agent_gets_working_tools_and_automatic_security_state() {
    let t = tmp();
    let db = fixture(&t);
    let state = t.path().join("state");

    let input = format!(
        "{}\n{}\n",
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"footnote_stats","arguments":{}}}),
    );
    let out = bin()
        .env("FOOTNOTE_STATE_DIR", &state)
        .arg("--db")
        .arg(&db)
        .arg("mcp")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("serverInfo"), "{stdout}");
    assert!(
        stdout.contains("_meta"),
        "tool result not audit-bound: {stdout}"
    );

    // Security material appears without any flag or key file being supplied.
    for name in ["audit.jsonl", "audit.key"] {
        assert!(state.join(name).exists(), "missing automatic {name}");
    }
    // And the automatic log verifies against the automatic key.
    let verified = bin()
        .args(["audit", "verify", "--file"])
        .arg(state.join("audit.jsonl"))
        .arg("--mac-key")
        .arg(state.join("audit.key"))
        .output()
        .unwrap();
    assert!(verified.status.success(), "{verified:?}");
    let report: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(report["entries"], 1);
    assert_eq!(report["mac"], "verified");
}

#[test]
fn install_dry_run_writes_nothing() {
    let t = tmp();
    let db = fixture(&t);
    let home = t.path().join("home");
    fs::create_dir_all(home.join(".codex")).unwrap();

    let out = bin()
        .env("USERPROFILE", &home)
        .env("HOME", &home)
        .arg("--db")
        .arg(&db)
        .args(["install", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["daemon"]["status"], "dry-run");
    assert!(
        !home.join(".codex/config.toml").exists(),
        "dry run wrote an agent config"
    );
}

/// The install path really writes config, so the user does not hand-edit files.
#[test]
fn install_registers_detected_agents() {
    let t = tmp();
    let db = fixture(&t);
    let home = t.path().join("home");
    fs::create_dir_all(home.join(".codex")).unwrap();
    fs::create_dir_all(home.join(".gemini")).unwrap();

    let out = bin()
        .env("USERPROFILE", &home)
        .env("HOME", &home)
        .arg("--db")
        .arg(&db)
        .arg("install")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");

    let codex = fs::read_to_string(home.join(".codex/config.toml")).unwrap();
    assert!(codex.contains("[mcp_servers.footnote]"), "{codex}");
    assert!(codex.contains("mcp"), "{codex}");

    let gemini: Value =
        serde_json::from_str(&fs::read_to_string(home.join(".gemini/settings.json")).unwrap())
            .unwrap();
    assert_eq!(gemini["mcpServers"]["footnote"]["args"][2], "mcp");
}
