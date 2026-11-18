//! What some tests need that a fresh checkout lacks (#206): such a test skips with the reason instead of failing.
//! `python scripts/setup_tests.py` prepares everything.
#![allow(dead_code)] // each test crate uses only some of these
use std::path::Path;

const SETUP: &str = "run `python scripts/setup_tests.py` to prepare it";

/// A local embedding model under `artifacts/models/<name>`.
pub fn model(name: &str) -> bool {
    let ok = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts/models")
        .join(name)
        .join("manifest.json")
        .exists();
    if !ok {
        eprintln!("model {name} not installed: skipped ({SETUP})");
    }
    ok
}

/// The Python environment of the PDF and Office extraction helper (`.venv`).
pub fn extraction() -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ok =
        root.join(".venv/Scripts/python.exe").exists() || root.join(".venv/bin/python").exists();
    if !ok {
        eprintln!("extraction environment (.venv) missing: skipped ({SETUP})");
    }
    ok
}
