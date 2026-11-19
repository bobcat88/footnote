//! Experimental per-user Windows Credential Manager storage for personal database keys.
//! The random identifier is not the key; callers must persist it separately with database metadata.
use anyhow::{Context, Result, bail};
use std::{fmt::Write, ptr};
use windows_sys::Win32::{
    Foundation::ERROR_NOT_FOUND,
    Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredFree, CredReadW, CredWriteW,
    },
};
use zeroize::{Zeroize, Zeroizing};

/// A randomly generated identifier, safe to store separately from the secret key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KeyId([u8; 16]);

impl KeyId {
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    fn target(&self) -> Vec<u16> {
        let mut name = String::from("Footnote/PersonalMemory/v1/");
        for byte in self.0 {
            write!(&mut name, "{byte:02x}").expect("writing to a String cannot fail");
        }
        name.encode_utf16().chain(Some(0)).collect()
    }
}

struct Credential(*mut CREDENTIALW);

impl Drop for Credential {
    fn drop(&mut self) {
        // CredReadW returns one allocation; wipe its blob before releasing it through CredFree.
        unsafe {
            let credential = &mut *self.0;
            if !credential.CredentialBlob.is_null() && credential.CredentialBlobSize > 0 {
                std::slice::from_raw_parts_mut(
                    credential.CredentialBlob,
                    credential.CredentialBlobSize as usize,
                )
                .zeroize();
            }
            CredFree(self.0.cast());
        }
    }
}

/// Read one key. Missing entries are distinct from an inaccessible or malformed key store.
pub fn load(id: &KeyId) -> Result<Option<Zeroizing<[u8; 32]>>> {
    let target = id.target();
    let mut raw = ptr::null_mut();
    // The target is NUL terminated, and CredFree owns the allocation returned on success.
    let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) };
    if ok == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
            return Ok(None);
        }
        return Err(error).context("cannot read the Windows credential store");
    }
    if raw.is_null() {
        bail!("Windows credential store returned an invalid credential");
    }
    let credential = Credential(raw);
    let value = unsafe { &*credential.0 };
    if value.CredentialBlobSize != 32 || value.CredentialBlob.is_null() {
        bail!("stored personal memory key has an invalid length");
    }
    let mut key = Zeroizing::new([0_u8; 32]);
    // CredReadW guarantees that the blob points inside its returned allocation.
    unsafe { key.copy_from_slice(std::slice::from_raw_parts(value.CredentialBlob, 32)) };
    Ok(Some(key))
}

/// Store a key under a fresh random identifier. No existing identifier is intentionally replaced.
pub fn insert(key: &[u8; 32]) -> Result<KeyId> {
    let mut id = [0_u8; 16];
    getrandom::fill(&mut id).map_err(|e| anyhow::anyhow!("OS entropy unavailable: {e}"))?;
    let id = KeyId(id);
    if load(&id)?.is_some() {
        bail!("random key identifier already exists; no key was replaced");
    }
    let mut target = id.target();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: target.as_mut_ptr(),
        CredentialBlobSize: 32,
        CredentialBlob: key.as_ptr().cast_mut(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        ..Default::default()
    };
    // CredWriteW reads the supplied blob synchronously; the key stays owned by the caller.
    if unsafe { CredWriteW(&credential, 0) } == 0 {
        return Err(std::io::Error::last_os_error())
            .context("cannot write the Windows credential store");
    }
    Ok(id)
}

