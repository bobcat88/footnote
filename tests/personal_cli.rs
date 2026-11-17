#![cfg(all(feature = "encryption", windows))]

use assert_cmd::Command;
use std::{fmt::Write, path::Path};
use windows_sys::Win32::Security::Credentials::{CRED_TYPE_GENERIC, CredDeleteW};

struct Cleanup(Vec<u16>);
impl Cleanup {
    fn for_store(dir: &Path) -> Self {
        let id = std::fs::read(dir.join("key.id")).unwrap();
        assert_eq!(id.len(), 16);
        let mut target = String::from("Footnote/PersonalMemory/v1/");
        for byte in id {
            write!(&mut target, "{byte:02x}").unwrap();
        }
        Self(target.encode_utf16().chain(Some(0)).collect())
    }
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        unsafe { CredDeleteW(self.0.as_ptr(), CRED_TYPE_GENERIC, 0) };
    }
}

fn command(args: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn explicit_personal_workflow_keeps_secrets_out_of_ordinary_output() {
    let temp = tempfile::tempdir().unwrap();
    let store = temp.path().join("original");
    let backup = temp.path().join("backup.db");
    let restored = temp.path().join("restored");
    let rejected = temp.path().join("rejected");
    let s = store.as_os_str();
    let b = backup.as_os_str();
    let r = restored.as_os_str();
    let init = command(&["personal".as_ref(), "init".as_ref(), "--store".as_ref(), s]);
    assert!(init.status.success());
    let _cleanup_original = Cleanup::for_store(&store);
    assert!(!String::from_utf8_lossy(&init.stdout).contains("footnote-recovery-v1:"));
    let status = command(&[
        "personal".as_ref(),
        "status".as_ref(),
        "--store".as_ref(),
        s,
    ]);
    assert!(status.status.success());
    let item = serde_json::json!([
        {"text":"Synthetic river otter remembers the bridge", "kind":"fact", "scope":"global", "observed_at":"2026-11-16", "tags":["fixture"]},
        {"text":"Synthetic river otter has a project note", "kind":"fact", "scope":"test-project", "observed_at":"2026-11-16", "tags":["project"]}
    ]).to_string();
    let saved_memory = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "remember".as_ref(),
            "--store".as_ref(),
            s,
        ])
        .write_stdin(item)
        .output()
        .unwrap();
    assert!(saved_memory.status.success());
    assert!(!String::from_utf8_lossy(&saved_memory.stdout).contains("river otter"));
    assert!(!store.join("memory.db.tantivy").exists());
    let recall = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "recall".as_ref(),
            "--store".as_ref(),
            s,
        ])
        .write_stdin("river otter")
        .output()
        .unwrap();
    assert!(recall.status.success());
    let value: serde_json::Value = serde_json::from_slice(&recall.stdout).unwrap();
    assert_eq!(value["mode"], "lexical_bm25");
    assert_eq!(
        value["memories"][0]["text"],
        "Synthetic river otter remembers the bridge"
    );
    assert_eq!(value["memories"].as_array().unwrap().len(), 1);
    assert!(!store.join("memory.db.tantivy").exists());
    let scoped = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "recall".as_ref(),
            "--store".as_ref(),
            s,
            "--project".as_ref(),
            "test-project".as_ref(),
        ])
        .write_stdin("river otter")
        .output()
        .unwrap();
    assert!(scoped.status.success());
    let scoped: serde_json::Value = serde_json::from_slice(&scoped.stdout).unwrap();
    assert_eq!(scoped["memories"].as_array().unwrap().len(), 2);
    let oversized = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "remember".as_ref(),
            "--store".as_ref(),
            s,
        ])
        .write_stdin(vec![b' '; 512 * 1024 + 1])
        .output()
        .unwrap();
    assert!(!oversized.status.success());
    let invalid_query = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "recall".as_ref(),
            "--store".as_ref(),
            s,
        ])
        .write_stdin("x".repeat(513))
        .output()
        .unwrap();
    assert!(!invalid_query.status.success());
    let secret = command(&[
        "personal".as_ref(),
        "recovery-text".as_ref(),
        "--store".as_ref(),
        s,
    ]);
    assert!(secret.status.success());
    let text = String::from_utf8(secret.stdout).unwrap();
    assert!(text.starts_with("footnote-recovery-v1:"));
    let no_qr_to_pipe = command(&[
        "personal".as_ref(),
        "recovery-qr".as_ref(),
        "--store".as_ref(),
        s,
    ]);
    assert!(!no_qr_to_pipe.status.success());
    assert!(no_qr_to_pipe.stdout.is_empty());
    let saved = command(&[
        "personal".as_ref(),
        "backup".as_ref(),
        "--store".as_ref(),
        s,
        "--output".as_ref(),
        b,
    ]);
    assert!(saved.status.success());
    let invalid = "sensitive-fixture-invalid-recovery";
    let failed = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "restore".as_ref(),
            "--store".as_ref(),
            rejected.as_os_str(),
            "--backup".as_ref(),
            b,
        ])
        .write_stdin(invalid)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(!rejected.exists());
    assert!(!String::from_utf8_lossy(&failed.stdout).contains(invalid));
    assert!(!String::from_utf8_lossy(&failed.stderr).contains(invalid));
    let backup_permissions = std::fs::metadata(&backup).unwrap().permissions();
    let mut readonly = backup_permissions.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(&backup, readonly).unwrap();
    let restore = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "restore".as_ref(),
            "--store".as_ref(),
            r,
            "--backup".as_ref(),
            b,
        ])
        .write_stdin(text.clone())
        .output();
    std::fs::set_permissions(&backup, backup_permissions).unwrap();
    let restore = restore.unwrap();
    assert!(restore.status.success());
    let _cleanup_restored = Cleanup::for_store(&restored);
    assert!(!String::from_utf8_lossy(&restore.stdout).contains(text.trim()));
    let status = command(&[
        "personal".as_ref(),
        "status".as_ref(),
        "--store".as_ref(),
        r,
    ]);
    assert!(status.status.success());
    let recall = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .args([
            "personal".as_ref(),
            "recall".as_ref(),
            "--store".as_ref(),
            r,
        ])
        .write_stdin("river otter")
        .output()
        .unwrap();
    assert!(recall.status.success());
    let value: serde_json::Value = serde_json::from_slice(&recall.stdout).unwrap();
    assert_eq!(
        value["memories"][0]["text"],
        "Synthetic river otter remembers the bridge"
    );
    assert!(!restored.join("memory.db.tantivy").exists());
    let bytes = std::fs::read(restored.join("memory.db")).unwrap();
    assert!(
        !bytes
            .windows(b"river otter".len())
            .any(|w| w == b"river otter")
    );
    let duplicate = command(&[
        "personal".as_ref(),
        "backup".as_ref(),
        "--store".as_ref(),
        s,
        "--output".as_ref(),
        b,
    ]);
    assert!(!duplicate.status.success());
}

