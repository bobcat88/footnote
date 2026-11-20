mod support;

use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path};

const MINILM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/minilm");

fn footnote(state: &Path, db: &Path, args: &[&str]) -> std::process::Output {
    Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .env("FOOTNOTE_POLICY_FILE", state.join("no-policy.json"))
        .arg("--db")
        .arg(db)
        .args(args)
        .output()
        .unwrap()
}

fn json(out: std::process::Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Search is hybrid by default once the corpus has vectors from a model installed in Footnote's models folder; lexical otherwise.
#[test]
fn search_is_hybrid_by_default_when_the_corpus_models_are_installed() {
    if !support::model("minilm") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let (state, root, db) = (
        t.path().join("state"),
        t.path().join("notes"),
        t.path().join("k.db"),
    );
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("wal.md"),
        "# WAL\nReaders and writers run concurrently with write-ahead logging.",
    )
    .unwrap();
    fs::write(
        root.join("pasta.md"),
        "# Cooking\nSalt the water before the pasta goes in.",
    )
    .unwrap();
    let r = root.to_str().unwrap();
    json(footnote(&state, &db, &["index", "--root", r, "--allow", r]));
    json(footnote(
        &state,
        &db,
        &["rebuild-embeddings", "--model", MINILM],
    ));

    // Nothing installed: lexical, and forcing hybrid explains what is missing.
    let lexical = json(footnote(&state, &db, &["search", "write-ahead logging"]));
    assert!(lexical["mode"].as_str() != Some("hybrid_cc"), "{lexical}");
    let forced = footnote(
        &state,
        &db,
        &["search", "write-ahead logging", "--mode", "hybrid"],
    );
    assert!(!forced.status.success());
    assert!(String::from_utf8_lossy(&forced.stdout).contains("footnote models install"));

    // The same model installed (byte-identical manifest): hybrid without naming it (the MCP tool shares app::search_models).
    let installed = state.join("models/minilm");
    fs::create_dir_all(&installed).unwrap();
    for f in ["manifest.json", "model.onnx", "tokenizer.json"] {
        fs::copy(Path::new(MINILM).join(f), installed.join(f)).unwrap();
    }
    let hybrid = json(footnote(&state, &db, &["search", "write-ahead logging"]));
    assert_eq!(hybrid["mode"], "hybrid_cc", "{hybrid}");
    let words = json(footnote(
        &state,
        &db,
        &["search", "write-ahead logging", "--mode", "lexical"],
    ));
    assert!(words["mode"].as_str() != Some("hybrid_cc"));
}
