//! `footnote docs bundle|install|keygen|licence`: prebuilt documentation indexes (#309).
//!
//! A bundle is one file: `FNBUNDLE1\n`, a u32 big-endian manifest length, the manifest JSON, a 64-byte ed25519 signature over
//! exactly those manifest bytes, then the SQLite database. The manifest holds the database's sha256, so the signature covers
//! the data too. There are no archive entries, so there is no entry name that could climb out of the cache; the only names
//! are the manifest's library and version, which must already be in their plain cache form. A bundle is installed only when
//! the signature verifies against a pinned key (or one the user adds with `--trust-key`) and the sha256 matches.
//! Signing: the private key is never in the repository; `docs keygen <file>` makes one and `docs bundle --key <file>` signs.
use crate::{docs, find};
use anyhow::{Context, Result, ensure};
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8] = b"FNBUNDLE1\n";
const MAX_MANIFEST: usize = 64 * 1024;
/// Largest bundle a URL install will download (shortcut: one fixed cap; raise it when a real bundle needs it).
const MAX_DOWNLOAD: u64 = 1 << 30;
/// Footnote's release verification keys (hex, ed25519), current first; add the next key here before it signs anything.
/// The matching private keys are kept outside the repository.
const PINNED_KEYS: &[&str] = &["faae7862a8bf81d3b21a739e26039e00335399abc6ad3794473ee4b110ec17fd"];
/// Key ids (see `key_id`) whose bundles are refused even when the key is pinned or passed with `--trust-key`.
const REVOKED_KEY_IDS: &[&str] = &[];

#[derive(Serialize, Deserialize)]
struct Manifest {
    /// Id of the signing key (`key_id`), inside the signed bytes, so install knows which pinned key to check.
    key_id: String,
    library: String,
    site_url: String,
    version: String,
    indexed_unix: u64,
    pages: u64,
    licence: String,
    redistributable: bool,
    db_sha256: String,
    /// Identity of the embedding model(s) the vectors were made with; empty for a keyword-only index.
    embedding_model: String,
}

/// The source licence recorded beside a cache entry (`<version>.licence.json`).
#[derive(Serialize, Deserialize)]
struct Licence {
    name: String,
    redistributable: bool,
    url: String,
}

fn sidecar(db: &Path) -> PathBuf {
    db.with_extension("licence.json")
}

fn unhex(s: &str) -> Result<Vec<u8>> {
    let s = s.trim();
    ensure!(
        s.len().is_multiple_of(2) && s.is_ascii(),
        "not a hex string"
    );
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(Into::into))
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A key's id: the first 8 bytes of the sha256 of its public key, in hex.
fn key_id(public: &[u8]) -> String {
    hex(&Sha256::digest(public)[..8])
}

/// Check the manifest signature against the pinned and user-trusted keys: the manifest's `key_id` picks the key, a revoked id is refused.
fn verify(
    mj: &[u8],
    sig: &[u8],
    pinned: &[&str],
    revoked: &[&str],
    trust: &[String],
) -> Result<()> {
    let id = serde_json::from_slice::<Value>(mj)
        .ok()
        .and_then(|v| v["key_id"].as_str().map(String::from))
        .unwrap_or_default();
    ensure!(
        !revoked.contains(&id.as_str()),
        "signing key {id} is revoked: bundle rejected"
    );
    let key = pinned
        .iter()
        .copied()
        .chain(trust.iter().map(String::as_str))
        .filter_map(|k| unhex(k).ok())
        .find(|k| key_id(k) == id)
        .with_context(|| {
            format!(
                "signature key id {id:?} is unknown (no pinned or trusted key): bundle rejected"
            )
        })?;
    UnparsedPublicKey::new(&ED25519, key)
        .verify(mj, sig)
        .map_err(|_| {
            anyhow::anyhow!("signature does not verify against a trusted key: bundle rejected")
        })
}

