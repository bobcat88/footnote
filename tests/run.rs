//! Integration tests for `footnote run` and `footnote output` (#243).
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

#[test]
fn run_preserves_exit_code_and_shortens_with_reversible_handle() {
    let state = TempDir::new().unwrap();
    // Feed noisy cargo-test style output through --stdin to verify shortening, handle storage, and grep retrieval.
    let mut noisy = String::new();
    for i in 0..45 {
        noisy.push_str(&format!("   Compiling dep-{i} v0.1.0\n"));
    }
    for i in 0..35 {
        noisy.push_str(&format!("test unit::case_{i} ... ok\n"));
    }
    noisy.push_str(
        "test unit::broken ... FAILED\n\nfailures:\n\n---- unit::broken stdout ----\nassertion `left == right` failed\n  left: 10\n right: 20\n\nfailures:\n    unit::broken\n\ntest result: FAILED. 35 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n",
    );

    let out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["run", "--stdin", "--family", "cargo-test"])
        .write_stdin(noisy.clone())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.len() < noisy.len() / 2,
        "expected >50% shrink:\n{text}"
    );
    assert!(text.contains("unit::broken ... FAILED"), "{text}");
    assert!(text.contains("left: 10"), "{text}");
    assert!(text.contains("footnote output h-"), "{text}");

    // Extract handle id and verify `footnote output <id> --grep` retrieves the original line.
    let handle = text
        .split("footnote output ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();

    let retrieved = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["output", &handle, "--grep", "case_17"])
        .output()
        .unwrap();
    assert!(retrieved.status.success(), "{retrieved:?}");
    let v: Value = serde_json::from_slice(&retrieved.stdout).unwrap();
    assert_eq!(v["returned_lines"], 1, "{v}");
    assert!(
        v["content"]
            .as_str()
            .unwrap()
            .contains("test unit::case_17 ... ok")
    );

    // Verify usage/lean.jsonl recorded both the `run` saving and the `output` re-retrieval miss.
    let lean = fs::read_to_string(state.path().join("usage/lean.jsonl")).unwrap();
    assert!(
        lean.contains("\"tool\":\"run\"") && lean.contains("\"family\":\"cargo-test\""),
        "{lean}"
    );
    assert!(
        lean.contains("\"tool\":\"output\"") && lean.contains("\"miss\":true"),
        "{lean}"
    );
}

#[test]
fn output_containing_credentials_is_never_stored() {
    let state = TempDir::new().unwrap();
    let mut secret_output = String::new();
    for i in 0..50 {
        secret_output.push_str(&format!("   Compiling pkg-{i} v1.0.0\n"));
    }
    secret_output.push_str("error: auth failed with token ghp_1234567890abcdef1234567890abcdef\n");

    let out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["run", "--stdin", "--family", "cargo-build"])
        .write_stdin(secret_output.clone())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("potential credentials detected"),
        "expected credential refusal note: {text}"
    );

    let outputs_dir = state.path().join("outputs");
    let stored_files: Vec<_> = if outputs_dir.exists() {
        fs::read_dir(&outputs_dir).unwrap().flatten().collect()
    } else {
        vec![]
    };
    assert!(
        stored_files.is_empty(),
        "credential-bearing output must never be written to outputs/: {stored_files:?}"
    );
}

#[test]
fn run_never_emits_more_than_raw_and_propagates_nonzero_exit() {
    let state = TempDir::new().unwrap();
    let py = if cfg!(windows) { "python" } else { "python3" };
    let out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args([
            "run",
            "--",
            py,
            "-c",
            "import sys; print('hi'); sys.exit(7)",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hi");
    assert!(!state.path().join("outputs").exists());
}

#[test]
fn output_rejects_path_traversal() {
    let state = TempDir::new().unwrap();
    let out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["output", "../memory.db"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("invalid output handle id"), "{text}");
}

#[cfg(feature = "code")]
#[test]
fn failing_diagnostic_links_to_code_graph_symbol() {
    let state = TempDir::new().unwrap();
    let proj = TempDir::new().unwrap();
    fs::create_dir_all(proj.path().join("src")).unwrap();
    fs::write(
        proj.path().join("src/lib.rs"),
        "pub fn compute_total(a: i32, b: i32) -> i32 {\n    a + b\n}\npub fn caller() -> i32 {\n    compute_total(1, 2)\n}\n",
    )
    .unwrap();

    let idx = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["code-index", proj.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(idx.status.success(), "{idx:?}");

    let mut diag = String::new();
    for i in 0..45 {
        diag.push_str(&format!("   Compiling dep-{i} v0.1.0\n"));
    }
    diag.push_str(
        "error[E0308]: mismatched types\n --> src/lib.rs:2:5\n  |\n2 |     a + b\n  |     ^^^^^\nerror: could not compile `demo`\n",
    );

    let out = cargo_bin_cmd!("footnote")
        .current_dir(proj.path())
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["run", "--stdin", "--family", "cargo-build"])
        .write_stdin(diag)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("code graph:") && text.contains("compute_total"),
        "expected code graph symbol link in:\n{text}"
    );
}

#[test]
fn uniform_json_array_compacts_over_40_percent_and_keeps_full_handle() {
    let state = TempDir::new().unwrap();
    let items: Vec<Value> = (1..=18)
        .map(|i| {
            serde_json::json!({
                "number": 300 + i,
                "title": format!("Issue {i}: improve shell output compaction and diagnostics"),
                "state": if i % 5 == 0 { "CLOSED" } else { "OPEN" },
                "url": format!("https://github.com/example/repo/issues/{}", 300 + i),
                "labels": [{"name": "story"}, {"name": "usage"}],
                "author": {"login": "dev"},
                "body": format!("Detailed context for item {i}: {}", "benchmark payload ".repeat(14)),
            })
        })
        .collect();
    let raw = serde_json::to_string_pretty(&items).unwrap();
    assert!(raw.len() > 4000, "fixture should exceed 2KB");

    let out = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["run", "--stdin"])
        .write_stdin(raw.clone())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.len() * 100 <= raw.len() * 60,
        "expected >=40% reduction (raw={}B, out={}B):\n{text}",
        raw.len(),
        text.len()
    );
    assert!(text.contains("json array: 18 items"), "{text}");
    assert!(text.contains("state: CLOSED=3, OPEN=15"), "{text}");
    assert!(text.contains("json-array"), "{text}");

    let handle = text
        .split("footnote output ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();
    let retrieved = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state.path())
        .args(["output", &handle, "--grep", "Issue 12:"])
        .output()
        .unwrap();
    assert!(retrieved.status.success(), "{retrieved:?}");
    let v: Value = serde_json::from_slice(&retrieved.stdout).unwrap();
    assert_eq!(v["returned_lines"], 1, "{v}");

    // Fail-closed secret guard: JSON array with credential-like token is never stored or compacted.
    let secret_raw = raw.replace(
        "Detailed context for item 1:",
        "token=ghp_1234567890abcdef1234567890abcdef",
    );
    let state2 = TempDir::new().unwrap();
    let out_secret = cargo_bin_cmd!("footnote")
        .env("FOOTNOTE_STATE_DIR", state2.path())
        .args(["run", "--stdin"])
        .write_stdin(secret_raw.clone())
        .output()
        .unwrap();
    assert!(out_secret.status.success());
    assert!(!state2.path().join("outputs").exists());
}

