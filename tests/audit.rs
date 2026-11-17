//! Audit-log integration tests: chain integrity, MAC binding, tamper detection
//! and fail-closed behaviour. These drive the real binary.
use assert_cmd::Command;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Output,
};

const KEY_HEX: &str = "a1b2c3d4a1b2c3d4a1b2c3d4a1b2c3d4a1b2c3d4a1b2c3d4a1b2c3d4a1b2c3d4";
const WRONG_KEY_HEX: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

fn bin() -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
    // An empty state folder: without it, `audit verify` with no --mac-key picks up this machine's managed key (once any
    // daemon or MCP session has created one) and an unkeyed test log fails with "MAC mismatch".
    c.env(
        "FOOTNOTE_STATE_DIR",
        std::env::temp_dir().join(format!("footnote-audit-test-{}", std::process::id())),
    );
    c
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

fn key_file(t: &tempfile::TempDir, name: &str, hex: &str) -> PathBuf {
    let path = t.path().join(name);
    fs::write(&path, hex).unwrap();
    path
}

/// Run one MCP session with auditing enabled and return raw stdout.
fn serve_audited(db: &Path, audit: &Path, key: Option<&Path>, lines: &[Value]) -> (Output, String) {
    let input: String = lines
        .iter()
        .map(|v| format!("{v}\n"))
        .collect::<Vec<_>>()
        .concat();
    let mut cmd = bin();
    cmd.arg("--db")
        .arg(db)
        .args(["serve", "--mcp", "--audit"])
        .arg(audit);
    if let Some(k) = key {
        cmd.arg("--mac-key").arg(k);
    }
    let out = cmd.write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    (out, stdout)
}

fn two_calls() -> Vec<Value> {
    vec![
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"footnote_stats","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"footnote_recall","arguments":{"query":"checkpoint","limit":1}}}),
    ]
}

fn verify(audit: &Path, key: Option<&Path>, expect_last: Option<&str>) -> Output {
    let mut cmd = bin();
    cmd.args(["audit", "verify", "--file"]).arg(audit);
    if let Some(k) = key {
        cmd.arg("--mac-key").arg(k);
    }
    if let Some(head) = expect_last {
        cmd.arg("--expect-last").arg(head);
    }
    cmd.output().unwrap()
}

fn verify_ok(audit: &Path, key: Option<&Path>) -> Value {
    let out = verify(audit, key, None);
    assert!(out.status.success(), "verify failed: {out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}

fn audit_lines(audit: &Path) -> Vec<Value> {
    fs::read_to_string(audit)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn every_tool_call_is_logged_chained_and_bound_to_its_response() {
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    let key = key_file(&t, "key.txt", KEY_HEX);

    let (out, stdout) = serve_audited(&db, &audit, Some(&key), &two_calls());
    assert!(out.status.success(), "serve failed: {out:?}");

    // Responses carry a _meta.footnote binding to the audit line.
    let mut seqs = Vec::new();
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let response: Value = serde_json::from_str(line).unwrap();
        let meta = &response["result"]["_meta"]["footnote"];
        assert_eq!(meta["mac"].as_str().map(|m| m.len()), Some(64));
        seqs.push(meta["seq"].as_u64().unwrap());
    }
    assert_eq!(seqs, vec![1, 2]);

    let entries = audit_lines(&audit);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["tool"], "footnote_stats");
    assert_eq!(entries[1]["tool"], "footnote_recall");
    // Queries are hashed, never stored verbatim.
    assert!(entries[1]["args_sha256"].as_str().unwrap().len() == 64);
    assert!(!fs::read_to_string(&audit).unwrap().contains("checkpoint"));

    let report = verify_ok(&audit, Some(&key));
    assert_eq!(report["entries"], 2);
    assert_eq!(report["mac"], "verified");
    assert_eq!(report["head"], "not_checked");
}

#[test]
fn a_modified_entry_breaks_the_chain() {
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    serve_audited(&db, &audit, None, &two_calls());
    let original = fs::read_to_string(&audit).unwrap();
    fs::write(&audit, original.replace("\"ok\":true", "\"ok\":false")).unwrap();

    let out = verify(&audit, None, None);
    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("hash mismatch"), "{stdout}");
}

#[test]
fn a_wrong_mac_key_is_rejected() {
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    let key = key_file(&t, "key.txt", KEY_HEX);
    let wrong = key_file(&t, "wrong.txt", WRONG_KEY_HEX);
    serve_audited(&db, &audit, Some(&key), &two_calls());

    let out = verify(&audit, Some(&wrong), None);
    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("MAC mismatch"), "{stdout}");
}

#[test]
fn dropping_an_entry_is_detected_as_a_sequence_gap() {
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    serve_audited(&db, &audit, None, &two_calls());
    let lines = fs::read_to_string(&audit).unwrap();
    let second = lines.lines().nth(1).unwrap();
    fs::write(&audit, format!("{second}\n")).unwrap();

    let out = verify(&audit, None, None);
    assert!(!out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("sequence gap"), "{stdout}");
}

