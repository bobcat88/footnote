#![cfg(feature = "code")]
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path, time::Duration};

fn footnote(state: &Path, args: &[&str]) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .env("FOOTNOTE_POLICY_FILE", state.join("no-policy.json"))
        .args(args)
        .timeout(Duration::from_secs(300))
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

/// A two-function project: the graph must say `helper` is called by `run` on line 2, and flag the file once it changes.
#[test]
fn callers_and_callees_come_from_the_compiler_index() {
    if std::process::Command::new("rust-analyzer").arg("--version").output().is_err() {
        eprintln!("rust-analyzer not installed: skipped");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (state, project) = (dir.path().join("state"), dir.path().join("demo"));
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    let lib = "pub fn run() -> u32 {\n    helper() + 1\n}\n\nfn helper() -> u32 {\n    41\n}\n";
    fs::write(project.join("src/lib.rs"), lib).unwrap();
    let root = project.to_str().unwrap();

    let built = footnote(&state, &["code-index", root]);
    assert!(built["symbols"].as_u64().unwrap() >= 2, "{built}");

    let found = footnote(&state, &["code", "helper", "--root", root]);
    let helper = &found["symbols"][0];
    assert_eq!(helper["signature"], "fn helper() -> u32", "{found}");
    assert_eq!(helper["lines"], serde_json::json!([5, 7]), "{found}");
    assert_eq!(helper["callers"][0]["name"], "run", "{found}");
    assert_eq!(helper["callers"][0]["lines"], serde_json::json!([2]), "{found}");
    assert_eq!(footnote(&state, &["code", "run", "--root", root])["symbols"][0]["callees"][0]["name"], "helper");
    assert_eq!(found["stale"], serde_json::json!([]));

    fs::write(project.join("src/lib.rs"), format!("{lib}// edited\n")).unwrap();
    assert_eq!(footnote(&state, &["code", "helper", "--root", root])["stale"], serde_json::json!(["src/lib.rs"]));
}
