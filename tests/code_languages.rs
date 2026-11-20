//! #110: Footnote Code picks the indexer from the project's files, never writes into the project, and says plainly what it cannot
//! read.
use assert_cmd::Command;
use serde_json::Value;
use std::fs;

fn footnote(state: &std::path::Path) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
    c.env("FOOTNOTE_STATE_DIR", state);
    c
}

#[test]
fn a_typescript_project_gets_definitions_and_callers() {
    let probe = if cfg!(windows) {
        "scip-typescript.cmd"
    } else {
        "scip-typescript"
    };
    if std::process::Command::new(probe)
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("scip-typescript not installed: skipped");
        return;
    }
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = t.path().join("tsfix");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("tsconfig.json"), r#"{"compilerOptions":{"strict":true,"target":"ES2022","module":"ESNext"},"include":["src"]}"#).unwrap();
    fs::write(root.join("src/math.ts"), "export function total(xs: number[]): number {\n  return xs.reduce((a, b) => a + b, 0);\n}\n").unwrap();
    fs::write(root.join("src/report.ts"), "import { total } from \"./math\";\n\nexport function report(): string {\n  return `sum=${total([1, 2, 3])}`;\n}\n").unwrap();
    let before: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();

    let state = t.path().join("state");
    let out = footnote(&state)
        .arg("code-index")
        .arg(&root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["indexer"], "scip-typescript", "{v}");

    let out = footnote(&state)
        .args(["code", "total", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let s = &v["symbols"][0];
    assert_eq!(
        (s["path"].as_str(), s["kind"].as_str()),
        (Some("src/math.ts"), Some("function")),
        "{v}"
    );
    assert!(s["callers"].to_string().contains("report.ts"), "{v}");

    let after: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(before, after, "the project folder is never written to");
}

#[test]
fn what_code_cannot_read_is_said_plainly() {
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let js = t.path().join("js");
    fs::create_dir(&js).unwrap();
    fs::write(js.join("package.json"), "{}").unwrap();
    let out = footnote(&state)
        .arg("code-index")
        .arg(&js)
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        text.contains("tsconfig.json") && text.contains("never writes"),
        "{text}"
    );
    assert!(!js.join("tsconfig.json").exists());

    let none = t.path().join("none");
    fs::create_dir(&none).unwrap();
    let out = footnote(&state)
        .arg("code-index")
        .arg(&none)
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        text.contains("Rust, TypeScript/JavaScript and Python"),
        "{text}"
    );
}

#[test]
fn python_windows_loader_corrects_only_the_broken_constructor_and_restores_it() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Node not installed: Python compatibility test skipped");
        return;
    }
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let entry = t.path().join("entry.cjs");
    fs::write(
        &entry,
        r#"
const assert = require('node:assert/strict');
assert.equal('pkg\\math.py/path'.replace(new RegExp('\\', 'g'), '/'), 'pkg/math.py/path');
assert.equal('pkg\\math.py/path'.replace(/[\\/]/g, '/'), 'pkg/math.py/path');
assert.equal('abc'.replace(new RegExp('b', 'g'), 'x'), 'axc');
assert.throws(() => new RegExp('[', 'g'), SyntaxError);
process.nextTick(() => {
  assert.equal(global.RegExp, /x/.constructor);
  assert.throws(() => new RegExp('\\', 'g'), SyntaxError);
});
"#,
    )
    .unwrap();
    let out = std::process::Command::new("node")
        .args([
            "-e",
            include_str!("../crates/code/src/scip_python_windows.cjs"),
        ])
        .arg(entry)
        .output()
        .expect("Node is required to test the Python compatibility loader");
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn a_python_project_gets_normalized_paths_and_cross_file_references() {
    let probe = if cfg!(windows) {
        "scip-python.cmd"
    } else {
        "scip-python"
    };
    if std::process::Command::new(probe)
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("scip-python not installed: skipped");
        return;
    }
    // scip-python needs a real interpreter: `python3` as the Windows Store stub does not count (#332); the indexer falls back to `python`.
    let runs = |p: &str| {
        std::process::Command::new(p)
            .args(["-c", "pass"])
            .output()
            .is_ok_and(|o| o.status.success())
    };
    if !["python3", "python"].iter().any(|p| runs(p)) {
        eprintln!("no real python interpreter (python3 is the Microsoft Store stub?): skipped");
        return;
    }
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = t.path().join("python project with spaces");
    fs::create_dir_all(root.join("pkg")).unwrap();
    fs::write(
        root.join("pyproject.toml"),
        "[project]\nname = \"pyfix\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(root.join("pkg/__init__.py"), "").unwrap();
    fs::write(
        root.join("pkg/math.py"),
        "def total(xs: list[int]) -> int:\n    return sum(xs)\n",
    )
    .unwrap();
    fs::write(
        root.join("pkg/report.py"),
        "from pkg.math import total\n\ndef report() -> int:\n    return total([1, 2, 3])\n",
    )
    .unwrap();
    let state = t.path().join("state");
    let out = footnote(&state)
        .arg("code-index")
        .arg(&root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["indexer"], "scip-python");
    let out = footnote(&state)
        .args(["code", "total", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let s = &v["symbols"][0];
    assert_eq!(s["path"], "pkg/math.py", "{v}");
    assert!(s["callers"].to_string().contains("pkg/report.py"), "{v}");
    assert!(!root.join("index.scip").exists());
}