#[test]
fn repeat_run_delta_collapses_identical_failures_and_resets_on_change_or_pass() {
    let state = TempDir::new().unwrap();
    let proj = TempDir::new().unwrap();
    let py = if cfg!(windows) { "python" } else { "python3" };

    // A small runner script controlled by an env var so the exact `(cwd, command)` stays identical across turns.
    let script = proj.path().join("sim_test.py");
    fs::write(
        &script,
        r#"import os, sys
mode = os.environ.get("SIM_MODE", "fail1")
dur = os.environ.get("SIM_DUR", "0.42s")
if mode == "pass":
    for i in range(30):
        print(f"test suite::ok_{i} ... ok")
    print(f"test result: ok. 30 passed; 0 failed; 0 ignored; finished in {dur}")
    sys.exit(0)
for i in range(40):
    print(f"   Compiling dep-{i} v0.1.0")
for i in range(25):
    print(f"test suite::ok_{i} ... ok")
val = "99" if mode == "fail2" else "10"
print("test suite::broken ... FAILED")
print("")
print("failures:")
print("---- suite::broken stdout ----")
print("thread 'suite::broken' panicked at src/lib.rs:42:5:")
print("assertion `left == right` failed")
print(f"  left: {val}")
print(" right: 20")
print("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace")
print("")
print("failures:")
print("    suite::broken")
print("")
print(f"test result: FAILED. 25 passed; 1 failed; 0 ignored; finished in {dur}")
sys.exit(101)
"#,
    )
    .unwrap();

    let run_sim = |mode: &str, dur: &str| {
        cargo_bin_cmd!("footnote")
            .current_dir(proj.path())
            .env("FOOTNOTE_STATE_DIR", state.path())
            .env("SIM_MODE", mode)
            .env("SIM_DUR", dur)
            .args(["run", "--family", "cargo-test", "--", py])
            .arg(&script)
            .output()
            .unwrap()
    };

    // Run 1: full preservation-first failure diagnostics + handle.
    let r1 = run_sim("fail1", "0.42s");
    assert_eq!(r1.status.code(), Some(101));
    let t1 = String::from_utf8(r1.stdout).unwrap();
    assert!(t1.contains("left: 10"), "{t1}");
    let h1 = t1
        .split("footnote output ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();

    // Run 2: identical failure (different duration) -> 3-line repeat delta referencing prior handle, same exit code.
    let r2 = run_sim("fail1", "0.87s");
    assert_eq!(r2.status.code(), Some(101));
    let t2 = String::from_utf8(r2.stdout).unwrap();
    assert!(
        t2.contains("repeat failure (#2 identical in session, exit 101)"),
        "{t2}"
    );
    assert!(t2.contains(&format!("footnote output {h1}")), "{t2}");
    assert_eq!(t2.lines().count(), 3, "{t2}");
    assert!(t2.len() < t1.len(), "repeat delta should shrink vs run 1");

    // Run 3: changed failure line (`left: 99`) -> must emit full diagnostics, never suppress!
    let r3 = run_sim("fail2", "0.51s");
    assert_eq!(r3.status.code(), Some(101));
    let t3 = String::from_utf8(r3.stdout).unwrap();
    assert!(!t3.contains("repeat failure"), "{t3}");
    assert!(t3.contains("left: 99"), "{t3}");

    // Run 4: passing run clears repeat state.
    let r4 = run_sim("pass", "0.30s");
    assert_eq!(r4.status.code(), Some(0));

    // Run 5: regression with `fail2` after pass -> full diagnostics again!
    let r5 = run_sim("fail2", "0.33s");
    assert_eq!(r5.status.code(), Some(101));
    let t5 = String::from_utf8(r5.stdout).unwrap();
    assert!(!t5.contains("repeat failure"), "{t5}");
    assert!(t5.contains("left: 99"), "{t5}");
}
