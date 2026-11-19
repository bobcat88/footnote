//! Update and rollback of the installed binary (#126). Data lives in the state folder, apart from the binary, so neither
//! touches it. The binary being replaced is kept beside the new one as `<name>.replaced-<time>`; only the newest such copy is
//! kept (one a running daemon still uses cannot be removed and is left for later). Windows lets a running binary be renamed,
//! which is what makes updating in place possible.
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn stamp() -> String {
    let t = time::OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        t.year(),
        t.month() as u8,
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

/// The replaced copies beside `exe`, newest first (the time in their name sorts them).
fn replaced(exe: &Path) -> Result<Vec<PathBuf>> {
    let name = exe
        .file_name()
        .context("binary name")?
        .to_string_lossy()
        .into_owned();
    let prefix = format!("{name}.replaced-");
    let mut found: Vec<PathBuf> = std::fs::read_dir(exe.parent().context("binary folder")?)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&prefix))
        })
        .collect();
    found.sort();
    found.reverse();
    Ok(found)
}

/// Replace the running binary with `from`, after checking that `from` is a Footnote binary that runs.
pub fn update(from: &Path) -> Result<Value> {
    let exe = std::env::current_exe()?;
    let out = std::process::Command::new(from)
        .arg("--version")
        .output()
        .with_context(|| format!("cannot run {}", from.display()))?;
    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || !version.starts_with("footnote ") {
        bail!("{} is not a Footnote binary that runs", from.display());
    }
    let keep = exe.with_file_name(format!(
        "{}.replaced-{}",
        exe.file_name().context("binary name")?.to_string_lossy(),
        stamp()
    ));
    std::fs::rename(&exe, &keep).context("set the current binary aside")?;
    if let Err(e) = std::fs::copy(from, &exe) {
        let _ = std::fs::rename(&keep, &exe); // put the old one back
        return Err(e).context("copy the new binary into place");
    }
    // Keep only the newest replaced copy; one still in use stays until a later update.
    for old in replaced(&exe)?.into_iter().skip(1) {
        let _ = std::fs::remove_file(old);
    }
    Ok(
        json!({"updated": exe, "version": version, "previous": keep, "note": "`footnote rollback` puts the previous binary back; data in the state folder is not touched"}),
    )
}

/// Put the newest replaced binary back in place; the current one is set aside as a replaced copy too.
pub fn rollback() -> Result<Value> {
    let exe = std::env::current_exe()?;
    let previous = replaced(&exe)?
        .into_iter()
        .next()
        .context("no previous binary to roll back to (none was kept by `footnote update`)")?;
    let current = exe.with_file_name(format!(
        "{}.rolled-back-{}",
        exe.file_name().context("binary name")?.to_string_lossy(),
        stamp()
    ));
    std::fs::rename(&exe, &current).context("set the current binary aside")?;
    if let Err(e) = std::fs::rename(&previous, &exe) {
        let _ = std::fs::rename(&current, &exe);
        return Err(e).context("move the previous binary into place");
    }
    Ok(json!({"restored": exe, "from": previous, "set_aside": current}))
}
