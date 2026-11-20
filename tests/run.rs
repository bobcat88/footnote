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
