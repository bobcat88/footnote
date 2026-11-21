use assert_cmd::Command;
use serde_json::Value;
use std::fs;

fn bin() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin!("footnote").to_path_buf()
}

#[test]
fn grep_honors_gitignore_annotates_symbols_and_spills_overflow_to_handle() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let repo = dir.path().join("repo");
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::create_dir_all(repo.join("ignored_dir")).unwrap();
    fs::create_dir_all(&state).unwrap();

    fs::write(repo.join(".gitignore"), "ignored_dir/\n*.log\n").unwrap();
    fs::write(
        repo.join("src/lib.rs"),
        "pub fn compute_score(x: i32) -> i32 {\n    let magic_marker = x * 2;\n    magic_marker + 1\n}\n\npub fn other_fn() -> i32 {\n    let magic_marker = 7;\n    magic_marker\n}\n",
    )
    .unwrap();
    fs::write(repo.join("ignored_dir/skip.rs"), "let magic_marker = 99;\n").unwrap();
    fs::write(repo.join("debug.log"), "magic_marker in ignored log\n").unwrap();
    // Binary file containing NUL byte must be skipped
    fs::write(repo.join("src/blob.bin"), b"\x00magic_marker\x00").unwrap();

    // Index with Footnote Code so `footnote grep` annotates matches with enclosing symbols
    Command::new(bin())
        .env("FOOTNOTE_STATE_DIR", &state)
        .args(["code-index"])
        .arg(&repo)
        .assert()
        .success();

    // 1. JSON mode with symbol annotations & .gitignore pruning
    let out = Command::new(bin())
        .env("FOOTNOTE_STATE_DIR", &state)
        .args(["grep", "magic_marker"])
        .arg(&repo)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["total_matches"], 4);
    assert_eq!(v["total_files"], 1);
    assert_eq!(v["annotated_symbols"], 4);
    assert_eq!(v["truncated"], false);
    let first = &v["matches"][0];
    assert_eq!(first["file"], "src/lib.rs");
    assert_eq!(first["line"], 2);
    assert!(
        first["symbol"]
            .as_str()
            .is_some_and(|s| s.contains("compute_score")),
        "expected enclosing symbol compute_score, got {first}"
    );

    // 2. Compact text mode with --limit 2 spills full output to `footnote output <id>`
    let out2 = Command::new(bin())
        .env("FOOTNOTE_STATE_DIR", &state)
        .args(["grep", "magic_marker"])
        .arg(&repo)
        .args(["--limit", "2"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out2).unwrap();
    assert!(
        !text.trim_end().ends_with("null"),
        "unexpected trailing null: {text}"
    );
    assert!(text.contains("src/lib.rs (2)"), "{text}");
    assert!(text.contains("[fn compute_score"), "{text}");
    assert!(text.contains("truncated (2/4 matches"), "{text}");
    // Extract handle id and verify `footnote output <id> --grep other_fn` retrieves line 7!
    let handle = text
        .split('`')
        .find(|s| s.starts_with("h-"))
        .expect("handle id in truncated notice");
    let retrieved = Command::new(bin())
        .env("FOOTNOTE_STATE_DIR", &state)
        .args(["output", handle, "--grep", "other_fn"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rtext = String::from_utf8(retrieved).unwrap();
    assert!(rtext.contains("other_fn"), "{rtext}");

    // 3. Fail-closed secret guard: matches containing credential-like tokens are never persisted to handle
    fs::write(
        repo.join("src/secret.txt"),
        "a=1\nb=2\ntoken=ghp_1234567890abcdefghijklmnopqrstuvwxyzABCD\n",
    )
    .unwrap();
    let out3 = Command::new(bin())
        .env("FOOTNOTE_STATE_DIR", &state)
        .args(["grep", "="])
        .arg(&repo)
        .args(["--limit", "1", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v3: Value = serde_json::from_slice(&out3).unwrap();
    assert_eq!(v3["truncated"], true);
    assert_eq!(v3["secret_redacted"], true);
    assert!(v3["handle"].is_null());
}
