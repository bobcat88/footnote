//! `footnote code-index --deep` (#299): a missing indexer is reported and never installed, an index written by an indexer is data
//! that may be bad, and a valid one becomes the compiler graph. The indexer is a stand-in script that copies a prepared index.
#![cfg(feature = "code")]
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path};

fn varint(mut n: usize) -> Vec<u8> {
    let mut v = vec![];
    while n >= 0x80 {
        v.push((n & 0x7f) as u8 | 0x80);
        n >>= 7;
    }
    v.push(n as u8);
    v
}

fn field(n: u8, bytes: &[u8]) -> Vec<u8> {
    let mut v = vec![n << 3 | 2];
    v.extend(varint(bytes.len()));
    v.extend_from_slice(bytes);
    v
}

/// A SCIP index of `src/a.ts`: `f` and `g` defined, `g` calls `f`.
fn index() -> Vec<u8> {
    let (f, g) = (
        "scip-typescript npm demo 1.0.0 src/`a.ts`/f().",
        "scip-typescript npm demo 1.0.0 src/`a.ts`/g().",
    );
    let occ = |range: &[u8], sym: &str, roles: u8, enc: Option<&[u8]>| {
        let mut o = field(1, range);
        o.extend(field(2, sym.as_bytes()));
        if roles != 0 {
            o.extend([3 << 3, roles]);
        }
        if let Some(e) = enc {
            o.extend(field(7, e));
        }
        field(2, &o)
    };
    let mut doc = field(1, b"src/a.ts");
    doc.extend(occ(&[0, 9, 10], f, 1, Some(&[0, 0, 0, 15])));
    doc.extend(occ(&[1, 9, 10], g, 1, Some(&[1, 0, 1, 22])));
    doc.extend(occ(&[1, 15, 16], f, 0, None));
    field(2, &doc)
}

struct Fixture {
    dir: tempfile::TempDir,
    project: std::path::PathBuf,
    bin: std::path::PathBuf,
}

/// A TypeScript project and a folder holding a stand-in `scip-typescript` that copies `$FAKE_SCIP` to its `--output`.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("demo");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("tsconfig.json"), "{}").unwrap();
    fs::write(
        project.join("src/a.ts"),
        "function f() {}\nfunction g() { f(); }\n",
    )
    .unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    if cfg!(windows) {
        fs::write(
            bin.join("scip-typescript.cmd"),
            "@echo off\r\nif \"%1\"==\"--version\" (echo 0.0.0& exit /b 0)\r\ncopy /y \"%FAKE_SCIP%\" \"%4\" >nul\r\n",
        )
        .unwrap();
    } else {
        let sh = bin.join("scip-typescript");
        fs::write(
            &sh,
            "#!/bin/sh\n[ \"$1\" = --version ] && { echo 0.0.0; exit 0; }\nPATH=\"/usr/bin:/bin:$PATH\" cp \"$FAKE_SCIP\" \"$4\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        fs::set_permissions(&sh, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }
    Fixture { dir, project, bin }
}

impl Fixture {
    /// Run `footnote <args>` with only `path` on PATH.
    fn run(&self, path: &Path, fake: Option<&Path>, args: &[&str]) -> Value {
        let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", self.dir.path().join("state"))
            .env(
                "FOOTNOTE_POLICY_FILE",
                self.dir.path().join("no-policy.json"),
            )
            .env("PATH", path)
            .args(args);
        if let Some(f) = fake {
            c.env("FAKE_SCIP", f);
        }
        let out = c.output().unwrap();
        serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
    }
    fn index(&self, path: &Path, fake: Option<&Path>, extra: &[&str]) -> Value {
        let p = self.project.to_str().unwrap();
        let mut args = vec!["code-index", p];
        args.extend_from_slice(extra);
        self.run(path, fake, &args)
    }
}

#[test]
fn a_missing_indexer_is_reported_with_its_install_command_and_nothing_is_installed() {
    let t = fixture();
    let nothing = t.dir.path().join("empty");
    let v = t.index(&nothing, None, &["--deep"]);
    let ix = &v["deep"]["indexers"][0];
    assert_eq!(
        (ix["status"].as_str(), ix["install"].as_str()),
        (
            Some("missing"),
            Some("npm install -g @sourcegraph/scip-typescript")
        ),
        "{v}"
    );
    assert_eq!(ix["licence"], "Apache-2.0", "{v}");
    assert_eq!(v["deep"]["compiler_graph"], false, "{v}");
    assert_eq!(
        v["indexer"], "tree-sitter",
        "the parser graph is still built: {v}"
    );
}

#[test]
fn a_malformed_index_is_reported_not_fatal() {
    let t = fixture();
    let bad = t.dir.path().join("bad.scip");
    fs::write(&bad, [0x12, 0x05, 1]).unwrap();
    let v = t.index(&t.bin, Some(&bad), &["--deep"]);
    assert_eq!(v["deep"]["indexers"][0]["status"], "ready", "{v}");
    assert_eq!(v["deep"]["compiler_graph"], false, "{v}");
    assert_eq!(v["indexer"], "tree-sitter", "{v}");
    assert!(v["upgrade"].as_str().unwrap().contains("truncated"), "{v}");
}

#[test]
fn an_index_over_the_size_limit_is_refused() {
    // The limit is 512 MiB, too big to write here: a sparse file of that size plus a byte is enough, the file is never read.
    let t = fixture();
    let big = t.dir.path().join("big.scip");
    fs::File::create(&big)
        .unwrap()
        .set_len((512 << 20) + 1)
        .unwrap();
    let v = t.index(&t.bin, Some(&big), &["--deep"]);
    assert_eq!(v["indexer"], "tree-sitter", "{v}");
    assert!(v["upgrade"].as_str().unwrap().contains("limit"), "{v}");
}

#[test]
fn a_valid_deep_index_is_the_compiler_graph_and_without_deep_nothing_changes() {
    let t = fixture();
    let good = t.dir.path().join("good.scip");
    fs::write(&good, index()).unwrap();
    let v = t.index(&t.bin, Some(&good), &["--deep"]);
    assert_eq!(
        (v["origin"].as_str(), v["indexer"].as_str()),
        (Some("compiler"), Some("scip-typescript")),
        "{v}"
    );
    assert_eq!(v["deep"]["compiler_graph"], true, "{v}");
    let found = t.run(
        &t.bin,
        None,
        &["code", "f", "--root", t.project.to_str().unwrap()],
    );
    assert_eq!(found["origin"], "compiler", "{found}");
    assert_eq!(found["symbols"][0]["callers"][0]["name"], "g", "{found}");
    // Default run, no indexer on PATH: the answer has no `deep` key and the same tier as before this flag existed.
    let plain = t.index(&t.dir.path().join("empty"), None, &[]);
    assert!(plain.get("deep").is_none(), "{plain}");
    assert_eq!(plain["origin"], "resolved", "{plain}");
}

#[test]
fn yes_and_no_install_need_deep_and_exclude_each_other() {
    let t = fixture();
    let p = t.project.to_str().unwrap();
    for args in [
        vec!["code-index", p, "--yes"],
        vec!["code-index", p, "--deep", "--yes", "--no-install"],
    ] {
        let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
            .args(&args)
            .output()
            .unwrap();
        assert!(!out.status.success(), "{args:?}");
    }
}
