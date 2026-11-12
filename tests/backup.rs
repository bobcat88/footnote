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
#[test]
fn consistent_backup_restores_search_and_refuses_overwrite() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    let r = root.to_str().unwrap();
    let db = t.path().join("k.db");
    fs::write(root.join("x.md"), "# Recovery\nrecoverable checkpoint").unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);
    let c = rusqlite::Connection::open(&db).unwrap();
    c.execute_batch(
        "CREATE TABLE padding(bytes BLOB); INSERT INTO padding VALUES(zeroblob(20000000));",
    )
    .unwrap();
    drop(c);
    let backup = t.path().join("backup.db");
    run(&db, &["backup", "--output", backup.to_str().unwrap()]);
    assert_eq!(
        run(&backup, &["search", "recoverable"])["results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    cmd(&db, &["backup", "--output", backup.to_str().unwrap()])
        .assert()
        .failure();
    assert_eq!(run(&backup, &["stats"])["sources"], 1);
}
