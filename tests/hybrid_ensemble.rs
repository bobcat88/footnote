use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path};

const MINILM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/minilm");
const E5: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/e5-small");

fn run(db: &Path, args: &[&str], scan_limit: Option<&str>) -> Value {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("borg-knowledge"));
    c.arg("--db").arg(db).args(args);
    if let Some(n) = scan_limit {
        c.env("BORG_KNOWLEDGE_SCAN_LIMIT", n);
    }
    serde_json::from_slice(&c.assert().success().get_output().stdout).unwrap()
}

#[test]
fn two_models_blend_and_both_search_strategies_find_the_right_note() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("wal.md"), "# WAL
Readers and writers run concurrently with write-ahead logging; a checkpoint copies frames into the database.").unwrap();
    fs::write(
        root.join("http.md"),
        "# HTTP caching
ETag validators let a client skip downloads of unchanged resources.",
    )
    .unwrap();
    fs::write(
        root.join("pasta.md"),
        "# Cooking
Salt the water before the pasta goes in and stop when it is al dente.",
    )
    .unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    run(&db, &["index", "--root", r, "--allow", r], None);

    let built = run(
        &db,
        &["rebuild-embeddings", "--model", MINILM, "--model", E5],
        None,
    );
    assert_eq!(built["embedded"], 6); // 3 chunks x 2 models
    assert_eq!(built["models"].as_array().unwrap().len(), 2);
    // a second rebuild reuses everything
    let again = run(
        &db,
        &["rebuild-embeddings", "--model", MINILM, "--model", E5],
        None,
    );
    assert_eq!(again["embedded"], 0);
    assert_eq!(again["models"][0]["reused"], 3);

    // no word in common with the note: only the dense side can find it (full scan)
    let q = "Comment lire pendant qu'un autre processus écrit dans la base ?";
    let args = [
        "search", q, "--mode", "hybrid", "--model", MINILM, "--model", E5, "--limit", "1",
    ];
    let scan = run(&db, &args, None);
    assert_eq!(scan["mode"], "hybrid_cc");
    assert_eq!(scan["strategy"], "full_dense_scan");
    assert!(
        scan["results"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("wal.md"),
        "{scan}"
    );

    // forced shortlist strategy: a query sharing words with the note still finds it, and vectors are fetched by id
    let args = [
        "search",
        "write-ahead logging checkpoint",
        "--mode",
        "hybrid",
        "--model",
        MINILM,
        "--model",
        E5,
        "--limit",
        "1",
    ];
    let short = run(&db, &args, Some("1"));
    assert_eq!(short["strategy"], "lexical_shortlist");
    assert!(
        short["results"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("wal.md"),
        "{short}"
    );
    assert!(short["with_vectors"].as_u64().unwrap() >= 1);
}