/// True when the vectors in a bundle were made with a model this machine has not selected (manifest ids are comma-joined).
fn models_differ(manifest: &str, machine: &[String]) -> bool {
    let mut a: Vec<&str> = manifest.split(',').filter(|x| !x.is_empty()).collect();
    let mut b: Vec<&str> = machine.iter().map(String::as_str).collect();
    a.sort_unstable();
    b.sort_unstable();
    !a.is_empty() && a != b
}

/// A new signing key written to `file` (hex PKCS#8, never overwrites); returns the public half to pin or `--trust-key`.
pub fn keygen(file: &Path) -> Result<Value> {
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
        .map_err(|_| anyhow::anyhow!("no randomness"))?;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)
        .with_context(|| format!("create {}", file.display()))?;
    // Restricted before the key is written, so it is never readable by others.
    footnote_core::state::make_file_private(file)?;
    f.write_all(hex(pkcs8.as_ref()).as_bytes())?;
    let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(
        json!({"private_key_file": file, "public_key": hex(pair.public_key().as_ref()), "key_id": key_id(pair.public_key().as_ref()),
        "note": "keep the private key file out of any repository"}),
    )
}

/// Record a library version's source licence beside its cache entry; `bundle` refuses without it.
pub fn licence(
    state: &Path,
    library: &str,
    version: Option<&str>,
    name: &str,
    redistributable: bool,
    url: &str,
) -> Result<Value> {
    let (_, _, db) = docs::find_entry(state, library, version)?;
    ensure!(!name.trim().is_empty(), "the licence name is empty");
    let rec = Licence {
        name: name.trim().into(),
        redistributable,
        url: url.trim().into(),
    };
    std::fs::write(sidecar(&db), serde_json::to_vec_pretty(&rec)?)?;
    Ok(json!({"library": library, "licence": rec.name, "redistributable": redistributable}))
}

/// Build a signed bundle of an installed cache entry.
pub fn bundle(
    state: &Path,
    library: &str,
    version: Option<&str>,
    key: &Path,
    out: Option<&Path>,
) -> Result<Value> {
    let (site, version, db) = docs::find_entry(state, library, version)?;
    let lic: Licence = std::fs::read(sidecar(&db))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .filter(|l: &Licence| !l.name.trim().is_empty())
        .with_context(|| {
            format!("no licence recorded for {site} {version}; run `footnote docs licence` first")
        })?;
    ensure!(
        lic.redistributable,
        "{site} {version} is under {:?}, which does not allow redistribution: no bundle",
        lic.name
    );
    let pair =
        Ed25519KeyPair::from_pkcs8(&unhex(&std::fs::read_to_string(key)?)?).map_err(|_| {
            anyhow::anyhow!("{} is not a signing key from `docs keygen`", key.display())
        })?;
    let out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(format!("{site}-{version}.fnbundle")));
    // A consistent copy (no WAL), so the hash is of one file.
    let snap = out.with_extension("db.part");
    let _ = std::fs::remove_file(&snap);
    let c = crate::app::open(&db, false)?;
    c.execute("VACUUM INTO ?1", [snap.to_string_lossy().as_ref()])?;
    let pages: i64 = c.query_row(
        "SELECT count(*) FROM sources WHERE status='indexed'",
        [],
        |r| r.get(0),
    )?;
    let model: String = c
        .query_row(
            "SELECT group_concat(identity) FROM embedding_models",
            [],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or_default();
    drop(c);
    let indexed = std::fs::metadata(&db)?
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let m = Manifest {
        key_id: key_id(pair.public_key().as_ref()),
        library: site.clone(),
        site_url: lic.url,
        version: version.clone(),
        indexed_unix: indexed,
        pages: pages as u64,
        licence: lic.name,
        redistributable: true,
        db_sha256: sha256_file(&snap)?,
        embedding_model: model,
    };
    let mj = serde_json::to_vec_pretty(&m)?;
    let write = || -> Result<()> {
        let mut f = std::fs::File::create(&out)?;
        f.write_all(MAGIC)?;
        f.write_all(&(mj.len() as u32).to_be_bytes())?;
        f.write_all(&mj)?;
        f.write_all(pair.sign(&mj).as_ref())?;
        std::io::copy(&mut std::fs::File::open(&snap)?, &mut f)?;
        Ok(())
    };
    let r = write();
    let _ = std::fs::remove_file(&snap);
    r?;
    Ok(
        json!({"bundle": out, "library": site, "version": version, "pages": pages, "sha256": m.db_sha256}),
    )
}

