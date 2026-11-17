#![cfg(feature = "encryption")]

use footnote_search::store;
use rusqlite::Connection;

#[test]
fn encrypted_database_reopens_only_with_the_right_key() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.db");
    let key = [0x5a; 32];
    let c = store::open_encrypted(&path, true, &key).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "temp_store", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    let cipher: String = c
        .pragma_query_value(None, "cipher_version", |r| r.get(0))
        .unwrap();
    let mut version = cipher.split('.');
    assert_eq!(version.next(), Some("4"), "unreviewed SQLCipher: {cipher}");
    assert!(
        version.next().unwrap().parse::<u32>().unwrap() >= 20,
        "outdated SQLCipher: {cipher}"
    );
    c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:fixture','hash',1,'indexed','memory')", []).unwrap();
    drop(c);

    let original = std::fs::read(&path).unwrap();
    assert!(!original.starts_with(b"SQLite format 3\0"));
    assert!(
        !original
            .windows(b"memory:fixture".len())
            .any(|w| w == b"memory:fixture")
    );
    assert!(store::open_encrypted(&path, false, &[0x6b; 32]).is_err());
    assert!(store::open_encrypted(&path, true, &[0x6b; 32]).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(store::open(&path, false).is_err());
    assert!(
        Connection::open(&path)
            .unwrap()
            .query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
            .is_err()
    );
    let c = store::open_encrypted(&path, false, &key).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        store::SCHEMA
    );
    assert_eq!(
        c.pragma_query_value(None, "integrity_check", |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn encrypted_opener_does_not_accept_existing_plaintext() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("plain.db");
    drop(store::open(&path, true).unwrap());
    assert!(store::open_encrypted(&path, true, &[7; 32]).is_err());
    assert!(store::open(&path, false).is_ok());
}

#[test]
fn encrypted_backup_and_restore_preserve_data_and_refuse_overwrites() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.db");
    let backup = temp.path().join("backup.db");
    let restored = temp.path().join("restored.db");
    let key = [0x41; 32];
    let c = store::open_encrypted(&source, true, &key).unwrap();
    c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:fixture','hash',1,'indexed','memory')", []).unwrap();
    c.pragma_update(None, "application_id", 12345).unwrap();
    c.pragma_update(None, "auto_vacuum", 1).unwrap();
    c.execute_batch("VACUUM").unwrap();
    drop(c);
    let source_before = std::fs::read(&source).unwrap();

    let missing = temp.path().join("missing.db");
    assert!(store::copy_encrypted(&missing, &backup, &key).is_err());
    assert!(!missing.exists() && !backup.exists());
    assert!(store::copy_encrypted(&source, &backup, &[0x42; 32]).is_err());
    assert!(!backup.exists());
    store::copy_encrypted(&source, &backup, &key).unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), source_before);
    let backup_bytes = std::fs::read(&backup).unwrap();
    assert!(!backup_bytes.starts_with(b"SQLite format 3\0"));
    assert!(store::open(&backup, false).is_err());
    assert!(store::open_encrypted(&backup, false, &[0x42; 32]).is_err());
    assert!(store::copy_encrypted(&source, &backup, &key).is_err());
    assert_eq!(std::fs::read(&backup).unwrap(), backup_bytes);

    store::copy_encrypted(&backup, &restored, &key).unwrap();
    let c = store::open_encrypted(&restored, false, &key).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        store::SCHEMA
    );
    assert_eq!(
        c.pragma_query_value(None, "application_id", |r| r.get::<_, i64>(0))
            .unwrap(),
        12345
    );
    assert_eq!(
        c.pragma_query_value(None, "auto_vacuum", |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 3);
}

#[test]
fn encrypted_backup_includes_committed_wal_data() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("wal.db");
    let backup = temp.path().join("backup.db");
    let key = [0x43; 32];
    let c = store::open_encrypted(&source, true, &key).unwrap();
    let mode: String = c
        .query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    c.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:wal','hash',1,'indexed','memory')", []).unwrap();
    assert!(temp.path().join("wal.db-wal").exists());
    store::copy_encrypted(&source, &backup, &key).unwrap();
    let restored = store::open_encrypted(&backup, false, &key).unwrap();
    assert_eq!(
        restored
            .query_row(
                "SELECT count(*) FROM sources WHERE path='memory:wal'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[cfg(windows)]
#[test]
fn encrypted_backup_reads_readonly_source_with_special_characters_in_path() {
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().join("fixture # % é");
    std::fs::create_dir(&folder).unwrap();
    let source = folder.join("readonly # %.db");
    let backup = folder.join("backup # %.db");
    let key = [0x57; 32];
    let c = store::open_encrypted(&source, true, &key).unwrap();
    c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:readonly','hash',1,'indexed','memory')", []).unwrap();
    drop(c);
    let original = std::fs::read(&source).unwrap();
    let original_permissions = std::fs::metadata(&source).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(&source, readonly).unwrap();
    let write_denied = std::fs::OpenOptions::new()
        .write(true)
        .open(&source)
        .is_err();
    let result = store::copy_encrypted(&source, &backup, &key);
    std::fs::set_permissions(&source, original_permissions).unwrap();
    assert!(write_denied);
    result.unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let restored = store::open_encrypted(&backup, false, &key).unwrap();
    assert_eq!(
        restored
            .query_row(
                "SELECT count(*) FROM sources WHERE path='memory:readonly'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn encrypted_backup_excludes_uncommitted_half_transactions() {
    use std::sync::mpsc;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("concurrent.db");
    let key = [0x58; 32];
    let c = store::open_encrypted(&source, true, &key).unwrap();
    let mode: String = c
        .query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    c.execute_batch("CREATE TABLE left_rows(id INTEGER PRIMARY KEY); CREATE TABLE right_rows(id INTEGER PRIMARY KEY);").unwrap();
    drop(c);
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let writer_path = source.clone();
    let writer = std::thread::spawn(move || {
        let mut c = store::open_encrypted(&writer_path, true, &key).unwrap();
        for id in 0..5 {
            let tx = c.transaction().unwrap();
            tx.execute("INSERT INTO left_rows VALUES (?1)", [id])
                .unwrap();
            started_tx.send(id).unwrap();
            resume_rx.recv().unwrap();
            tx.execute("INSERT INTO right_rows VALUES (?1)", [id])
                .unwrap();
            tx.commit().unwrap();
        }
    });
    for expected in 0..5 {
        assert_eq!(started_rx.recv().unwrap(), expected);
        let backup = temp.path().join(format!("snapshot-{expected}.db"));
        store::copy_encrypted(&source, &backup, &key).unwrap();
        let snapshot = store::open_encrypted(&backup, false, &key).unwrap();
        let left: i64 = snapshot
            .query_row("SELECT count(*) FROM left_rows", [], |r| r.get(0))
            .unwrap();
        let right: i64 = snapshot
            .query_row("SELECT count(*) FROM right_rows", [], |r| r.get(0))
            .unwrap();
        assert_eq!((left, right), (expected, expected));
        resume_tx.send(()).unwrap();
    }
    writer.join().unwrap();
}
