//! `footnote docs bundle|install`: signed prebuilt indexes (#309). No network: bundles are files.
use assert_cmd::Command;
use serde_json::Value;
use std::{fs, path::Path};

struct Env {
    _t: tempfile::TempDir,
    root: std::path::PathBuf,
}

fn footnote(state: &Path, args: &[&str]) -> std::process::Output {
    Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .args(args)
        .output()
        .unwrap()
}
fn ok(state: &Path, args: &[&str]) -> Value {
    let o = footnote(state, args);
    assert!(o.status.success(), "{args:?}: {o:?}");
    serde_json::from_slice(&o.stdout).unwrap()
}
fn err(state: &Path, args: &[&str]) -> String {
    let o = footnote(state, args);
    assert!(!o.status.success(), "{args:?} should fail");
    String::from_utf8_lossy(&o.stderr).into_owned() + &String::from_utf8_lossy(&o.stdout)
}

/// A source machine with one cached library, a signing key, and its built bundle.
fn setup(redistributable: bool) -> (Env, std::path::PathBuf, String) {
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = t.path().to_path_buf();
    let src = root.join("src-state");
    let notes = root.join("notes");
    fs::create_dir(&notes).unwrap();
    fs::write(
        notes.join("a.md"),
        "# Pool\n## Sizing\nConnections are reused from the pool.\n",
    )
    .unwrap();
    ok(
        &src,
        &[
            "docs",
            "add",
            "--from",
            notes.to_str().unwrap(),
            "--site",
            "acme",
            "--version",
            "1.0",
            "--no-embed",
        ],
    );
    let key = root.join("key.hex");
    let public = ok(&src, &["docs", "keygen", key.to_str().unwrap()])["public_key"]
        .as_str()
        .unwrap()
        .to_string();
    ok(
        &src,
        &[
            "docs",
            "licence",
            "acme",
            "--name",
            "MIT",
            "--redistributable",
            if redistributable { "true" } else { "false" },
            "--url",
            "https://docs.acme.example",
        ],
    );
    (Env { _t: t, root }, src, public)
}

fn build(env: &Env, src: &Path) -> std::path::PathBuf {
    let out = env.root.join("acme.fnbundle");
    ok(
        src,
        &[
            "docs",
            "bundle",
            "acme",
            "--key",
            env.root.join("key.hex").to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    out
}

fn dest(env: &Env) -> std::path::PathBuf {
    env.root.join("dst-state")
}

#[test]
fn good_bundle_installs_and_lists() {
    let (env, src, public) = setup(true);
    let b = build(&env, &src);
    let d = dest(&env);
    let r = ok(
        &d,
        &[
            "docs",
            "install",
            b.to_str().unwrap(),
            "--trust-key",
            &public,
        ],
    );
    assert_eq!(r["installed"], "acme", "{r}");
    let l = ok(&d, &["docs", "list"]);
    assert_eq!(l["docs"][0]["site"], "acme", "{l}");
    assert_eq!(l["docs"][0]["pages"], 1, "{l}");
    assert_eq!(ok(&d, &["corpora"])["corpora"].as_array().unwrap().len(), 1);
    // Installing again does not overwrite.
    assert!(
        err(
            &d,
            &[
                "docs",
                "install",
                b.to_str().unwrap(),
                "--trust-key",
                &public
            ]
        )
        .contains("already installed")
    );
}

#[test]
fn tampered_database_manifest_and_wrong_key_are_rejected() {
    let (env, src, public) = setup(true);
    let b = build(&env, &src);
    let d = dest(&env);
    let good = fs::read(&b).unwrap();
    let t = |bytes: Vec<u8>, key: &str| {
        let p = env.root.join("t.fnbundle");
        fs::write(&p, bytes).unwrap();
        err(
            &d,
            &["docs", "install", p.to_str().unwrap(), "--trust-key", key],
        )
    };
    // Last byte of the database.
    let mut db = good.clone();
    *db.last_mut().unwrap() ^= 1;
    assert!(t(db, &public).contains("hash does not match"));
    // A byte inside the manifest (the sha256 text), signature no longer fits.
    let mut mf = good.clone();
    mf[20] ^= 1;
    assert!(t(mf, &public).contains("signature"));
    // Signed by a key nobody trusts.
    let other = ok(
        &src,
        &[
            "docs",
            "keygen",
            env.root.join("other.hex").to_str().unwrap(),
        ],
    )["public_key"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(t(good, &other).contains("signature"));
    assert!(!d.join("docs/acme/1.0.db").exists());
}

#[test]
fn manifest_names_cannot_leave_the_cache() {
    let (env, src, public) = setup(true);
    let b = build(&env, &src);
    // Re-sign a manifest whose library climbs out, with a key the destination trusts: still refused.
    let bytes = fs::read(&b).unwrap();
    let len = u32::from_be_bytes(bytes[10..14].try_into().unwrap()) as usize;
    let m = String::from_utf8(bytes[14..14 + len].to_vec())
        .unwrap()
        .replace("\"acme\"", "\"../../evil\"");
    let key = env.root.join("key.hex");
    let forged = env.root.join("forged.fnbundle");
    // Build the forged file through the same signer: write a bundle by hand using `docs bundle`'s layout.
    let sig = sign(&key, m.as_bytes());
    let mut out = bytes[..10].to_vec();
    out.extend((m.len() as u32).to_be_bytes());
    out.extend(m.as_bytes());
    out.extend(sig);
    out.extend(&bytes[14 + len + 64..]);
    fs::write(&forged, out).unwrap();
    let d = dest(&env);
    assert!(
        err(
            &d,
            &[
                "docs",
                "install",
                forged.to_str().unwrap(),
                "--trust-key",
                &public
            ]
        )
        .contains("plain cache name")
    );
    assert!(
        !env.root.join("evil").exists()
            && !d.join("docs").join("..").join("..").join("evil").exists()
    );
}

/// Sign with the key file (test helper through ring, the same crate the binary uses).
fn sign(key: &Path, msg: &[u8]) -> Vec<u8> {
    let raw = fs::read_to_string(key).unwrap();
    let der: Vec<u8> = (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect();
    ring::signature::Ed25519KeyPair::from_pkcs8(&der)
        .unwrap()
        .sign(msg)
        .as_ref()
        .to_vec()
}

#[test]
fn non_redistributable_or_unrecorded_licence_is_refused_at_build() {
    let (env, src, _) = setup(false);
    let key = env.root.join("key.hex");
    let out = env.root.join("x.fnbundle");
    let msg = err(
        &src,
        &[
            "docs",
            "bundle",
            "acme",
            "--key",
            key.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert!(msg.contains("does not allow redistribution"), "{msg}");
    assert!(!out.exists());
    // No licence recorded at all.
    fs::remove_file(src.join("docs/acme/1.0.licence.json")).unwrap();
    let msg = err(
        &src,
        &[
            "docs",
            "bundle",
            "acme",
            "--key",
            key.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert!(msg.contains("no licence recorded"), "{msg}");
}