/// Remove a key from the credential store; a key already gone is not an error. Used to undo a store creation that failed
/// after its key was written, so no orphaned credential is left behind (#131).
pub(crate) fn delete(id: &KeyId) -> Result<()> {
    let target = id.target();
    // SAFETY: `target` is a NUL-terminated wide string that outlives the call.
    if unsafe {
        windows_sys::Win32::Security::Credentials::CredDeleteW(
            target.as_ptr(),
            CRED_TYPE_GENERIC,
            0,
        )
    } == 0
    {
        let e = std::io::Error::last_os_error();
        // ERROR_NOT_FOUND: nothing to delete.
        if e.raw_os_error() != Some(1168) {
            return Err(e).context("cannot delete the Windows credential");
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn delete_fixture(id: &KeyId) {
    let _ = delete(id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Security::Credentials::CredDeleteW;

    struct Cleanup(KeyId);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let target = self.0.target();
            unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        }
    }

    #[test]
    fn deleting_a_key_removes_it_and_a_second_delete_is_not_an_error() {
        let id = insert(&Zeroizing::new([0x2a; 32])).unwrap();
        let _cleanup = Cleanup(id);
        delete(&id).unwrap();
        assert!(load(&id).unwrap().is_none());
        delete(&id).unwrap();
    }

    #[test]
    fn windows_store_round_trips_a_temporary_key() {
        let original = Zeroizing::new([0x63; 32]);
        let id = insert(&original).unwrap();
        let cleanup = Cleanup(id);
        let loaded = load(&id).unwrap().unwrap();
        assert_eq!(&*original, &*loaded);
        let copied_id = KeyId::from_bytes(*id.as_bytes());
        assert_eq!(&*load(&copied_id).unwrap().unwrap(), &*original);
        drop(cleanup);
        assert!(load(&id).unwrap().is_none());
    }

    #[test]
    fn malformed_credential_is_rejected() {
        let id = insert(&[0x74; 32]).unwrap();
        let cleanup = Cleanup(id);
        let mut target = id.target();
        let mut invalid = [0_u8; 1];
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: 1,
            CredentialBlob: invalid.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        assert_ne!(unsafe { CredWriteW(&credential, 0) }, 0);
        assert!(load(&id).is_err());
        drop(cleanup);
        assert!(load(&id).unwrap().is_none());
    }

    #[test]
    fn stored_key_and_recovery_text_restore_a_fixture() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.db");
        let backup = dir.path().join("backup.db");
        let restored = dir.path().join("restored.db");
        let key = crate::recovery::generate().unwrap();
        let id = insert(&key).unwrap();
        let cleanup = Cleanup(id);
        let local = load(&id).unwrap().unwrap();
        let c = footnote_search::store::open_encrypted(&source, true, &local).unwrap();
        c.execute("INSERT INTO sources (root,path,sha256,revision,status,extractor) VALUES ('','memory:fixture','hash',1,'indexed','memory')", []).unwrap();
        drop(c);
        footnote_search::store::copy_encrypted(&source, &backup, &local).unwrap();
        let recovered = crate::recovery::decode(&crate::recovery::encode(&key)).unwrap();
        footnote_search::store::copy_encrypted(&backup, &restored, &recovered).unwrap();
        let restored_id = insert(&recovered).unwrap();
        let restored_cleanup = Cleanup(restored_id);
        let restored_key = load(&restored_id).unwrap().unwrap();
        let c = footnote_search::store::open_encrypted(&restored, false, &restored_key).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM sources", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(restored_cleanup);
        drop(cleanup);
        assert!(load(&restored_id).unwrap().is_none());
        assert!(load(&id).unwrap().is_none());
    }

    #[test]
    fn child_reads_fixture_credential() {
        let Some(encoded) = std::env::var_os("FOOTNOTE_TEST_CREDENTIAL_ID") else {
            return;
        };
        let encoded = encoded.to_str().unwrap();
        assert_eq!(encoded.len(), 32);
        let mut bytes = [0_u8; 16];
        for (i, value) in bytes.iter_mut().enumerate() {
            *value = u8::from_str_radix(&encoded[i * 2..i * 2 + 2], 16).unwrap();
        }
        assert_eq!(
            &*load(&KeyId::from_bytes(bytes)).unwrap().unwrap(),
            &[0x35; 32]
        );
    }

    #[test]
    fn credential_survives_a_process_restart() {
        let id = insert(&[0x35; 32]).unwrap();
        let cleanup = Cleanup(id);
        let mut encoded = String::new();
        for b in id.as_bytes() {
            write!(&mut encoded, "{b:02x}").unwrap();
        }
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "windows_keystore::tests::child_reads_fixture_credential",
            ])
            .env("FOOTNOTE_TEST_CREDENTIAL_ID", encoded)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "fixture child failed: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
        drop(cleanup);
        assert!(load(&id).unwrap().is_none());
    }
}
