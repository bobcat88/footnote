//! Footnote's shared foundations: state folder, keys, audit log, metrics, safe file access (ADR 0001).
use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub mod audit;
pub mod config;
pub mod jobs;
pub mod metrics;
pub mod state;

pub const MAX_FILE: u64 = 16 * 1024 * 1024;
#[must_use]
pub fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
/// Build a child process with a scrubbed environment.
///
/// The MCP host spawns this binary inside the agent's own environment, which
/// typically carries provider credentials (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
/// GitHub tokens, ...). Helper subprocesses parse untrusted documents, so they
/// must never inherit those secrets. Only the variables a helper genuinely needs
/// are passed through; everything else is dropped.
pub fn helper_env(cmd: &mut std::process::Command) {
    cmd.env_clear();
    for key in [
        "PATH",
        "SYSTEMROOT",
        "WINDIR",
        "TMPDIR",
        "TMP",
        "TEMP",
        "HOME",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
    ] {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
}
#[must_use]
pub fn is_link(p: &Path) -> bool {
    let Ok(m) = fs::symlink_metadata(p) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
/// Private scratch directory for helper input/output: `TMPDIR`, else a per-user
/// folder (`%LOCALAPPDATA%` on Windows, `~/.cache` elsewhere). Created on demand,
/// never behind a link.
pub fn scratch_dir() -> Result<std::path::PathBuf> {
    let scratch = std::env::var_os("TMPDIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(|p| std::path::PathBuf::from(p).join("Footnote/scratch"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|p| std::path::PathBuf::from(p).join(".cache/Footnote/scratch"))
        })
        .ok_or_else(|| anyhow::anyhow!("configure TMPDIR for helper scratch space"))?;
    no_links(&scratch)?;
    std::fs::create_dir_all(&scratch)?;
    Ok(scratch)
}

pub fn no_links(p: &Path) -> Result<()> {
    let abs = std::path::absolute(p)?;
    if abs.ancestors().any(is_link) {
        bail!("symlinks/reparse points are not allowed");
    }
    Ok(())
}
pub fn bounded_read(p: &Path) -> Result<Vec<u8>> {
    no_links(p)?;
    if fs::metadata(p)?.len() > MAX_FILE {
        bail!("source exceeds 16 MiB");
    }
    if !fs::metadata(p)?.is_file() {
        bail!("source must be a regular file");
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(p)?
        .take(MAX_FILE + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE {
        bail!("source grew beyond bound");
    }
    Ok(bytes)
}

#[cfg(test)]
mod helper_env_tests {
    /// The scrubbed helper environment must drop everything the agent host
    /// injected (credentials, Cargo/CI variables) while keeping the handful of
    /// system variables a Python/ONNX helper needs to start.
    #[test]
    fn helper_env_clears_inherited_secrets_and_keeps_system_variables() {
        let mut cmd = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(["/c", "set"]);
            c
        } else {
            std::process::Command::new("env")
        };
        crate::helper_env(&mut cmd);
        let out = cmd.output().expect("probe command must run");
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        // Set in this test process by Cargo, deliberately NOT in the allowlist.
        assert!(
            !text.contains("cargo_manifest_dir"),
            "helper inherited the agent/cargo environment"
        );
        assert!(
            !text.contains("cargo_pkg_"),
            "helper inherited the agent/cargo environment"
        );
        assert!(text.contains("path"), "helper lost PATH");
    }
}