fn sha256_file(p: &Path) -> Result<String> {
    let mut h = Sha256::new();
    std::io::copy(&mut std::fs::File::open(p)?, &mut h)?;
    Ok(hex(&h.finalize()))
}

/// Install a bundle from a file or an https URL. Nothing reaches the cache until every check has passed.
pub fn install(state: &Path, source: &str, trust: &[String]) -> Result<Value> {
    if !source.starts_with("https://") {
        ensure!(!source.contains("://"), "only https URLs or local files");
        return install_file(state, Path::new(source), trust);
    }
    crate::state::ensure_private_dir(state)?;
    let root = state.join("docs");
    std::fs::create_dir_all(&root)?;
    let tmp = root.join(format!(".bundle-{}.part", std::process::id()));
    let r = fetch(source, &tmp).and_then(|_| install_file(state, &tmp, trust));
    let _ = std::fs::remove_file(&tmp);
    r
}

/// The existing HTTP client settings (system roots, as in models.rs), with a size cap on the body.
fn fetch(url: &str, to: &Path) -> Result<()> {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let mut body = agent
        .get(url)
        .call()
        .with_context(|| format!("download {url}"))?
        .into_body();
    let mut f = std::fs::File::create(to)?;
    // One byte past the cap tells "exactly at the cap" from "too big".
    let n = std::io::copy(&mut body.as_reader().take(MAX_DOWNLOAD + 1), &mut f)?;
    ensure!(
        n <= MAX_DOWNLOAD,
        "bundle is larger than {MAX_DOWNLOAD} bytes: refused"
    );
    Ok(())
}

