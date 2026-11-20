//! `footnote docs`: one database per site and version, shared, listed by `corpora`, pruned to the newest.
use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn one_database_per_site_and_version_and_prune_keeps_the_newest() {
    // Outside the system temp folder: corpora there are deliberately not recorded by `index` (docs registers explicitly).
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let run = |args: &[&str]| -> Value {
        let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
            .env("FOOTNOTE_STATE_DIR", &state)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let notes = t.path().join("notes");
    fs::create_dir(&notes).unwrap();
    fs::write(
        notes.join("a.md"),
        "# Pool\n## Sizing\nConnections are reused from the pool.\n",
    )
    .unwrap();
    let from = notes.to_str().unwrap();

    let first = run(&[
        "docs",
        "add",
        "--from",
        from,
        "--site",
        "Acme Docs",
        "--version",
        "9.2",
    ]);
    assert_eq!(first["reused"], false, "{first}");
    // A second project asking for the same site and version reuses it without reading the folder (it is gone).
    fs::remove_dir_all(&notes).unwrap();
    let second = run(&[
        "docs",
        "add",
        "--from",
        from,
        "--site",
        "acme docs",
        "--version",
        "9.2",
    ]);
    assert_eq!(second["reused"], true, "{second}");
    assert_eq!(first["db"], second["db"]);

    fs::create_dir(&notes).unwrap();
    fs::write(notes.join("a.md"), "# Pool\n## Sizing\nNew in ten.\n").unwrap();
    run(&[
        "docs",
        "add",
        "--from",
        from,
        "--site",
        "acme docs",
        "--version",
        "10.0",
    ]);
    assert_eq!(run(&["docs", "list"])["docs"].as_array().unwrap().len(), 2);
    assert_eq!(run(&["corpora"])["corpora"].as_array().unwrap().len(), 2);

    // Names cannot climb out of the cache.
    let sneaky = run(&[
        "docs",
        "add",
        "--from",
        from,
        "--site",
        "../../x",
        "--version",
        "../v1",
    ]);
    assert_eq!(sneaky["site"], "-..-x", "{sneaky}");
    assert_eq!(sneaky["version"], "-v1", "{sneaky}");
    assert!(state.join("docs/-..-x/-v1.db").is_file());
    // A name with nothing usable left is refused.
    let bad = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", &state)
        .args([
            "docs",
            "add",
            "--from",
            from,
            "--site",
            "x",
            "--version",
            "..",
        ])
        .output()
        .unwrap();
    assert!(!bad.status.success());

    let pruned = run(&["docs", "prune"]);
    assert_eq!(pruned["removed"].as_array().unwrap().len(), 1, "{pruned}");
    assert_eq!(pruned["removed"][0]["version"], "9.2");
    assert!(state.join("docs/acme-docs/10.0.db").is_file());
    assert!(!state.join("docs/acme-docs/9.2.db").exists());
    assert_eq!(run(&["corpora"])["corpora"].as_array().unwrap().len(), 2);
}
