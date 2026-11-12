//! Plug-and-play registration: point every detected agent at this corpus.
//!
//! Each agent's MCP entry runs `borg-knowledge --db <DB> mcp`, a thin stdio
//! front-end. The daemon, token and audit log are automatic, so the only thing
//! written here is "where is the corpus" — no secrets, no per-agent tuning.
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const SERVER: &str = "borg";

fn home() -> Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .context("cannot determine the home directory")
}

/// Normalise for config files: forward slashes avoid TOML/JSON escaping traps.
fn portable(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn command(db: &Path) -> Result<Value> {
    let exe = std::env::current_exe().context("locate this executable")?;
    Ok(json!({
        "command": portable(&exe),
        "args": ["--db", portable(db), "mcp"],
    }))
}

fn backup(path: &Path) -> Result<()> {
    if path.exists() {
        let mut backup_path = path.to_path_buf();
        let name = format!(
            "{}.{SERVER}-backup",
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        );
        backup_path.set_file_name(name);
        if !backup_path.exists() {
            std::fs::copy(path, &backup_path)?;
        }
    }
    Ok(())
}

fn write_json(path: &Path, entry: &Value, dry_run: bool) -> Result<&'static str> {
    let stamp = file_stamp(path);
    let mut root: Value = if path.exists() {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        if raw.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&raw)
                .with_context(|| format!("{} is not valid JSON", path.display()))?
        }
    } else {
        json!({})
    };
    let object = root
        .as_object_mut()
        .with_context(|| format!("{} is not a JSON object", path.display()))?;
    let servers = object
        .entry("mcpServers".to_string())
        .or_insert_with(|| json!({}));
    let servers = servers
        .as_object_mut()
        .with_context(|| format!("{}: mcpServers is not an object", path.display()))?;
    if servers.get(SERVER).is_some() {
        return Ok("already registered");
    }
    servers.insert(SERVER.to_string(), entry.clone());
    if !dry_run {
        backup(path)?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        replace_atomically(
            path,
            &stamp,
            format!("{}\n", serde_json::to_string_pretty(&root)?),
        )?;
    }
    Ok("registered")
}

/// (modified time, length) of a file, to notice a concurrent writer.
fn file_stamp(path: &Path) -> Option<(std::time::SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Replace `path` through a temp file and a rename, so a crash never leaves a
/// half-written agent config. Agents such as Claude Code rewrite their own file
/// while running: if it changed since we read it, stop instead of overwriting.
fn replace_atomically(
    path: &Path,
    read_stamp: &Option<(std::time::SystemTime, u64)>,
    text: String,
) -> Result<()> {
    if &file_stamp(path) != read_stamp {
        bail!(
            "{} changed while it was being updated (the agent is probably running); close it and re-run install",
            path.display()
        );
    }
    let mut temp = path.to_path_buf();
    temp.set_file_name(format!(
        "{}.borg-tmp",
        path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ));
    std::fs::write(&temp, text).with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, path).with_context(|| format!("replace {}", path.display()))
}

fn write_codex_toml(path: &Path, db: &Path, dry_run: bool) -> Result<&'static str> {
    let exe = std::env::current_exe().context("locate this executable")?;
    let existing = if path.exists() {
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?
    } else {
        String::new()
    };
    if existing.contains("[mcp_servers.borg]") {
        return Ok("already registered");
    }
    let block = format!(
        "\n[mcp_servers.borg]\ncommand = \"{}\"\nargs = [\"--db\", \"{}\", \"mcp\"]\n",
        portable(&exe),
        portable(db)
    );
    if !dry_run {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        backup(path)?;
        let mut text = existing.clone();
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&block);
        std::fs::write(path, text)?;
    }
    Ok("registered")
}

/// Register a logon task that keeps one daemon per corpus running, then start it.
///
/// Task Scheduler owns the process, so the daemon inherits no agent handles —
/// the reason the proxy must never spawn it itself.
fn register_daemon(db: &Path, dry_run: bool) -> Value {
    let endpoint = crate::daemon::endpoint_name(db);
    let task = format!(
        "borg-knowledge-{}",
        endpoint.trim_start_matches("borg-knowledge-")
    );
    #[cfg(windows)]
    {
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => return json!({"task": task, "status": format!("error: {e}")}),
        };
        let command = format!(
            "\"{}\" --db \"{}\" serve --mcp --daemon",
            portable(&exe),
            portable(db)
        );
        if dry_run {
            return json!({"task": task, "status": "dry-run", "command": command});
        }
        let created = std::process::Command::new("schtasks")
            .args([
                "/create", "/f", "/tn", &task, "/sc", "onlogon", "/tr", &command,
            ])
            .output();
        match created {
            Ok(out) if out.status.success() => {
                let _ = std::process::Command::new("schtasks")
                    .args(["/run", "/tn", &task])
                    .output();
                json!({
                    "task": task,
                    "status": "registered at logon and started now",
                    "command": command,
                })
            }
            Ok(out) => json!({
                "task": task,
                "status": "not registered",
                "detail": String::from_utf8_lossy(&out.stderr).trim().to_string(),
                "command": command,
            }),
            Err(e) => json!({"task": task, "status": format!("error: {e}"), "command": command}),
        }
    }
    #[cfg(not(windows))]
    {
        json!({
            "task": task,
            "status": "manual on this platform",
            "hint": "run `borg-knowledge --db <DB> serve --mcp --daemon` from your session manager",
        })
    }
}

/// Register the corpus in Claude Code, Codex CLI and Gemini CLI when present.
pub fn run(db: &Option<PathBuf>, dry_run: bool) -> Result<Value> {
    let Some(db) = db.as_deref() else {
        bail!("--db is required: install registers a specific corpus");
    };
    let db = std::path::absolute(db).unwrap_or_else(|_| db.to_path_buf());
    if !db.exists() {
        bail!(
            "corpus database {} does not exist; index a root first",
            db.display()
        );
    }
    let home = home()?;
    let entry = command(&db)?;
    let mut report: Vec<Value> = Vec::new();

    let claude = home.join(".claude.json");
    let codex = home.join(".codex/config.toml");
    let gemini = home.join(".gemini/settings.json");

    let targets: Vec<(&str, PathBuf)> = vec![
        ("claude-code", claude),
        ("codex", codex),
        ("gemini-cli", gemini),
    ];
    for (agent, path) in targets {
        let detected = if agent == "claude-code" {
            path.exists() || home.join(".claude").exists()
        } else {
            path.parent().map(|p| p.exists()).unwrap_or(false)
        };
        if !detected {
            report.push(json!({"agent": agent, "status": "not detected", "path": portable(&path)}));
            continue;
        }
        let status = if agent == "codex" {
            write_codex_toml(&path, &db, dry_run)?
        } else {
            write_json(&path, &entry, dry_run)?
        };
        report.push(json!({"agent": agent, "status": status, "path": portable(&path)}));
    }

    Ok(json!({
        "dry_run": dry_run,
        "corpus": portable(&db),
        "entry": entry,
        "targets": report,
        "daemon": register_daemon(&db, dry_run),
        "note": "each agent runs `borg-knowledge --db <DB> mcp`; the daemon, token and audit log are automatic",
    }))
}
