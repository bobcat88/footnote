use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path};
fn cmd(db: &Path, args: &[&str]) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
    c.arg("--db").arg(db).args(args);
    c
}
fn run(db: &Path, args: &[&str]) -> Value {
    serde_json::from_slice(&cmd(db, args).assert().success().get_output().stdout).unwrap()
}
const MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/minilm");
#[test]
fn zero_norm_derived_vectors_are_refused_instead_of_ranked() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("wal.md"),
        "# WAL\nCheckpoint frames and concurrent readers.",
    )
    .unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    run(&db, &["rebuild-embeddings", "--model", MODEL]);
    let c = rusqlite::Connection::open(&db).unwrap();
    c.execute("UPDATE embeddings SET vector=zeroblob(388)", [])
        .unwrap();
    drop(c);
    cmd(
        &db,
        &["search", "checkpoint", "--mode", "hybrid", "--model", MODEL],
    )
    .assert()
    .failure();
}
#[test]
fn changes_deletions_duplicates_and_missing_index_do_not_reuse_stale_vectors() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    let a = root.join("a.md");
    let b = root.join("b.md");
    fs::write(&a, "# WAL\nCheckpoint frames and concurrent readers.").unwrap();
    fs::copy(&a, &b).unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    cmd(
        &db,
        &["search", "frames", "--mode", "hybrid", "--model", MODEL],
    )
    .assert()
    .failure();
    run(&db, &["rebuild-embeddings", "--model", MODEL]);
    assert_eq!(
        run(
            &db,
            &["search", "frames", "--mode", "hybrid", "--model", MODEL]
        )["results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::write(&a, "# HTTP\nETag validators skip unchanged downloads.").unwrap();
    fs::remove_file(&b).unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    let v = run(
        &db,
        &["search", "frames", "--mode", "hybrid", "--model", MODEL],
    );
    assert_eq!(v["with_vectors"], 0);
    assert!(v["results"].as_array().unwrap().is_empty());
    run(&db, &["rebuild-embeddings", "--model", MODEL]);
    let etag = run(
        &db,
        &["search", "ETag", "--mode", "hybrid", "--model", MODEL],
    )["results"][0]["id"]
        .as_i64()
        .unwrap()
        .to_string();
    assert_eq!(run(&db, &["get", &etag])["revision"], 2);
    let c = rusqlite::Connection::open(&db).unwrap();
    c.execute("UPDATE embedding_models SET helper_sha256='tampered'", [])
        .unwrap();
    drop(c);
    cmd(
        &db,
        &["search", "ETag", "--mode", "hybrid", "--model", MODEL],
    )
    .assert()
    .failure();
}

#[test]
fn corrupt_model_rebuild_rolls_back() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("wal.md"),
        "# WAL\nCheckpoint frames and concurrent readers.",
    )
    .unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    run(&db, &["rebuild-embeddings", "--model", MODEL]);
    let model = t.path().join("model");
    fs::create_dir(&model).unwrap();
    fs::copy(
        Path::new(MODEL).join("manifest.json"),
        model.join("manifest.json"),
    )
    .unwrap();
    fs::write(model.join("model.onnx"), "corrupt").unwrap();
    fs::write(model.join("tokenizer.json"), "corrupt").unwrap();
    let before = fs::read(&db).unwrap();
    let output = cmd(
        &db,
        &["rebuild-embeddings", "--model", model.to_str().unwrap()],
    )
    .assert()
    .failure()
    .get_output()
    .stdout
    .clone();
    assert!(String::from_utf8_lossy(&output).contains("hash mismatch"));
    assert_eq!(before, fs::read(&db).unwrap());
}