/// #128: once Memory uses an encrypted store, every ordinary path reads and writes it (CLI, MCP, status), nothing in the file is
/// plain text, and stopping goes back to the plain database without touching either.
#[test]
fn memory_works_end_to_end_on_an_encrypted_store() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let store = temp.path().join("store");
    let db = store.join("memory.db");
    let footnote = || {
        let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
        c.env("FOOTNOTE_STATE_DIR", &state);
        c
    };
    let json = |o: std::process::Output| -> serde_json::Value {
        assert!(o.status.success(), "{o:?}");
        serde_json::from_slice(&o.stdout).unwrap()
    };
    footnote()
        .args(["personal", "init", "--store"])
        .arg(&store)
        .assert()
        .success();
    let _cleanup = Cleanup::for_store(&store);
    let used = json(
        footnote()
            .args(["personal", "use", "--store"])
            .arg(&store)
            .output()
            .unwrap(),
    );
    assert_eq!(used["encrypted"], true);

    // The ordinary commands, on the store's path, go through the key.
    let note = serde_json::json!([{"text": "Synthetic heron nests by the old mill", "kind": "fact", "scope": "global", "observed_at": "2026-11-17"}]);
    footnote()
        .arg("--db")
        .arg(&db)
        .arg("remember")
        .write_stdin(note.to_string())
        .assert()
        .success();
    let found = json(
        footnote()
            .arg("--db")
            .arg(&db)
            .args(["memory-recall", "--", "heron mill"])
            .output()
            .unwrap(),
    );
    assert!(found.to_string().contains("Synthetic heron"), "{found}");
    let status = json(footnote().arg("status").output().unwrap());
    assert_eq!(status["products"]["memory"]["encrypted"], true, "{status}");

    // An agent through MCP (in-process: no daemon in this state folder).
    let call = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"footnote_recall","arguments":{"query":"heron"}}});
    let mcp = footnote()
        .arg("--db")
        .arg(&db)
        .arg("mcp")
        .write_stdin(format!("{call}\n"))
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&mcp.stdout).contains("Synthetic heron"),
        "{mcp:?}"
    );

    // Nothing readable on disk.
    let raw = std::fs::read(&db).unwrap();
    assert!(
        !raw.windows(6).any(|w| w == b"heron "),
        "memory text found in plain text"
    );
    assert!(
        !store.join("memory.db.tantivy").exists(),
        "no keyword index on disk for an encrypted store"
    );

    let back = json(
        footnote()
            .args(["personal", "stop-using"])
            .output()
            .unwrap(),
    );
    assert_eq!(back["encrypted"], false);
    assert_eq!(
        json(footnote().arg("status").output().unwrap())["products"]["memory"]["encrypted"],
        false
    );
    assert!(db.exists(), "the encrypted store stays");
}
