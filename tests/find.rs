//! `footnote find`: one search over the products that are in the build and switched on (goal 13).
use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn find_searches_each_switched_on_product_and_says_why_one_has_nothing() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path().join("state");
    let footnote = || {
        let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", &state);
        c
    };
    let run = |args: &[&str]| -> Value {
        let out = footnote().args(args).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let product = |v: &Value, name: &str| {
        v["products"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["product"] == name)
            .unwrap()
            .clone()
    };

    // Nothing indexed yet: Library has no corpus to search, and says so.
    let v = run(&["find", "checkpoint"]);
    assert_eq!(product(&v, "library")["state"], "empty", "{v}");

    // Indexing a corpus records it, so status lists it and find searches it.
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("wal.md"),
        "# SQLite\n## WAL\nCheckpoint latency is amortized by batching writes.\n",
    )
    .unwrap();
    let db = t.path().join("k.db");
    let r = root.to_str().unwrap();
    footnote()
        .arg("--db")
        .arg(&db)
        .args(["index", "--root", r, "--allow", r])
        .assert()
        .success();
    let status = run(&["status"]);
    assert!(
        status["products"]["library"]["corpora"]
            .to_string()
            .contains("k.db"),
        "{status}"
    );
    let v = run(&["find", "checkpoint latency"]);
    let lib = product(&v, "library");
    assert_eq!(lib["state"], "ok", "{v}");
    assert!(
        lib["results"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("wal.md"),
        "{v}"
    );
    // Code looks symbols up by name: a phrase is not one.
    assert_eq!(product(&v, "code")["state"], "not_a_symbol", "{v}");
    // A word that matches nothing is "nothing found", not an error.
    assert_eq!(
        product(&run(&["find", "zebra"]), "library")["state"],
        "nothing_found"
    );

    // A switched-off product is not searched.
    footnote()
        .args(["config", "set", "library.enabled", "false"])
        .assert()
        .success();
    let lib = product(&run(&["find", "checkpoint"]), "library");
    assert_eq!(
        (
            lib["state"].as_str(),
            lib["results"].as_array().unwrap().len()
        ),
        (Some("off"), 0)
    );

    // An empty query is refused.
    assert!(
        !footnote()
            .args(["find", "  "])
            .output()
            .unwrap()
            .status
            .success()
    );
}
