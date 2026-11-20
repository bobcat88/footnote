#![cfg(all(feature = "code", feature = "memory"))]
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path, time::Duration};

fn footnote(state: &Path, args: &[&str]) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .env("FOOTNOTE_POLICY_FILE", state.join("no-policy.json"))
        .args(args)
        .timeout(Duration::from_secs(120))
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}

fn lib<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["libraries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == name)
        .unwrap_or_else(|| panic!("{name} in {v}"))
}

/// Manifests win, a lockfile pins the version, the result is recorded, and a re-run reports only the difference.
#[test]
fn manifests_are_recorded_and_a_rerun_reports_only_the_difference() {
    let dir = tempfile::tempdir().unwrap();
    let (state, p) = (dir.path().join("state"), dir.path().join("app"));
    fs::create_dir_all(&p).unwrap();
    fs::write(
        p.join("Cargo.toml"),
        "[package]\nname = \"app\"\n[dependencies]\nserde = \"1\"\nanyhow = \"1\"\n",
    )
    .unwrap();
    fs::write(
        p.join("Cargo.lock"),
        "[[package]]\nname = \"serde\"\nversion = \"1.0.229\"\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        p.join("package.json"),
        r#"{"dependencies": {"react": "^18.2.0"}}"#,
    )
    .unwrap();
    fs::write(
        p.join("requirements.txt"),
        "flask==3.0.0\n# comment\n-r other.txt\n",
    )
    .unwrap();
    fs::write(
        p.join("go.mod"),
        "module app\nrequire github.com/gin-gonic/gin v1.9.1\n",
    )
    .unwrap();
    // An import with no manifest entry is not used while manifests exist.
    fs::write(p.join("x.py"), "import numpy\n").unwrap();
    let root = p.to_str().unwrap();

    let first = footnote(&state, &["stack", root]);
    assert_eq!(
        (first["status"].as_str(), first["recorded"].as_bool()),
        (Some("detected"), Some(true)),
        "{first}"
    );
    let serde = lib(&first, "serde");
    assert_eq!(
        (&serde["version"], &serde["exact"], &serde["source"]),
        (&"1.0.229".into(), &true.into(), &"manifest".into())
    );
    assert_eq!(lib(&first, "anyhow")["exact"], false);
    assert_eq!(
        (
            &lib(&first, "react")["ecosystem"],
            &lib(&first, "react")["version"]
        ),
        (&"npm".into(), &"^18.2.0".into())
    );
    assert_eq!(lib(&first, "flask")["version"], "3.0.0");
    assert_eq!(lib(&first, "github.com/gin-gonic/gin")["ecosystem"], "go");
    assert!(
        first["libraries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["name"] != "numpy"),
        "{first}"
    );

    let same = footnote(&state, &["stack", root]);
    assert_eq!(same["status"], "unchanged", "{same}");
    assert!(
        same.get("libraries").is_none() && same["recorded"] == false,
        "{same}"
    );

    fs::write(
        p.join("Cargo.toml"),
        "[package]\nname = \"app\"\n[dependencies]\nserde = \"1\"\nurl = \"2\"\n",
    )
    .unwrap();
    let next = footnote(&state, &["stack", root]);
    assert_eq!(
        (next["status"].as_str(), next["recorded"].as_bool()),
        (Some("changed"), Some(true)),
        "{next}"
    );
    assert_eq!(next["changes"]["added"][0]["name"], "url", "{next}");
    assert_eq!(next["changes"]["removed"][0]["name"], "anyhow", "{next}");
    assert!(next.get("libraries").is_none(), "{next}");
    assert_eq!(footnote(&state, &["stack", root])["status"], "unchanged");
}

/// No manifest: the imports name the packages; the standard library and the project's own modules do not count.
#[test]
fn imports_name_the_packages_when_there_is_no_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let (state, p) = (dir.path().join("state"), dir.path().join("script"));
    fs::create_dir_all(&p).unwrap();
    fs::write(
        p.join("main.py"),
        "import os, yaml\nimport requests\nfrom helpers import go\nfrom . import sibling\n",
    )
    .unwrap();
    fs::write(p.join("helpers.py"), "def go():\n    pass\n").unwrap();
    fs::write(
        p.join("web.js"),
        "import express from 'express'\nconst _ = require('lodash')\nimport fs from 'node:fs'\nimport p from 'path'\nimport x from './local.js'\nimport s from '@scope/pkg/sub'\n",
    )
    .unwrap();
    fs::write(
        p.join("main.go"),
        "package main\nimport (\n\t\"fmt\"\n\t\"github.com/gin-gonic/gin\"\n)\n",
    )
    .unwrap();
    fs::write(
        p.join("lib.rs"),
        "use std::io;\nuse serde::Deserialize;\nuse crate::x;\n",
    )
    .unwrap();
    let out = footnote(&state, &["stack", p.to_str().unwrap()]);
    let mut got: Vec<String> = out["libraries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            assert_eq!(
                (&l["source"], &l["version"]),
                (&"import".into(), &Value::Null)
            );
            format!(
                "{}:{}",
                l["ecosystem"].as_str().unwrap(),
                l["name"].as_str().unwrap()
            )
        })
        .collect();
    got.sort();
    assert_eq!(
        got,
        [
            "cargo:serde",
            "go:github.com/gin-gonic/gin",
            "npm:@scope/pkg",
            "npm:express",
            "npm:lodash",
            "pypi:pyyaml",
            "pypi:requests"
        ],
        "{out}"
    );
}

/// Nothing to read: one short question, nothing recorded; the answer comes back as `--library` with source `user`.
#[test]
fn an_empty_folder_gets_one_question_and_an_answer_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (state, p) = (dir.path().join("state"), dir.path().join("empty"));
    fs::create_dir_all(&p).unwrap();
    let root = p.to_str().unwrap();
    let out = footnote(&state, &["stack", root]);
    assert_eq!(out["status"], "needs_answer", "{out}");
    assert!(
        out["question"].as_str().unwrap().len() < 200
            && out["libraries"].as_array().unwrap().is_empty()
    );
    assert!(!state.join("memory.db").exists());

    let told = footnote(&state, &["stack", root, "--library", "pypi:flask@3.0.0"]);
    let flask = lib(&told, "flask");
    assert_eq!(
        (&flask["source"], &flask["version"], &flask["exact"]),
        (&"user".into(), &"3.0.0".into(), &true.into())
    );
    assert_eq!(told["recorded"], true);
}
