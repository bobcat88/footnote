//! Zero-configuration security state.
//!
//! The audit log, its MAC key and the daemon token live in one per-user
//! directory and are created on first use. Nobody has to generate a secret, pass
//! a key file or remember a flag: security is the default and the plumbing is
//! automatic. `FOOTNOTE_STATE_DIR` overrides the location (used by tests).
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub fn state_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("FOOTNOTE_STATE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            })
    };
    let base =
        base.context("cannot determine a per-user state directory; set FOOTNOTE_STATE_DIR")?;
    Ok(base.join("footnote"))
}

/// Append one line to a log shared by several processes (metrics, activity, hook errors). The whole line goes out in one write
/// to a file opened for appending, so lines from processes writing at the same moment never interleave; formatting straight
/// into the file wrote each line in many small pieces, and lines from the launcher's parallel reads came out shuffled.
pub fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut buf = String::with_capacity(line.len() + 1);
    buf.push_str(line.trim_end_matches('\n'));
    buf.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(buf.as_bytes())
}

pub fn audit_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("audit.jsonl"))
}

pub fn key_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("audit.key"))
}

pub fn token_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("daemon.token"))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        ensure_private_dir(parent)?;
    }
    Ok(())
}

/// Create the state directory and, when this call creates it, make it private to
/// the current user. On Windows the profile's inherited ACL is not enough: groups
/// such as the Codex sandbox users can read `%LOCALAPPDATA%`, which would expose
/// the token and the MAC key to sandboxed agents. Inheritance is removed and only
/// the user and SYSTEM keep access; files created inside inherit that.
pub fn ensure_private_dir(dir: &Path) -> Result<()> {
    let existed = dir.exists();
    std::fs::create_dir_all(dir)
        .with_context(|| format!("create state directory {}", dir.display()))?;
    #[cfg(windows)]
    if !existed {
        restrict_to_current_user(dir, "(OI)(CI)F")?;
    }
    #[cfg(not(windows))]
    if !existed {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Make a file readable and writable by the current user only (0600; on Windows an access list with no inherited entries),
/// for secrets written outside the private state folder, such as a `docs keygen` signing key.
pub fn make_file_private(path: &Path) -> Result<()> {
    #[cfg(windows)]
    restrict_to_current_user(path, "F")?;
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// An existing state folder may predate the private rule or have been loosened since (#209): make it private again.
/// Called at session and daemon start, not on every write. Windows: a restricted folder has no inherited entries, so
/// any `(I)` in its access list means it was never restricted (`(I)` is not translated, unlike account names).
pub fn fix_private_dir(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let out = std::process::Command::new("icacls")
            .arg(dir)
            .output()
            .context("run icacls to read the state directory's access list")?;
        if String::from_utf8_lossy(&out.stdout).contains("(I)") {
            restrict_to_current_user(dir, "(OI)(CI)F")?;
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::metadata(dir)?.permissions().mode() & 0o077 != 0 {
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn restrict_to_current_user(dir: &Path, rights: &str) -> Result<()> {
    let user = std::env::var("USERNAME").context("USERNAME is not set")?;
    let who = match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{user}"),
        _ => user,
    };
    let out = std::process::Command::new("icacls")
        .arg(dir)
        .args(["/inheritance:r", "/grant:r"])
        .arg(format!("{who}:{rights}"))
        .args(["/grant:r"])
        .arg(format!("SYSTEM:{rights}"))
        .output()
        .context("run icacls to restrict the state directory")?;
    if !out.status.success() {
        bail!(
            "could not restrict {} to the current user: {}",
            dir.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Create `path` with `bytes` unless it already exists, and never expose a partial
/// file: the content is written to a private temp file, then hard-linked into
/// place (the link fails if the target exists, so two first runs cannot overwrite
/// each other and a reader never sees an empty file). 0600 on Unix; on Windows the
/// per-user state directory carries a user-only ACL. Returns true when this call
/// created it.
fn write_private(path: &Path, bytes: &[u8]) -> Result<bool> {
    ensure_parent(path)?;
    let temp = path.with_extension(format!("tmp{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    {
        use std::io::Write;
        let mut file = options
            .open(&temp)
            .with_context(|| format!("create {}", temp.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    let linked = std::fs::hard_link(&temp, path);
    let _ = std::fs::remove_file(&temp);
    match linked {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e).with_context(|| format!("create {}", path.display())),
    }
}

pub fn random_hex(bytes: usize) -> Result<String> {
    let mut buf = vec![0_u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| anyhow::anyhow!("OS entropy unavailable: {e}"))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

/// The audit MAC key, generated on first use.
pub fn load_or_create_key() -> Result<PathBuf> {
    let path = key_path()?;
    write_private(&path, random_hex(32)?.as_bytes())?;
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read MAC key {}", path.display()))?;
    if raw.trim().len() < 32 {
        // Never overwrite: replacing a key silently invalidates the whole audit chain.
        bail!(
            "MAC key {} is malformed; restore it from backup (or delete it together with the audit log to start over)",
            path.display()
        );
    }
    Ok(path)
}

/// The daemon token, generated on first use. Both the daemon and its local proxy
/// read this same file, so the handshake needs no user action.
pub fn load_or_create_token() -> Result<String> {
    let path = token_path()?;
    write_private(&path, random_hex(32)?.as_bytes())?;
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read daemon token {}", path.display()))?;
    let token = raw.trim().to_string();
    if token.len() < 32 {
        bail!(
            "daemon token {} is malformed; delete it and restart the daemon to issue a new one",
            path.display()
        );
    }
    Ok(token)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// The state directory and the secrets in it must be readable by the current
    /// user and SYSTEM only, whatever the parent directory's ACL grants.
    #[test]
    fn state_files_are_private_to_the_current_user() {
        let parent = std::env::temp_dir().join(format!("footnote-acl-{}", std::process::id()));
        let dir = parent.join("state");
        std::fs::create_dir_all(&parent).unwrap();
        ensure_private_dir(&dir).unwrap();
        let secret = dir.join("daemon.token");
        assert!(write_private(&secret, b"s".repeat(64).as_slice()).unwrap());
        let listing = std::process::Command::new("icacls")
            .arg(&secret)
            .output()
            .unwrap();
        let listing = String::from_utf8_lossy(&listing.stdout).to_string();
        let user = std::env::var("USERNAME").unwrap().to_lowercase();
        let listing = listing.to_lowercase();
        // Only the current user and SYSTEM may appear; no inherited group survives.
        assert!(
            listing.contains(&format!("\\{user}:")),
            "user missing: {listing}"
        );
        assert!(listing.contains("system:"), "SYSTEM missing: {listing}");
        // An elevated administrator (the CI runner) can take ownership of any file anyway, and Windows Server lists the
        // Administrators group on what it creates; for a normal user, administrators must not appear either.
        let elevated = std::process::Command::new("net")
            .arg("session")
            .output()
            .is_ok_and(|o| o.status.success());
        let mut broad = vec!["codexsandbox", "everyone", "users:", "authenticated"];
        if !elevated {
            broad.push("administrators");
        }
        for broad in broad {
            assert!(
                !listing.contains(broad),
                "{broad} can reach the secret: {listing}"
            );
        }
        std::fs::remove_dir_all(&parent).ok();
    }

    #[test]
    fn an_existing_folder_with_inherited_access_is_made_private() {
        let parent =
            std::env::temp_dir().join(format!("footnote-fix-parent-{}", std::process::id()));
        let dir = parent.join("child");
        std::fs::create_dir_all(&parent).unwrap();
        // Guarantee an inheritable ACE on the parent regardless of the runner's %TEMP% ACL.
        let grant = std::process::Command::new("icacls")
            .arg(&parent)
            .args(["/grant", "*S-1-1-0:(OI)(CI)R"])
            .output()
            .unwrap();
        assert!(grant.status.success(), "{grant:?}");
        std::fs::create_dir_all(&dir).unwrap(); // made outside Footnote: inherits the parent's access list
        let acl = |d: &Path| {
            let o = std::process::Command::new("icacls")
                .arg(d)
                .output()
                .unwrap();
            String::from_utf8_lossy(&o.stdout).to_string()
        };
        assert!(acl(&dir).contains("(I)"), "{}", acl(&dir));
        fix_private_dir(&dir).unwrap();
        assert!(!acl(&dir).contains("(I)"), "{}", acl(&dir));
        std::fs::remove_dir_all(&parent).ok();
    }

    #[test]
    fn creating_the_same_secret_twice_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("footnote-once-{}", std::process::id()));
        ensure_private_dir(&dir).unwrap();
        let path = dir.join("k");
        assert!(write_private(&path, b"first").unwrap());
        assert!(!write_private(&path, b"second").unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    #[test]
    fn an_existing_loose_folder_is_made_private() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        std::fs::set_permissions(t.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        super::fix_private_dir(t.path()).unwrap();
        let mode = std::fs::metadata(t.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }
}

#[cfg(test)]
mod append_tests {
    #[test]
    fn lines_from_writers_at_the_same_moment_never_interleave() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("log.jsonl");
        let workers: Vec<_> = (0..16)
            .map(|w| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for i in 0..200 {
                        let line = serde_json::json!({"worker": w, "i": i, "pad": "x".repeat(200)})
                            .to_string();
                        super::append_line(&path, &line).unwrap(); // a fresh handle each time, like separate processes
                    }
                })
            })
            .collect();
        for w in workers {
            w.join().unwrap();
        }
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 16 * 200);
        assert!(
            text.lines()
                .all(|l| serde_json::from_str::<serde_json::Value>(l).is_ok())
        );
    }
}