fn install_file(state: &Path, file: &Path, trust: &[String]) -> Result<Value> {
    let mut f = std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?;
    let mut magic = [0u8; MAGIC.len()];
    let _ = f.read_exact(&mut magic);
    ensure!(magic == MAGIC, "not a Footnote docs bundle");
    let mut len = [0u8; 4];
    f.read_exact(&mut len).context("bundle truncated")?;
    let len = u32::from_be_bytes(len) as usize;
    ensure!(len <= MAX_MANIFEST, "manifest too large");
    let mut mj = vec![0u8; len];
    f.read_exact(&mut mj).context("bundle truncated")?;
    let mut sig = [0u8; 64];
    f.read_exact(&mut sig).context("bundle truncated")?;
    verify(&mj, &sig, PINNED_KEYS, REVOKED_KEY_IDS, trust)?;
    let m: Manifest = serde_json::from_slice(&mj).context("manifest")?;
    ensure!(m.redistributable, "bundle is not marked redistributable");
    ensure!(!m.licence.trim().is_empty(), "bundle has no licence");
    for (what, v) in [("library", &m.library), ("version", &m.version)] {
        ensure!(
            !v.is_empty()
                && !v.starts_with('.')
                && v.chars().all(|c| c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || matches!(c, '.' | '_' | '-')),
            "manifest {what} {v:?} is not a plain cache name"
        );
    }
    let dir = state.join("docs").join(&m.library);
    let db = dir.join(format!("{}.db", m.version));
    ensure!(
        !db.exists(),
        "{} {} is already installed",
        m.library,
        m.version
    );
    std::fs::create_dir_all(&dir)?;
    let part = db.with_extension("db.part");
    let copied = (|| {
        let mut h = Sha256::new();
        let mut out = std::fs::File::create(&part)?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            out.write_all(&buf[..n])?;
        }
        drop(out);
        ensure!(
            hex(&h.finalize()) == m.db_sha256,
            "database hash does not match the manifest: bundle rejected"
        );
        let pages: i64 = crate::app::open(&part, false)?.query_row(
            "SELECT count(*) FROM sources WHERE status='indexed'",
            [],
            |r| r.get(0),
        )?;
        ensure!(
            pages as u64 == m.pages,
            "page count does not match the manifest"
        );
        Ok(())
    })();
    if let Err(e) = copied {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    std::fs::rename(&part, &db)?;
    std::fs::write(
        sidecar(&db),
        serde_json::to_vec_pretty(&Licence {
            name: m.licence.clone(),
            redistributable: true,
            url: m.site_url.clone(),
        })?,
    )?;
    // Never mix vectors from another model into this machine's search: keep the keyword index only.
    let machine: Vec<String> = crate::models::in_use(state)?
        .iter()
        .filter_map(|p| footnote_search::semantic::identity(Path::new(p)).ok())
        .map(|(id, _)| id)
        .collect();
    let stripped = models_differ(&m.embedding_model, &machine);
    if stripped {
        let c = crate::app::open(&db, true)?;
        c.execute_batch("DELETE FROM embeddings; DELETE FROM embedding_models;")?;
    }
    find::register(state, &db)?;
    let mut r = json!({"installed": m.library, "version": m.version, "pages": m.pages, "licence": m.licence, "db": db});
    if stripped {
        r["embeddings"] = json!(
            "not installed: the bundle's vectors come from a different model than the ones selected here; keyword search works now, run `footnote docs embed` to rebuild them"
        );
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> (Ed25519KeyPair, String) {
        let p = Ed25519KeyPair::from_pkcs8(
            Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
                .unwrap()
                .as_ref(),
        )
        .unwrap();
        let h = hex(p.public_key().as_ref());
        (p, h)
    }
    fn signed(p: &Ed25519KeyPair, id: &str) -> (Vec<u8>, Vec<u8>) {
        let m = serde_json::to_vec(&json!({"key_id": id})).unwrap();
        let s = p.sign(&m).as_ref().to_vec();
        (m, s)
    }

    #[test]
    fn second_pinned_key_accepted_revoked_and_unknown_rejected() {
        let (a, ah) = pair();
        let (b, bh) = pair();
        let (ida, idb) = (
            key_id(a.public_key().as_ref()),
            key_id(b.public_key().as_ref()),
        );
        let pinned = [ah.as_str(), bh.as_str()];
        let (m, s) = signed(&b, &idb);
        verify(&m, &s, &pinned, &[], &[]).unwrap();
        let e = verify(&m, &s, &pinned, &[idb.as_str()], &[])
            .unwrap_err()
            .to_string();
        assert!(e.contains("revoked"), "{e}");
        // Revocation also beats --trust-key.
        assert!(verify(&m, &s, &[], &[idb.as_str()], std::slice::from_ref(&bh)).is_err());
        let e = verify(&m, &s, &[ah.as_str()], &[], &[])
            .unwrap_err()
            .to_string();
        assert!(e.contains("unknown"), "{e}");
        // A manifest naming key a but signed by b does not verify.
        let (m, s) = signed(&b, &ida);
        let e = verify(&m, &s, &pinned, &[], &[]).unwrap_err().to_string();
        assert!(e.contains("does not verify"), "{e}");
    }

    #[test]
    fn model_mismatch_means_keyword_only() {
        let m = |x: &str| x.to_string();
        assert!(!models_differ("", &[m("a")]));
        assert!(!models_differ("a,b", &[m("b"), m("a")]));
        assert!(models_differ("a,b", &[m("a")]));
        assert!(models_differ("a", &[]));
    }
}
