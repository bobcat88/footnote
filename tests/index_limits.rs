use assert_cmd::Command;
use std::fs;

fn index(db: &std::path::Path, root: &str, extra: &[&str]) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .arg("--db")
        .arg(db)
        .args(["index", "--root", root, "--allow", root])
        .args(extra)
        .assert()
}

#[test]
fn scan_bounds_are_options_and_an_exceeded_bound_rolls_back() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    for i in 0..3 {
        fs::write(
            root.join(format!("n{i}.md")),
            format!("# Note {i}\nsome text {i}"),
        )
        .unwrap();
    }
    let r = root.to_str().unwrap();
    let db = t.path().join("k.db");
    let out = index(&db, r, &["--max-files", "2"])
        .failure()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8_lossy(&out).contains("--max-files"));
    let c = rusqlite::Connection::open(&db).unwrap();
    let sources: i64 = c
        .query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
        .unwrap();
    assert_eq!(sources, 0, "an aborted scan must leave nothing behind");
    drop(c);
    index(&db, r, &["--max-mib", "0"]).failure();
    index(&db, r, &[]).success(); // the defaults are large
}
