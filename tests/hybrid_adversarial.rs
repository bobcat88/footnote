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

/// Goal 18: a new version of a documentation set re-embeds only the text that changed. A vector is reused by the hash of the
/// text it encodes (any chunk, any id, any file), never across models, and never for text that changed.
#[test]
fn a_new_version_reembeds_only_changed_text_and_never_across_models() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("docs");
    fs::create_dir(&root).unwrap();
    let page = |extra: &str| {
        format!(
            "# Install\nRun the installer once.\n\n## Upgrade\nStop the service before upgrading.{extra}\n\n## Remove\nDelete the folder.\n"
        )
    };
    fs::write(root.join("guide.md"), page("")).unwrap();
    fs::write(root.join("faq.md"), "# FAQ\nPorts are 8080 and 8443.\n").unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    let embed = |model: &str| -> (u64, u64) {
        let v = run(&db, &["rebuild-embeddings", "--model", model]);
        let m = &v["models"][0];
        (
            m["embedded"].as_u64().unwrap(),
            m["reused"].as_u64().unwrap(),
        )
    };
    run(&db, &["index", "--root", r, "--allow", r]);
    let (first, _) = embed(MODEL);
    assert!(first >= 2, "{first}");

    // Version 2: one section changes, the FAQ moves to a new file name (new chunk ids, same text).
    fs::write(root.join("guide.md"), page(" Back up the database first.")).unwrap();
    fs::rename(root.join("faq.md"), root.join("questions.md")).unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    let (embedded, reused) = embed(MODEL);
    assert!(
        embedded >= 1 && embedded < first,
        "only changed text is embedded: {embedded} of {first}"
    );
    assert_eq!(embedded + reused, first, "every chunk has a vector");
    let hit = run(
        &db,
        &[
            "search",
            "back up the database",
            "--mode",
            "hybrid",
            "--model",
            MODEL,
        ],
    );
    assert!(
        hit["results"]
            .to_string()
            .contains("Back up the database first"),
        "{hit}"
    );

    // Another model shares nothing: everything is embedded for it.
    let other = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/e5-small");
    let (embedded, reused) = embed(other);
    assert_eq!((embedded, reused), (first, 0));
}

/// A database embedded before the signature change keeps its vectors: search uses them as they are, and the next rebuild
/// converts their signatures without embedding anything again.
#[test]
fn vectors_signed_the_old_way_stay_in_use_and_convert_without_reembedding() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("a.md"),
        "# WAL\nCheckpoint frames and concurrent readers.\n",
    )
    .unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    run(&db, &["rebuild-embeddings", "--model", MODEL]);
    {
        let c = rusqlite::Connection::open(&db).unwrap();
        let rows: Vec<(i64, String)> = c
            .prepare("SELECT c.id, s.sha256 || ':' || s.revision || ':' || c.heading || char(10) || c.text FROM chunks c JOIN sources s ON s.id=c.source_id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for (id, old) in rows {
            c.execute(
                "UPDATE embeddings SET signature=?1 WHERE kind='chunk' AND id=?2",
                rusqlite::params![footnote_core::hash(old.as_bytes()), id],
            )
            .unwrap();
        }
    }
    let hit = run(
        &db,
        &["search", "readers", "--mode", "hybrid", "--model", MODEL],
    );
    assert!(
        hit["results"][0]["fusion_score"].is_number(),
        "old vectors still used: {hit}"
    );
    let v = run(&db, &["rebuild-embeddings", "--model", MODEL]);
    assert_eq!(v["models"][0]["embedded"], 0, "{v}");
}
