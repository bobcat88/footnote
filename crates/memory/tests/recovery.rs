#![cfg(feature = "encryption")]

use footnote_memory::recovery;
use footnote_search::store;

#[test]
fn recovery_text_round_trips_and_unlocks_a_new_database() {
    let key = recovery::generate().unwrap();
    let text = recovery::encode(&key);
    let recovered = recovery::decode(&text).unwrap();
    assert_eq!(&*key, &*recovered);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");
    let c = store::open_encrypted(&path, true, &key).unwrap();
    c.execute("INSERT INTO sources(root,path,sha256,revision,status,extractor) VALUES ('','memory:fixture','hash',1,'indexed','memory')", []).unwrap();
    drop(c);
    let c = store::open_encrypted(&path, false, &recovered).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn malformed_or_changed_recovery_material_is_rejected_without_echoing_it() {
    let valid = recovery::encode(&[0x23; 32]);
    assert_eq!(
        valid.as_str(),
        concat!(
            "footnote-recovery-v1:",
            "2323232323232323232323232323232323232323232323232323232323232323",
            ":25ee140ac501c496"
        )
    );
    let mut changed = valid.to_string();
    changed.replace_range(21..22, "4");
    for bad in [
        changed,
        valid.replace("v1", "v2"),
        valid.to_uppercase(),
        format!("{}:extra", *valid),
        "é".repeat(50),
    ] {
        let err = match recovery::decode(&bad) {
            Ok(_) => panic!("invalid recovery material accepted"),
            Err(e) => e.to_string(),
        };
        assert!(!err.contains(&bad));
    }
}

#[test]
fn recovery_qr_scans_to_the_same_key() {
    let key = [0x23; 32];
    let (width, modules) = recovery::qr(&key).unwrap();
    let scale = 8;
    let edge = (width + 8) * scale;
    let mut image = vec![255_u8; edge * edge];
    for y in 0..width {
        for x in 0..width {
            if modules[y * width + x] == 1 {
                for py in (y + 4) * scale..(y + 5) * scale {
                    for px in (x + 4) * scale..(x + 5) * scale {
                        image[py * edge + px] = 0;
                    }
                }
            }
        }
    }
    let mut scanner = quircs::Quirc::default();
    let codes: Vec<_> = scanner.identify(edge, edge, &image).collect();
    assert_eq!(codes.len(), 1);
    let decoded = codes.into_iter().next().unwrap().unwrap().decode().unwrap();
    assert_eq!(
        decoded.payload.as_slice(),
        recovery::encode(&key).as_bytes()
    );
    let recovered = recovery::decode(std::str::from_utf8(&decoded.payload).unwrap()).unwrap();
    assert_eq!(&*recovered, &key);
}

#[test]
fn encrypted_restore_preserves_memory_and_recall() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.db");
    let backup = dir.path().join("backup.db");
    let restored = dir.path().join("restored.db");
    let key = recovery::generate().unwrap();
    let mut c = store::open_encrypted(&source, true, &key).unwrap();
    let item = serde_json::json!([{"text":"Synthetic marmot prefers encrypted backups", "kind":"fact", "scope":"global", "observed_at":"2026-11-16", "tags":["fixture"]}]);
    footnote_memory::remember(&mut c, &item, &[], "cli").unwrap();
    assert!(!dir.path().join("source.db.tantivy").exists());
    let before = footnote_memory::recall(&c, "marmot backups", None, 10, &[], &[]).unwrap();
    assert_eq!(before["mode"], "lexical_bm25");
    drop(c);
    store::copy_encrypted(&source, &backup, &key).unwrap();
    let recovered = recovery::decode(&recovery::encode(&key)).unwrap();
    store::copy_encrypted(&backup, &restored, &recovered).unwrap();
    let c = store::open_encrypted(&restored, false, &recovered).unwrap();
    let results = footnote_memory::recall(&c, "marmot backups", None, 10, &[], &[]).unwrap();
    assert_eq!(results["mode"], "lexical_bm25");
    let memories = results["memories"].as_array().unwrap();
    assert_eq!(memories.len(), 1);
    assert_eq!(
        memories[0]["text"],
        "Synthetic marmot prefers encrypted backups"
    );
    assert_eq!(memories[0]["tags"], serde_json::json!(["fixture"]));
    assert!(!dir.path().join("restored.db.tantivy").exists());
}

#[test]
fn encrypted_memory_never_reuses_or_deletes_an_old_disk_index() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("same.db");
    let index = dir.path().join("same.db.tantivy");
    let mut plain = store::open(&db, true).unwrap();
    let old = serde_json::json!([{"text":"Synthetic orchid", "kind":"fact", "scope":"global", "observed_at":"2026-11-16"}]);
    footnote_memory::remember(&mut plain, &old, &[], "cli").unwrap();
    let first = footnote_memory::recall(&plain, "orchid", None, 10, &[], &[]).unwrap();
    assert_eq!(first["memories"][0]["text"], "Synthetic orchid");
    assert!(index.is_dir());
    let metadata = std::fs::read(index.join("footnote.json")).unwrap();
    let sentinel = index.join("fixture-marker");
    std::fs::write(&sentinel, b"preserve").unwrap();
    drop(plain);
    std::fs::remove_file(&db).unwrap();
    let mut encrypted = store::open_encrypted(&db, true, &[0x24; 32]).unwrap();
    let new = serde_json::json!([{"text":"Synthetic marmot", "kind":"fact", "scope":"global", "observed_at":"2026-11-16"}]);
    footnote_memory::remember(&mut encrypted, &new, &[], "cli").unwrap();
    let result = footnote_memory::recall(&encrypted, "marmot", None, 10, &[], &[]).unwrap();
    assert_eq!(result["mode"], "lexical_bm25");
    assert_eq!(result["memories"][0]["text"], "Synthetic marmot");
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"preserve");
    assert_eq!(
        std::fs::read(index.join("footnote.json")).unwrap(),
        metadata
    );
    drop(encrypted);
    std::fs::remove_file(&db).unwrap();
    let mut replacement = store::open_encrypted(&db, true, &[0x24; 32]).unwrap();
    let newer = serde_json::json!([{"text":"Synthetic kingfisher", "kind":"fact", "scope":"global", "observed_at":"2026-11-16"}]);
    footnote_memory::remember(&mut replacement, &newer, &[], "cli").unwrap();
    let result = footnote_memory::recall(&replacement, "kingfisher", None, 10, &[], &[]).unwrap();
    assert_eq!(result["memories"][0]["text"], "Synthetic kingfisher");
    assert_eq!(
        std::fs::read(index.join("footnote.json")).unwrap(),
        metadata
    );
}
