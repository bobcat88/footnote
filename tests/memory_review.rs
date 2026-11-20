//! Goal 11: `memory-review` lists memories worth a second look and changes nothing; `forget` retires one (kept as history) and
//! `memory-restore` brings it back.
use assert_cmd::Command;
use serde_json::Value;

#[test]
fn review_lists_candidates_and_every_choice_is_reversible() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path().join("state");
    let db = state.join("memory.db");
    std::fs::create_dir_all(&state).unwrap();
    let footnote = || {
        let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", &state);
        c
    };
    let run = |args: &[&str]| -> Value {
        let out = footnote().args(args).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let memories = serde_json::json!([
        {"text": "Prune branches after their PR is merged, locally and on GitHub.", "kind": "preference", "scope": "global", "observed_at": "2026-11-10"},
        {"text": "Prune branches after their PR is merged locally and on GitHub", "kind": "preference", "scope": "global", "observed_at": "2026-11-17"},
        {"text": "Session of 2 October: the crawler was rewritten; tests left to write.", "kind": "session", "scope": "footnote", "observed_at": "2026-10-02"},
        {"text": "The daemon falls back to serving in-process when the relay loses it.", "kind": "fix", "scope": "footnote", "observed_at": "2026-11-17"}
    ]);
    footnote()
        .arg("--db")
        .arg(&db)
        .arg("remember")
        .write_stdin(memories.to_string())
        .assert()
        .success();

    let v = run(&["memory-review"]);
    assert_eq!(v["counts"]["memories"], 4, "{v}");
    assert_eq!(v["duplicates"].as_array().unwrap().len(), 1, "{v}");
    let older = v["duplicates"][0]["older"]["id"].as_i64().unwrap();
    assert!(
        v["old_sessions"]
            .to_string()
            .contains("crawler was rewritten"),
        "{v}"
    );
    // Reviewing changes nothing.
    assert_eq!(run(&["memory-review"])["counts"]["memories"], 4);

    // Forget the older copy: it leaves the review and recall, and is kept as history.
    run(&["--db", db.to_str().unwrap(), "forget", &older.to_string()]);
    let v = run(&["memory-review"]);
    assert_eq!(
        (
            v["counts"]["memories"].as_u64(),
            v["duplicates"].as_array().unwrap().len()
        ),
        (Some(3), 0),
        "{v}"
    );

    // Restore undoes it.
    let r = run(&[
        "--db",
        db.to_str().unwrap(),
        "memory-restore",
        &older.to_string(),
    ]);
    assert_eq!(r["was"], "deleted", "{r}");
    assert_eq!(run(&["memory-review"])["counts"]["memories"], 4);
    let found = run(&[
        "--db",
        db.to_str().unwrap(),
        "memory-recall",
        "--",
        "branches pruned",
    ]);
    assert!(
        found["memories"]
            .to_string()
            .contains(&format!("\"id\":{older}")),
        "{found}"
    );
    // Restoring a live memory is refused.
    assert!(
        !footnote()
            .args([
                "--db",
                db.to_str().unwrap(),
                "memory-restore",
                &older.to_string()
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
}
