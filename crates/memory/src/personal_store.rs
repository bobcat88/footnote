//! Explicit new encrypted personal-store lifecycle. Not yet wired into application commands.
use crate::{recovery, windows_keystore};
use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

const READY: &[u8] = b"footnote-personal-v1\n";
const KEY_ID: &str = "key.id";
const DB: &str = "memory.db";
const MARKER: &str = "ready";

trait Keys {
    fn insert(&self, key: &[u8; 32]) -> Result<[u8; 16]>;
    fn load(&self, id: &[u8; 16]) -> Result<Option<Zeroizing<[u8; 32]>>>;
}

struct WindowsKeys;
impl Keys for WindowsKeys {
    fn insert(&self, key: &[u8; 32]) -> Result<[u8; 16]> {
        Ok(*windows_keystore::insert(key)?.as_bytes())
    }
    fn load(&self, id: &[u8; 16]) -> Result<Option<Zeroizing<[u8; 32]>>> {
        windows_keystore::load(&windows_keystore::KeyId::from_bytes(*id))
    }
}

fn write_new(path: &Path, contents: &[u8]) -> Result<()> {
    footnote_core::no_links(path)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(())
}

fn read_exact_file<const N: usize>(path: &Path) -> Result<[u8; N]> {
    footnote_core::no_links(path)?;
    let mut file = fs::File::open(path)?;
    let mut result = [0_u8; N];
    file.read_exact(&mut result)?;
    let mut extra = [0_u8; 1];
    if file.read(&mut extra)? != 0 {
        bail!("personal store metadata has an invalid length");
    }
    Ok(result)
}

/// Create a new directory and return the recovery secret. Existing paths are never overwritten.
pub fn create_new(dir: &Path) -> Result<Zeroizing<String>> {
    create_new_with(dir, &WindowsKeys)
}

fn create_new_with(dir: &Path, keys: &impl Keys) -> Result<Zeroizing<String>> {
    footnote_core::no_links(dir)?;
    fs::create_dir(dir).context("create new personal store directory")?;
    let key = recovery::generate()?;
    let id = keys.insert(&key)?;
    write_new(&dir.join(KEY_ID), &id)?;
    let c = footnote_search::store::open_encrypted(&dir.join(DB), true, &key)?;
    drop(c);
    write_new(&dir.join(MARKER), READY)?;
    Ok(recovery::encode(&key))
}

/// Open only a completed store. Missing metadata, key or database is an error, never a create request.
pub fn open_existing(dir: &Path, write: bool) -> Result<Connection> {
    open_existing_with(dir, write, &WindowsKeys)
}

/// Copy an existing protected store to a new encrypted backup file.
pub fn backup(dir: &Path, output: &Path) -> Result<()> {
    let key = unlock(dir, &WindowsKeys)?;
    drop(footnote_search::store::open_encrypted_existing(
        &dir.join(DB),
        false,
        &key,
    )?);
    footnote_search::store::copy_encrypted(&dir.join(DB), output, &key)
}

/// Restore an encrypted backup to a new store and register its recovered key locally.
pub fn restore_new(dir: &Path, backup: &Path, recovery_text: &str) -> Result<()> {
    restore_new_with(dir, backup, recovery_text, &WindowsKeys)
}

fn restore_new_with(
    dir: &Path,
    backup: &Path,
    recovery_text: &str,
    keys: &impl Keys,
) -> Result<()> {
    let key = recovery::decode(recovery_text)?;
    drop(footnote_search::store::open_encrypted_existing(
        backup, false, &key,
    )?);
    footnote_core::no_links(dir)?;
    fs::create_dir(dir).context("create new personal store directory")?;
    footnote_search::store::copy_encrypted(backup, &dir.join(DB), &key)?;
    let id = keys.insert(&key)?;
    write_new(&dir.join(KEY_ID), &id)?;
    write_new(&dir.join(MARKER), READY)?;
    Ok(())
}

fn open_existing_with(dir: &Path, write: bool, keys: &impl Keys) -> Result<Connection> {
    let key = unlock(dir, keys)?;
    footnote_search::store::open_encrypted_existing(&dir.join(DB), write, &key)
}

/// Reveal the recovery secret only after verifying that it opens the store's database.
pub fn recovery_text(dir: &Path) -> Result<Zeroizing<String>> {
    recovery_text_with(dir, &WindowsKeys)
}

fn recovery_text_with(dir: &Path, keys: &impl Keys) -> Result<Zeroizing<String>> {
    let key = unlock(dir, keys)?;
    drop(footnote_search::store::open_encrypted_existing(
        &dir.join(DB),
        false,
        &key,
    )?);
    Ok(recovery::encode(&key))
}

