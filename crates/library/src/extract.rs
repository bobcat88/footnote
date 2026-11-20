use anyhow::{Result, bail};
use serde::Deserialize;
use std::{
    fs,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

#[derive(Deserialize)]
pub struct Page {
    pub page: i64,
    pub text: String,
}
#[derive(Deserialize)]
pub struct Extraction {
    pub status: String,
    pub extractor: String,
    pub pages: Vec<Page>,
}

pub fn extract(bytes: &[u8], ext: &str) -> Result<Extraction> {
    let scratch = footnote_core::scratch_dir()?;
    let dir = tempfile::tempdir_in(scratch)?;
    let source = dir.path().join(format!("snapshot.{ext}"));
    fs::write(&source, bytes)?;
    let output = dir.path().join("output.json");
    let helper = crate::repo().join("tools/extract.py");
    let python = std::env::var_os("FOOTNOTE_PYTHON")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            crate::repo().join(if cfg!(windows) {
                ".venv/Scripts/python.exe"
            } else {
                ".venv/bin/python"
            })
        });
    let mut cmd = Command::new(python);
    cmd.arg("-I")
        .arg(&helper)
        .arg(source)
        .stdout(fs::File::create(&output)?)
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    // Untrusted documents are parsed here: never hand the agent's credentials on.
    footnote_core::helper_env(&mut cmd);
    let mut child = cmd.spawn()?;
    let Some(status) = child.wait_timeout(Duration::from_secs(120))? else {
        child.kill()?;
        child.wait()?;
        bail!("extractor timed out");
    };
    if !status.success() || fs::metadata(&output)?.len() > 8_000_000 {
        bail!("extractor failed or exceeded output limit");
    }
    let mut result: Extraction = serde_json::from_slice(&fs::read(output)?)?;
    if result.pages.len() > 100
        || result.pages.iter().map(|p| p.text.len()).sum::<usize>() > 4_000_000
    {
        bail!("extractor response exceeded limits");
    }
    result.extractor.push_str(&format!(
        ";helper-sha256={}",
        footnote_core::hash(&fs::read(helper)?)
    ));
    result.extractor.push_str(";chunker-v2");
    Ok(result)
}
