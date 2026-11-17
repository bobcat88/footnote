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
    if std::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("rust-analyzer not installed: skipped");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (state, project) = (dir.path().join("state"), dir.path().join("demo"));
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
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
    assert_eq!(
        helper["callers"][0]["lines"],
        serde_json::json!([2]),
        "{found}"
    );
    assert_eq!(
        footnote(&state, &["code", "run", "--root", root])["symbols"][0]["callees"][0]["name"],
        "helper"
    );
    assert_eq!(found["stale"], serde_json::json!([]));

    // The same answer reaches agents through the MCP tool.
    let call = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "footnote_code", "arguments": {"name": "helper", "root": root}}});
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", &state)
        .env("FOOTNOTE_POLICY_FILE", state.join("no-policy.json"))
        .args([
            "--db",
            state.join("kb.db").to_str().unwrap(),
            "serve",
            "--mcp",
        ])
        .write_stdin(format!("{call}\n"))
        .output()
        .unwrap();
    let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
    let text: Value =
        serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text["symbols"][0]["callers"][0]["name"], "run", "{reply}");

    fs::write(project.join("src/lib.rs"), format!("{lib}// edited\n")).unwrap();
    assert_eq!(
        footnote(&state, &["code", "helper", "--root", root])["stale"],
        serde_json::json!(["src/lib.rs"])
    );
}

/// Two crates and one package outside the indexed folder: the map gives each crate as a block, the link from `b` to `a`
/// counted in references, and `ext` as an outside package used by `b`.
#[test]
fn the_architecture_map_counts_blocks_links_and_outside_packages() {
    if std::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("rust-analyzer not installed: skipped");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (state, root, ext) = (
        dir.path().join("state"),
        dir.path().join("ws"),
        dir.path().join("ext"),
    );
    let file = |p: &Path, text: &str| {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    };
    let manifest = |name: &str, deps: &str| {
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\n{deps}"
        )
    };
    file(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\nresolver = \"2\"\n",
    );
    file(&root.join("crates/a/Cargo.toml"), &manifest("a", ""));
    file(
        &root.join("crates/a/src/lib.rs"),
        "pub fn base() -> u8 {\n    1\n}\n",
    );
    file(
        &root.join("crates/b/Cargo.toml"),
        &manifest(
            "b",
            "a = { path = \"../a\" }\next = { path = \"../../../ext\" }\n",
        ),
    );
    file(
        &root.join("crates/b/src/lib.rs"),
        "pub fn top() -> u8 {\n    a::base() + a::base() + ext::call()\n}\n",
    );
    file(&ext.join("Cargo.toml"), &manifest("ext", ""));
    file(&ext.join("src/lib.rs"), "pub fn call() -> u8 {\n    7\n}\n");
    let root = root.to_str().unwrap();

    footnote(&state, &["code-index", root]);
    let map = footnote(&state, &["code-map", "--root", root]);
    let names: Vec<&str> = map["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"crates/a") && names.contains(&"crates/b"),
        "{map}"
    );
    assert_eq!(
        map["links"],
        serde_json::json!([{"from": "crates/b", "to": "crates/a", "references": 2}]),
        "{map}"
    );
    let ext = &map["outside"][0];
    assert_eq!(
        (ext["package"].as_str(), ext["references"].as_i64()),
        (Some("ext"), Some(1)),
        "{map}"
    );
    assert_eq!(ext["used_by"], serde_json::json!(["crates/b"]), "{map}");
}

/// Without rust-analyzer on PATH, indexing says how to install it instead of a raw "program not found".
#[test]
fn a_missing_indexer_falls_back_to_the_parser_and_says_how_to_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("demo");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
    fs::write(
        project.join("src/main.rs"),
        "fn main() {\n    greet();\n}\n\nfn greet() {}\n",
    )
    .unwrap();
    let state = dir.path().join("state");
    let run = |args: &[&str]| {
        Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
            .env("FOOTNOTE_STATE_DIR", &state)
            .env("FOOTNOTE_POLICY_FILE", dir.path().join("no-policy.json"))
            .env("PATH", dir.path().join("empty"))
            .args(args)
            .output()
            .unwrap()
    };
    let out = run(&["code-index", project.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        (v["indexer"].as_str(), v["origin"].as_str()),
        (Some("tree-sitter"), Some("resolved"))
    );
    assert!(
        v["upgrade"]
            .as_str()
            .unwrap()
            .contains("rustup component add rust-analyzer"),
        "{v}"
    );
    let out = run(&["code", "greet", "--root", project.to_str().unwrap()]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["origin"], "resolved");
    assert_eq!(v["symbols"][0]["callers"][0]["name"], "main", "{v}");
}
