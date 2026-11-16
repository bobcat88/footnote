use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path, time::Duration};

fn models(state: &Path, args: &[&str]) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .env("FOOTNOTE_POLICY_FILE", state.join("no-policy.json"))
        .arg("models")
        .args(args)
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Offline: an installed model is a folder with a manifest; downloads are checked by hand (`footnote models install`).
#[test]
fn use_and_remove_protect_the_models_memory_relies_on() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();

    let list = models(state, &["list"]);
    let names: Vec<&str> = list["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"bekko-a8m") && names.contains(&"gte-small"),
        "{list}"
    );
    assert!(
        models(state, &["install", "nope"])["error"]
            .as_str()
            .unwrap()
            .contains("unknown model")
    );

    // The French model shows its licence; without llama.cpp compiled in, installing it is refused before any download.
    assert!(
        list["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["name"] == "jina-v5-nano"
                && m["license"] == "CC-BY-NC-4.0 (no commercial use)"),
        "{list}"
    );
    if !cfg!(feature = "llama") {
        assert!(
            models(state, &["install", "jina-v5-nano"])["error"]
                .as_str()
                .unwrap()
                .contains("does not include")
        );
        assert!(!state.join("models/jina-v5-nano").exists());
    }

    // Alias's detector is listed, refused as an embedding model, and refused at install in builds without Alias.
    assert!(
        list["models"].as_array().unwrap().iter().any(
            |m| m["name"] == "openai-privacy-filter" && m["installed"] == false
        ),
        "{list}"
    );
    assert!(
        models(state, &["use", "openai-privacy-filter"])["error"]
            .as_str()
            .unwrap()
            .contains("not an embedding model")
    );
    if !cfg!(feature = "alias") {
        assert!(
            models(state, &["install", "openai-privacy-filter"])["error"]
                .as_str()
                .unwrap()
                .contains("does not include")
        );
        assert!(!state.join("models/openai-privacy-filter").exists());
    }

    // Not installed: refused, nothing written.
    assert!(
        models(state, &["use", "gte-small"])["error"]
            .as_str()
            .unwrap()
            .contains("not installed")
    );
    assert!(!state.join("memory.json").exists());

    let installed = state.join("models/gte-small");
    fs::create_dir_all(&installed).unwrap();
    fs::write(installed.join("manifest.json"), "{}").unwrap();
    fs::write(state.join("memory.json"), r#"{"every_turns": 5}"#).unwrap();
    let used = models(state, &["use", "gte-small"]);
    assert_eq!(used["memory_models"].as_array().unwrap().len(), 1, "{used}");
    let settings: Value =
        serde_json::from_slice(&fs::read(state.join("memory.json")).unwrap()).unwrap();
    assert_eq!(settings["every_turns"], 5, "other settings must survive");
    assert!(
        models(state, &["list"])["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["name"] == "gte-small"
                && m["installed"] == true
                && m["used_by_memory"] == true)
    );

    // In use: refused; once memory uses something else, removed.
    assert!(
        models(state, &["remove", "gte-small"])["error"]
            .as_str()
            .unwrap()
            .contains("used by Footnote Memory")
    );
    let other = state.join("models/minilm");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("manifest.json"), "{}").unwrap();
    models(state, &["use", "minilm"]);
    assert_eq!(models(state, &["remove", "gte-small"])["removed"], true);
    assert!(!installed.exists());

    let rec = models(state, &["recommend"]);
    assert!(
        rec["hardware"]["cpu_threads"].as_u64().unwrap() > 0,
        "{rec}"
    );
    assert_eq!(rec["recommended"][0], "bekko-a8m");
}
