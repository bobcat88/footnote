use assert_cmd::Command;
use serde_json::Value;
use std::fs;
#[test]
fn stats_reports_live_sqlite_fts_and_failed_extraction_states() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("bad.txt"), [255, 254]).unwrap();
    let db = t.path().join("k.db");
    let call = |args: &[&str]| -> Value {
        let o = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
            .arg("--db")
            .arg(&db)
            .args(args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice(&o).unwrap()
    };
    call(&[
        "index",
        "--root",
        root.to_str().unwrap(),
        "--allow",
        root.to_str().unwrap(),
    ]);
    let v = call(&["stats"]);
    assert_eq!(v["fts5"], true);
    assert!(v["sqlite_version"].as_str().is_some());
    assert_eq!(v["records"], 0);
    assert_eq!(v["source_statuses"]["failed"], 1);
}
