//! Story #148: Footnote's own lean search results are measured on each agent call and shown by `footnote usage`.
use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn each_agent_search_records_its_saving_and_usage_sums_it() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path().join("state");
    let footnote = || {
        let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", &state);
        c
    };
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("wal.md"),
        "# WAL\nCheckpoint latency is amortized by batching writes.\n",
    )
    .unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    footnote()
        .arg("--db")
        .arg(&db)
        .args(["index", "--root", r, "--allow", r])
        .assert()
        .success();
    let search = || {
        let call = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"footnote_recall","arguments":{"query":"checkpoint"}}});
        footnote()
            .arg("--db")
            .arg(&db)
            .arg("mcp")
            .write_stdin(format!("{call}\n"))
            .assert()
            .success();
    };
    search();
    let log = state.join("usage").join("lean.jsonl");
    let line: Value =
        serde_json::from_str(fs::read_to_string(&log).unwrap().lines().next().unwrap()).unwrap();
    assert!(
        line["before"].as_u64().unwrap() > line["after"].as_u64().unwrap(),
        "{line}"
    );
    assert!(
        !line.to_string().contains("Checkpoint"),
        "never any text: {line}"
    );

    let usage: Value = serde_json::from_slice(
        &footnote()
            .args(["usage", "--days", "1"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let f = &usage["measured_savings"]["footnote"];
    assert_eq!(f["calls"], 1, "{usage}");
    assert!(f["saved_bytes"].as_u64().unwrap() > 0, "{usage}");

    // Switched off: nothing more is recorded.
    footnote()
        .args(["config", "set", "usage.lean_log", "false"])
        .assert()
        .success();
    search();
    assert_eq!(fs::read_to_string(&log).unwrap().lines().count(), 1);
}