fn unlock(dir: &Path, keys: &impl Keys) -> Result<Zeroizing<[u8; 32]>> {
    footnote_core::no_links(dir)?;
    if read_exact_file::<{ READY.len() }>(&dir.join(MARKER))? != READY {
        bail!("personal store is incomplete or has an unsupported version");
    }
    let id = read_exact_file::<16>(&dir.join(KEY_ID))?;
    keys.load(&id)?
        .context("personal store key is unavailable; restore it from recovery")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};

    #[derive(Default)]
    struct FixtureKeys(RefCell<HashMap<[u8; 16], [u8; 32]>>);
    impl Keys for FixtureKeys {
        fn insert(&self, key: &[u8; 32]) -> Result<[u8; 16]> {
            let id = [0x31; 16];
            self.0.borrow_mut().insert(id, *key);
            Ok(id)
        }
        fn load(&self, id: &[u8; 16]) -> Result<Option<Zeroizing<[u8; 32]>>> {
            Ok(self.0.borrow().get(id).copied().map(Zeroizing::new))
        }
    }

    #[test]
    fn initializes_reopens_and_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("personal");
        let keys = FixtureKeys::default();
        let recovery = create_new_with(&dir, &keys).unwrap();
        assert_eq!(&*recovery, &*recovery_text_with(&dir, &keys).unwrap());
        let key = recovery::decode(&recovery).unwrap();
        let c = open_existing_with(&dir, true, &keys).unwrap();
        c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:fixture','hash',1,'indexed','memory')", []).unwrap();
        drop(c);
        assert!(create_new_with(&dir, &keys).is_err());
        let c =
            footnote_search::store::open_encrypted_existing(&dir.join(DB), false, &key).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(!dir.join("memory.db.tantivy").exists());
    }

    #[test]
    fn incomplete_or_missing_components_fail_closed() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("personal");
        let keys = FixtureKeys::default();
        fs::create_dir(&dir).unwrap();
        assert!(open_existing_with(&dir, true, &keys).is_err());
        assert!(!dir.join(DB).exists());
        write_new(&dir.join(KEY_ID), &[0x31; 16]).unwrap();
        write_new(&dir.join(MARKER), READY).unwrap();
        fs::write(dir.join(KEY_ID), [0x31; 15]).unwrap();
        assert!(open_existing_with(&dir, true, &keys).is_err());
        assert!(!dir.join(DB).exists());
        fs::write(dir.join(KEY_ID), [0x31; 16]).unwrap();
        assert!(open_existing_with(&dir, true, &keys).is_err());
        assert!(!dir.join(DB).exists());
        keys.0.borrow_mut().insert([0x31; 16], [0x42; 32]);
        assert!(open_existing_with(&dir, true, &keys).is_err());
        assert!(recovery_text_with(&dir, &keys).is_err());
        assert!(!dir.join(DB).exists());
        fs::write(dir.join(DB), b"").unwrap();
        assert!(open_existing_with(&dir, true, &keys).is_err());
        assert_eq!(fs::metadata(dir.join(DB)).unwrap().len(), 0);
        fs::write(dir.join(DB), b"not a database").unwrap();
        assert!(open_existing_with(&dir, true, &keys).is_err());
        assert_eq!(fs::read(dir.join(DB)).unwrap(), b"not a database");
    }

    #[test]
    fn unavailable_key_store_leaves_no_usable_or_plaintext_database() {
        struct Unavailable;
        impl Keys for Unavailable {
            fn insert(&self, _: &[u8; 32]) -> Result<[u8; 16]> {
                bail!("fixture store unavailable")
            }
            fn load(&self, _: &[u8; 16]) -> Result<Option<Zeroizing<[u8; 32]>>> {
                bail!("fixture store unavailable")
            }
        }
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("personal");
        assert!(create_new_with(&dir, &Unavailable).is_err());
        assert!(!dir.join(DB).exists());
        assert!(!dir.join(MARKER).exists());
        assert!(open_existing_with(&dir, true, &Unavailable).is_err());
    }

    #[test]
    fn real_windows_store_initializes_and_reopens_fixture() {
        struct Cleanup(windows_keystore::KeyId);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                windows_keystore::delete_fixture(&self.0);
            }
        }
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("personal");
        let recovery = create_new(&dir).unwrap();
        let id =
            windows_keystore::KeyId::from_bytes(read_exact_file::<16>(&dir.join(KEY_ID)).unwrap());
        let cleanup = Cleanup(id);
        let key = recovery::decode(&recovery).unwrap();
        let c = open_existing(&dir, true).unwrap();
        c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:fixture','hash',1,'indexed','memory')", []).unwrap();
        drop(c);
        let c =
            footnote_search::store::open_encrypted_existing(&dir.join(DB), false, &key).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(c);
        drop(cleanup);
        assert!(windows_keystore::load(&id).unwrap().is_none());
        assert!(open_existing(&dir, true).is_err());
    }

    #[test]
    fn restore_rejects_wrong_keys_and_existing_destinations() {
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("original");
        let target = tmp.path().join("restored");
        let backup = tmp.path().join("backup.db");
        let keys = FixtureKeys::default();
        let token = create_new_with(&original, &keys).unwrap();
        let key = recovery::decode(&token).unwrap();
        let c = open_existing_with(&original, true, &keys).unwrap();
        c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:fixture','hash',1,'indexed','memory')", []).unwrap();
        drop(c);
        footnote_search::store::copy_encrypted(&original.join(DB), &backup, &key).unwrap();
        let wrong = recovery::encode(&[0x88; 32]);
        assert!(restore_new_with(&target, &backup, &wrong, &keys).is_err());
        assert!(!target.exists());
        restore_new_with(&target, &backup, &token, &keys).unwrap();
        assert!(restore_new_with(&target, &backup, &token, &keys).is_err());
        let c = open_existing_with(&target, false, &keys).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
