//! `footnote update` and `footnote rollback` (#126), run on a copy of the built binary in a temporary install folder.
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path};

fn sha(p: &Path) -> String {
    footnote_core::hash(&fs::read(p).unwrap())
}

#[test]
fn update_keeps_the_previous_binary_and_rollback_restores_it() {
    let t = tempfile::tempdir().unwrap();
    let name = if cfg!(windows) {
        "footnote.exe"
    } else {
        "footnote"
    };
    let installed = t.path().join("install").join(name);
    fs::create_dir_all(installed.parent().unwrap()).unwrap();
    let built = assert_cmd::cargo::cargo_bin!("footnote");
    fs::copy(built, &installed).unwrap();
    // A "new" release: the same program with one more byte at the end, so the two files differ but both run.
    let newer = t.path().join(name);
    fs::copy(built, &newer).unwrap();
    let mut bytes = fs::read(&newer).unwrap();
    bytes.push(0);
    fs::write(&newer, bytes).unwrap();
    let (old_sha, new_sha) = (sha(&installed), sha(&newer));
    let run = |args: &[&str]| -> Value {
        let out = Command::new(&installed)
            .env("FOOTNOTE_STATE_DIR", t.path().join("state"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    };

    let not_footnote = t.path().join("notes.txt");
    fs::write(&not_footnote, "not a program").unwrap();
    let refused = Command::new(&installed)
        .args(["update", "--from"])
        .arg(&not_footnote)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(sha(&installed), old_sha, "nothing moved");

    let up = run(&["update", "--from", newer.to_str().unwrap()]);
    assert_eq!(sha(&installed), new_sha, "{up}");
    assert_eq!(sha(Path::new(up["previous"].as_str().unwrap())), old_sha);

    let back = run(&["rollback"]);
    assert_eq!(sha(&installed), old_sha, "{back}");
}
