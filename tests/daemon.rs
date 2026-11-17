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
    // Windows registers a logon task (dry run: nothing created); elsewhere registration is manual for now.
    let expected = if cfg!(windows) {
        "dry-run"
    } else {
        "manual on this platform"
    };
    assert_eq!(report["daemon"]["status"], expected);
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

/// The memory hook's per-prompt recall through a daemon started on demand, which leaves by itself once idle.
/// The daemon's stdio is null, so it cannot hold the harness's pipes open (see the module note).
#[test]
fn the_hook_recalls_through_a_resident_daemon_that_leaves_when_idle() {
    use std::process::{Command as Std, Stdio};
    let t = tmp();
    let db = t.path().join("memory.db");
    let state = t.path().join("state");
    let memory = serde_json::json!([{"text": "Johan wants relative % gaps in every benchmark table.", "kind": "preference",
                                     "scope": "global", "observed_at": "2026-11-15"}]);
    bin()
        .arg("--db")
        .arg(&db)
        .arg("remember")
        .write_stdin(memory.to_string())
        .assert()
        .success();
    let recall = |extra: &[&str]| {
        bin()
            .env("FOOTNOTE_STATE_DIR", &state)
            .arg("--db")
            .arg(&db)
            .args(["memory-recall", "--resident"])
            .args(extra)
            .args(["--", "benchmark table gaps"])
            .output()
            .unwrap()
    };
    let answered =
        |o: &std::process::Output| !String::from_utf8_lossy(&o.stdout).contains("\"error\"");
    // No daemon: --resident fails, so the hook falls back to its own search.
    assert!(!answered(&recall(&[])));

    let mut daemon = Std::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", &state)
        .arg("--db")
        .arg(&db)
        .args([
            "serve",
            "--mcp",
            "--daemon",
            "--no-audit",
            "--idle-seconds",
            "2",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut out = recall(&[]);
    for _ in 0..50 {
        if answered(&out) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        out = recall(&[]);
    }
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{out:?}"));
    assert!(v["memories"].to_string().contains("relative % gaps"), "{v}");

    // Idle with no session open: the daemon exits on its own.
    let mut gone = None;
    for _ in 0..100 {
        gone = daemon.try_wait().unwrap();
        if gone.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if gone.is_none() {
        daemon.kill().unwrap();
        panic!("daemon still running after its idle time");
    }
}

/// An agent's relay outlives its daemon: once the daemon stops (an install replacing the binary, a crash), the
/// next call is answered in-process instead of the agent losing its memory tools until it reconnects.
#[test]
fn an_agent_keeps_its_tools_when_the_daemon_stops() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command as Std, Stdio};
    let t = tmp();
    let db = t.path().join("memory.db");
    let state = t.path().join("state");
    let memory = serde_json::json!([{"text": "The relay falls back in-process.", "kind": "fact",
                                     "scope": "global", "observed_at": "2026-11-17"}]);
    bin()
        .arg("--db")
        .arg(&db)
        .arg("remember")
        .write_stdin(memory.to_string())
        .assert()
        .success();
    let footnote = || {
        let mut c = Std::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", &state).arg("--db").arg(&db);
        c
    };
    let mut daemon = footnote()
        .args([
            "serve",
            "--mcp",
            "--daemon",
            "--no-audit",
            "--idle-seconds",
            "30",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // Wait until the daemon answers, so the relay below really connects to it.
    let up = || {
        let o = footnote()
            .args(["memory-recall", "--resident", "--", "relay"])
            .output()
            .unwrap();
        !String::from_utf8_lossy(&o.stdout).contains("\"error\"")
    };
    assert!((0..50).any(|_| up() || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        false
    }));

    let mut relay = footnote()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = relay.stdin.take().unwrap();
    let mut output = BufReader::new(relay.stdout.take().unwrap());
    let mut call = |id: u64| {
        let req = serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/list"});
        writeln!(input, "{req}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let v: Value =
            serde_json::from_str(&line).unwrap_or_else(|_| panic!("reply {id}: {line:?}"));
        assert_eq!(v["id"], id, "{v}");
        assert!(v["result"]["tools"].is_array(), "{v}");
    };
    call(1);
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    call(2);
    call(3);
    drop(call);
    drop(input);
    let done = relay.wait_with_output().unwrap();
    assert!(done.status.success(), "{done:?}");
    assert!(
        String::from_utf8_lossy(&done.stderr).contains("serving in-process"),
        "{done:?}"
    );
}
