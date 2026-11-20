#![cfg(all(feature = "code", feature = "memory", feature = "library"))]
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path, time::Duration};

fn frame(state: &Path, project: &Path, extra: &[&str]) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .env("FOOTNOTE_POLICY_FILE", state.join("no-policy.json"))
        .arg("frame")
        .arg(project)
        .args(extra)
        .timeout(Duration::from_secs(120))
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}

fn check<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
}

fn flask(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("app");
    fs::create_dir_all(p.join(".git")).unwrap();
    fs::write(p.join("requirements.txt"), "flask==3.0.0\n").unwrap();
    fs::write(
        p.join("README.md"),
        "# App\n\nA small web service that stores bookmarks for a team.\n",
    )
    .unwrap();
    fs::create_dir_all(p.join("tests")).unwrap();
    fs::write(p.join("tests").join("test_app.py"), "def test_a(): pass\n").unwrap();
    p
}

#[test]
fn empty_folder_scores_low_and_names_next_actions() {
    let dir = tempfile::tempdir().unwrap();
    let (state, p) = (dir.path().join("state"), dir.path().join("empty"));
    fs::create_dir_all(&p).unwrap();
    let v = frame(&state, &p, &[]);
    assert_eq!(
        (v["score"].as_u64(), v["band"].as_str()),
        (Some(0), Some("not yet")),
        "{v}"
    );
    assert_eq!(v["thresholds"]["ready"], 80);
    assert!(
        check(&v, "stack")["next"]
            .as_str()
            .unwrap()
            .contains("footnote stack")
    );
    assert_eq!(v["questions"].as_array().unwrap().len(), 4);
    assert!(v["text"].as_str().unwrap().contains("Readiness 0/100"));
}

#[test]
fn flask_project_with_cached_docs_scores_higher_and_a_goal_raises_it() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let p = flask(dir.path());
    let empty = dir.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let bare = frame(&state, &p, &[]);
    assert_eq!(check(&bare, "docs_cached")["passed"], false);
    fs::create_dir_all(state.join("docs").join("flask")).unwrap();
    fs::write(state.join("docs").join("flask").join("3.0.0.db"), b"x").unwrap();
    let docs = frame(&state, &p, &[]);
    // stack 15 + dependency file 10 + docs 20 + readme 5 + tests 10 + git 5
    assert_eq!(
        (docs["score"].as_u64(), docs["band"].as_str()),
        (Some(65), Some("almost")),
        "{docs}"
    );
    assert!(docs["score"].as_u64() > bare["score"].as_u64());
    assert!(
        docs["questions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|q| q["key"] != "tests")
    );

    let set = frame(
        &state,
        &p,
        &[
            "--set",
            "goal=track team bookmarks",
            "--set",
            "users=the support team",
        ],
    );
    assert_eq!(set["score"], 95, "{set}");
    assert_eq!(set["band"], "ready");
    // Counted on the next run, from project memory.
    let again = frame(&state, &p, &[]);
    assert_eq!(again["answers"]["goal"], "track team bookmarks");
    assert_eq!(check(&again, "goal")["passed"], true);
    // Changing an answer replaces it.
    let changed = frame(&state, &p, &["--set", "goal=track bookmarks"]);
    assert_eq!(changed["answers"]["goal"], "track bookmarks");
}

#[test]
fn bad_answers_are_refused_and_json_shape_is_stable() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let p = flask(dir.path());
    let bad = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", &state)
        .args(["frame"])
        .arg(&p)
        .args(["--set", "colour=red"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    let v = frame(&state, &p, &[]);
    for k in [
        "root",
        "score",
        "band",
        "thresholds",
        "checks",
        "questions",
        "answers",
        "next",
        "text",
    ] {
        assert!(v.get(k).is_some(), "{k} in {v}");
    }
    let names: Vec<&str> = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "stack",
            "dependency_file",
            "docs_cached",
            "readme",
            "goal",
            "users",
            "constraints",
            "tests",
            "git"
        ]
    );
    let weight: u64 = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["weight"].as_u64().unwrap())
        .sum();
    assert_eq!(weight, 100);
    for c in v["checks"].as_array().unwrap() {
        for k in ["name", "weight", "earned", "passed", "why", "next"] {
            assert!(c.get(k).is_some(), "{k} in {c}");
        }
    }
}