/// Documents the known limit of a bare hash chain: truncating the tail still
/// verifies, so the head must be pinned out of band.
#[test]
fn tail_truncation_only_shows_up_against_a_pinned_head() {
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    serve_audited(&db, &audit, None, &two_calls());
    let report = verify_ok(&audit, None);
    let pinned = report["last_hash"].as_str().unwrap().to_string();

    let content = fs::read_to_string(&audit).unwrap();
    let first = content.lines().next().unwrap();
    fs::write(&audit, format!("{first}\n")).unwrap();

    // Without a pinned head the shortened log looks valid.
    let unpinned = verify(&audit, None, None);
    assert!(unpinned.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&unpinned.stdout).unwrap()["entries"],
        1
    );

    // With the pinned head it fails closed.
    let pinned_out = verify(&audit, None, Some(&pinned));
    assert!(!pinned_out.status.success());
    let stdout = String::from_utf8(pinned_out.stdout).unwrap();
    assert!(stdout.contains("audit head mismatch"), "{stdout}");
}

#[test]
fn a_corrupt_log_refuses_to_serve() {
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    serve_audited(&db, &audit, None, &two_calls());
    let content = fs::read_to_string(&audit).unwrap();
    fs::write(&audit, content.replace("\"ok\":true", "\"ok\":false")).unwrap();

    let out = bin()
        .arg("--db")
        .arg(&db)
        .args(["serve", "--mcp", "--audit"])
        .arg(&audit)
        .write_stdin("")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(stderr.contains("refusing to serve"), "{stderr}");
}

#[test]
fn a_mac_key_without_an_audit_log_is_rejected() {
    let t = tmp();
    let db = fixture(&t);
    let key = key_file(&t, "key.txt", KEY_HEX);
    let out = bin()
        .arg("--db")
        .arg(&db)
        .args(["serve", "--mcp", "--mac-key"])
        .arg(&key)
        .write_stdin("")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(stderr.contains("--mac-key requires --audit"), "{stderr}");
}

#[test]
fn two_sessions_open_at_once_keep_one_chain() {
    // Found 2026-11-15: each server kept the head it read at start, so the second session to write forked the chain (two lines with
    // the same seq) and every later start refused to serve.
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command as Std, Stdio};
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    let start = || {
        let mut c = Std::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", t.path().join("state"))
            .arg("--db")
            .arg(&db)
            .args(["serve", "--mcp", "--audit"])
            .arg(&audit)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        (child, out)
    };
    let (mut a, mut a_out) = start();
    let (mut b, mut b_out) = start();
    let call = |child: &mut std::process::Child,
                out: &mut BufReader<std::process::ChildStdout>,
                id: u64| {
        let msg = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"footnote_stats","arguments":{}}});
        writeln!(child.stdin.as_mut().unwrap(), "{msg}").unwrap();
        let mut line = String::new();
        out.read_line(&mut line).unwrap();
        assert!(line.contains("\"result\""), "call failed: {line}");
    };
    call(&mut a, &mut a_out, 1);
    call(&mut b, &mut b_out, 1);
    call(&mut a, &mut a_out, 2);
    call(&mut b, &mut b_out, 2);
    drop(a.stdin.take());
    drop(b.stdin.take());
    a.wait().unwrap();
    b.wait().unwrap();
    let report = verify_ok(&audit, None);
    let seqs: Vec<u64> = audit_lines(&audit)
        .iter()
        .map(|l| l["seq"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, vec![1, 2, 3, 4], "{report}");
}

/// #167: a server that starts while another session holds the audit log's lock (appending) waits for it, instead of reading a
/// locked file, failing verification and refusing to serve.
#[test]
fn a_server_starting_during_an_append_waits_instead_of_refusing() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command as Std, Stdio};
    let t = tmp();
    let db = fixture(&t);
    let audit = t.path().join("audit.jsonl");
    let state = t.path().join("state");
    let serve = || {
        let mut c = Std::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", &state)
            .arg("--db")
            .arg(&db)
            .args(["serve", "--mcp", "--audit"])
            .arg(&audit);
        c
    };
    // One call first, so the log exists and has a chain to verify.
    let msg = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"footnote_stats","arguments":{}}});
    let first = serve()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let out = first.wait_with_output_after(&format!("{msg}\n"));
    assert!(out.contains("\"result\""), "{out}");

    // Hold the append lock, start a second server, release after 300 ms: it must answer.
    let held = std::fs::OpenOptions::new()
        .read(true)
        .append(true)
        .open(&audit)
        .unwrap();
    held.lock().unwrap();
    let mut child = serve()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    held.unlock().unwrap();
    writeln!(child.stdin.as_mut().unwrap(), "{msg}").unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    drop(child.stdin.take());
    let status = child.wait_with_output().unwrap();
    assert!(
        line.contains("\"result\""),
        "server refused: {line} {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

trait WaitAfter {
    fn wait_with_output_after(self, input: &str) -> String;
}
impl WaitAfter for std::process::Child {
    fn wait_with_output_after(mut self, input: &str) -> String {
        use std::io::Write;
        self.stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        String::from_utf8_lossy(&self.wait_with_output().unwrap().stdout).to_string()
    }
}
