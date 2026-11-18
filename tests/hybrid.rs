mod support;
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path};
fn run(db: &Path, args: &[&str]) -> Value {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .arg("--db")
        .arg(db)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}
#[test]
fn local_embeddings_retrieve_bilingual_paraphrase_and_search_is_read_only() {
    if !(support::model("minilm")) {
        return;
    }
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let db = t.path().join("k.db");
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("wal.md"), "# SQLite WAL\nReaders and writers can run concurrently using write-ahead logging. A checkpoint copies frames into the main database.").unwrap();
    fs::write(
        root.join("http.md"),
        "# HTTP\nETag conditional requests avoid downloading unchanged resources.",
    )
    .unwrap();
    let root = root.to_str().unwrap();
    run(&db, &["index", "--root", root, "--allow", root]);
    let model = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/minilm");
    let rebuilt = run(&db, &["rebuild-embeddings", "--model", model]);
    assert_eq!(rebuilt["embedded"], 2);
    let before = fs::read(&db).unwrap();
    let v = run(
        &db,
        &[
            "search",
            "Comment permettre les lectures pendant une écriture SQLite?",
            "--mode",
            "hybrid",
            "--model",
            model,
            "--limit",
            "1",
        ],
    );
    assert_eq!(v["mode"], "hybrid_cc");
    assert!(
        v["results"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("wal.md")
    );
    assert_eq!(v["results"][0]["freshness"], "current");
    assert_eq!(before, fs::read(&db).unwrap());
}
