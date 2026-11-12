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
fn tmp() -> tempfile::TempDir {
    tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap()
}
#[test]
fn incremental_index_handles_stale_move_delete_dedup_and_read_only_search() {
    let t = tmp();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    let r = root.to_str().unwrap();
    let db = t.path().join("k.db");
    let p = root.join("a.md");
    fs::write(&p, "# Identity\nuniqueword baseline\n").unwrap();
    fs::copy(&p, root.join("b.md")).unwrap();
    assert_eq!(
        run(&db, &["index", "--root", r, "--allow", r])["indexed"],
        2
    );
    let v = run(&db, &["search", "uniqueword"]);
    assert_eq!(v["results"].as_array().unwrap().len(), 1);
    assert_eq!(
        run(&db, &["index", "--root", r, "--allow", r])["unchanged"],
        2
    );
    fs::write(&p, "# Identity\nuniqueword revised\n").unwrap();
    let id = v["results"][0]["id"].as_i64().unwrap().to_string();
    assert_eq!(run(&db, &["get", &id])["freshness"], "stale");
    fs::rename(root.join("b.md"), root.join("moved.md")).unwrap();
    let v = run(&db, &["index", "--root", r, "--allow", r]);
    assert_eq!(v["indexed"], 2);
    assert_eq!(v["deleted"], 1);
    let before = fs::read(&db).unwrap();
    run(&db, &["search", "revised"]);
    run(&db, &["stats"]);
    assert_eq!(before, fs::read(&db).unwrap());
    fs::remove_file(&p).unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    assert!(
        run(&db, &["search", "revised"])["results"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn long_unicode_lines_are_chunked_without_silently_losing_tail() {
    let t = tmp();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    let r = root.to_str().unwrap();
    let db = t.path().join("k.db");
    fs::write(
        root.join("long.txt"),
        format!("{} tailmarker", "é".repeat(6000)),
    )
    .unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    let v = run(&db, &["search", "tailmarker"]);
    let id = v["results"][0]["id"].as_i64().unwrap().to_string();
    assert!(
        run(&db, &["get", &id])["text"]
            .as_str()
            .unwrap()
            .contains("tailmarker")
    );
}
#[test]
fn safety_bounds_allowlist_exclusions_literal_fts_and_future_schema() {
    let t = tmp();
    let root = t.path().join("notes");
    let other = t.path().join("other");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&other).unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    fs::write(
        root.join("ok.md"),
        "# Technical\nSELECT and OR are literal data",
    )
    .unwrap();
    fs::write(root.join("credentials.md"), "neverindexme").unwrap();
    fs::create_dir(root.join("private")).unwrap();
    fs::write(root.join("private/hide.md"), "neverindexme").unwrap();
    cmd(
        &db,
        &["index", "--root", r, "--allow", other.to_str().unwrap()],
    )
    .assert()
    .failure();
    assert_eq!(
        run(&db, &["index", "--root", r, "--allow", r])["indexed"],
        1
    );
    assert!(
        run(&db, &["search", "neverindexme"])["results"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    run(&db, &["search", "\" OR * NOT SELECT"]);
    cmd(&db, &["search", "x", "--limit", "21"])
        .assert()
        .failure();
    let c = rusqlite::Connection::open(&db).unwrap();
    c.pragma_update(None, "user_version", 999).unwrap();
    drop(c);
    let before = fs::read(&db).unwrap();
    cmd(&db, &["stats"]).assert().failure();
    assert_eq!(before, fs::read(&db).unwrap());
}
