use assert_cmd::Command;
use serde_json::Value;
use std::fs;

fn run(db: &std::path::Path, args: &[&str]) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .arg("--db")
        .arg(db)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).unwrap()
}

fn paths(v: &Value) -> Vec<String> {
    v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn per_source_limits_how_many_chunks_of_one_file_are_returned() {
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    // Sections past 500 characters stay separate chunks (shorter ones join the next section).
    let pad = "filler text about write-ahead logs. ".repeat(15);
    fs::write(
        root.join("big.md"),
        format!("# One\ncheckpoint stalls under load\n{pad}\n# Two\ncheckpoint frames are copied\n{pad}\n# Three\ncheckpoint passive mode helps"),
    )
    .unwrap();
    fs::write(root.join("other.md"), "# Other\ncheckpoint in another file").unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    run(&db, &["index", "--root", r, "--allow", r]);

    let all = paths(&run(&db, &["search", "checkpoint", "--limit", "8"]));
    let big = |p: &Vec<String>| p.iter().filter(|x| x.ends_with("big.md")).count();
    assert!(
        big(&all) >= 2,
        "without a limit one file can fill the list: {all:?}"
    );

    let one = paths(&run(
        &db,
        &["search", "checkpoint", "--limit", "8", "--per-source", "1"],
    ));
    let mut sorted = one.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), one.len(), "one chunk per file: {one:?}");
    assert_eq!(one.len(), 2, "both files are still represented: {one:?}");

    let two = paths(&run(
        &db,
        &["search", "checkpoint", "--limit", "8", "--per-source", "2"],
    ));
    assert_eq!(big(&two), 2);
}
