//! Plug-and-play registration: point every detected agent at this corpus.
//!
//! Each agent's MCP entry runs `footnote --db <DB> mcp`, a thin stdio
//! front-end. The daemon, token and audit log are automatic, so the only thing
//! written here is "where is the corpus" — no secrets, no per-agent tuning.
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const SERVER: &str = "footnote";

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
    merge_json(path, dry_run, |root| {
        let servers = root
            .entry("mcpServers".to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .with_context(|| format!("{}: mcpServers is not an object", path.display()))?;
        if servers.get(SERVER).is_some() {
            return Ok(false);
        }
        servers.insert(SERVER.to_string(), entry.clone());
        Ok(true)
    })
}

/// Read a JSON object file (missing or empty: `{}`), let `change` edit it, and write it back only when it changed,
/// with a one-time backup and an atomic replace.
fn merge_json(
    path: &Path,
    dry_run: bool,
    change: impl FnOnce(&mut serde_json::Map<String, Value>) -> Result<bool>,
) -> Result<&'static str> {
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
    if !change(object)? {
        return Ok("already registered");
    }
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
        "{}.footnote-tmp",
        path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ));
    std::fs::write(&temp, text).with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, path).with_context(|| format!("replace {}", path.display()))
}

fn write_codex_toml(path: &Path, db: &Path, dry_run: bool) -> Result<&'static str> {
    codex_server(path, SERVER, db, dry_run)
}

fn codex_server(path: &Path, name: &str, db: &Path, dry_run: bool) -> Result<&'static str> {
    let exe = std::env::current_exe().context("locate this executable")?;
    let header = format!("[mcp_servers.{name}]");
    let block = format!(
        "\n{header}\ncommand = \"{}\"\nargs = [\"--db\", \"{}\", \"mcp\"]\n",
        portable(&exe),
        portable(db)
    );
    append_once(path, &header, &block, dry_run)
}

/// Append `block` to a text file unless `marker` is already in it (creating the file and its folder if needed).
fn append_once(path: &Path, marker: &str, block: &str, dry_run: bool) -> Result<&'static str> {
    let existing = if path.exists() {
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?
    } else {
        String::new()
    };
    if existing.contains(marker) {
        return Ok("already registered");
    }
    if !dry_run {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        backup(path)?;
        let mut text = existing.clone();
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(block);
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
    let task = format!("footnote-{}", endpoint.trim_start_matches("footnote-"));
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
        let _ = dry_run; // nothing is registered here, so a dry run and a real run answer the same
        json!({
            "task": task,
            "status": "manual on this platform",
            "hint": "run `footnote --db <DB> serve --mcp --daemon` from your session manager",
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
    if !dry_run {
        crate::find::record_corpus(&db);
    }
    let entry = command(&db)?;
    let mut report: Vec<Value> = Vec::new();

    let claude = home.join(".claude.json");
    let codex = home.join(".codex/config.toml");
    let gemini = home.join(".gemini/settings.json");

    let mut targets: Vec<(String, PathBuf, bool)> = vec![
        (
            "claude-code".into(),
            claude.clone(),
            claude.exists() || home.join(".claude").exists(),
        ),
        (
            "codex".into(),
            codex.clone(),
            codex.parent().is_some_and(Path::exists),
        ),
        (
            "gemini-cli".into(),
            gemini.clone(),
            gemini.parent().is_some_and(Path::exists),
        ),
        // Grok lists only its own config.toml servers (Codex keys).
        (
            "grok".into(),
            home.join(".grok/config.toml"),
            home.join(".grok").exists(),
        ),
    ];
    let agy_dirs = antigravity_config_dirs(&home);
    if agy_dirs.is_empty() {
        targets.push((
            "antigravity".into(),
            home.join(".gemini/config/mcp_config.json"),
            false,
        ));
    } else {
        for (agent, cfg) in agy_dirs {
            targets.push((agent, cfg.join("mcp_config.json"), true));
        }
    }
    for (agent, path, detected) in targets {
        if !detected {
            report.push(json!({"agent": agent, "status": "not detected", "path": portable(&path)}));
            continue;
        }
        let status = if agent == "codex" || agent == "grok" {
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
        "note": "each agent runs `footnote --db <DB> mcp`; the daemon, token and audit log are automatic",
    }))
}

fn has_antigravity(home: &Path) -> bool {
    let g = home.join(".gemini");
    g.join("config").exists()
        || g.join("antigravity").exists()
        || g.join("antigravity-cli").exists()
}

/// All detected Antigravity config directories (`<cfg>` holding `mcp_config.json`, `hooks.json`, `skills/`):
/// 1. Standard `~/.gemini/config` when Antigravity is present.
/// 2. T3 Code isolated provider dirs `~/.t3/userdata/providers/antigravity/<hash>/config`.
/// 3. Active `$GEMINI_HOME/config` when `GEMINI_HOME` points inside `home` (keeps isolated test homes clean).
fn antigravity_config_dirs(home: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut push_unique = |label: &str, cfg: PathBuf| {
        let key = portable(&cfg).to_ascii_lowercase();
        if !out
            .iter()
            .any(|(_, p)| portable(p).to_ascii_lowercase() == key)
        {
            out.push((label.to_string(), cfg));
        }
    };
    if has_antigravity(home) {
        push_unique("antigravity", home.join(".gemini/config"));
    }
    let t3_root = home.join(".t3/userdata/providers/antigravity");
    if let Ok(entries) = std::fs::read_dir(&t3_root) {
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            let cfg = dir.join("config");
            if cfg.is_dir() || dir.join("antigravity-acp").is_dir() {
                push_unique("antigravity (t3-code)", cfg);
            }
        }
    }
    if let Some(gh) = std::env::var_os("GEMINI_HOME").filter(|v| !v.is_empty()) {
        let gh = PathBuf::from(gh);
        if gh.starts_with(home) && gh.is_dir() {
            push_unique("antigravity (GEMINI_HOME)", gh.join("config"));
        }
    }
    out
}

#[cfg(feature = "memory")]
const MEMORY_SERVER: &str = "footnote-memory";
#[cfg(feature = "memory")]
fn is_memory_hook(command: &str) -> bool {
    if command.contains("memory_hook.py") {
        return true; // migrate the installed Python hook in place
    }
    command.trim().strip_suffix(" hook").is_some_and(|exe| {
        let exe = exe.trim_matches(['\'', '"']).replace('\\', "/");
        matches!(exe.rsplit('/').next(), Some("footnote" | "footnote.exe"))
    })
}

#[cfg(feature = "memory")]
/// Add an MCP server entry under `mcpServers`, leaving an existing entry of that name untouched.
fn add_server(path: &Path, name: &str, entry: &Value, dry_run: bool) -> Result<&'static str> {
    merge_json(path, dry_run, |root| {
        let servers = root
            .entry("mcpServers".to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .with_context(|| format!("{}: mcpServers is not an object", path.display()))?;
        if servers.contains_key(name) {
            return Ok(false);
        }
        servers.insert(name.to_string(), entry.clone());
        Ok(true)
    })
}

#[cfg(feature = "memory")]
/// Add the memory hook command to every listed event of a Claude-format `hooks` object, once.
fn add_hooks(
    path: &Path,
    events: &[(&str, u64)],
    command: &str,
    dry_run: bool,
) -> Result<&'static str> {
    merge_json(path, dry_run, |root| {
        let hooks = root
            .entry("hooks".to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .with_context(|| format!("{}: hooks is not an object", path.display()))?;
        let mut changed = false;
        for (event, timeout) in events {
            let list = hooks
                .entry(event.to_string())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .with_context(|| format!("{}: hooks.{event} is not an array", path.display()))?;
            let mut found = false;
            for group in list.iter_mut() {
                for hook in group["hooks"].as_array_mut().into_iter().flatten() {
                    if hook["command"].as_str().is_some_and(is_memory_hook) {
                        found = true;
                        if hook["command"] != command {
                            hook["command"] = json!(command);
                            for key in ["command_windows", "commandWindows"] {
                                if hook.get(key).is_some() {
                                    hook[key] = json!(if command.starts_with('"') {
                                        format!("& {command}")
                                    } else {
                                        command.to_string()
                                    });
                                }
                            }
                            changed = true;
                        }
                    }
                }
            }
            if !found {
                list.push(
                    json!({"hooks": [{"type": "command", "command": command, "timeout": timeout}]}),
                );
                changed = true;
            }
        }
        Ok(changed)
    })
}

#[cfg(feature = "memory")]
/// Add the memory hook command to every listed event of Gemini CLI's `~/.gemini/settings.json`
/// (`matcher: "*"` and millisecond timeouts), once.
fn add_gemini_memory_hooks(
    path: &Path,
    events: &[(&str, u64)],
    command: &str,
    dry_run: bool,
) -> Result<&'static str> {
    merge_json(path, dry_run, |root| {
        let hooks = root
            .entry("hooks".to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .with_context(|| format!("{}: hooks is not an object", path.display()))?;
        let mut changed = false;
        for (event, timeout) in events {
            let list = hooks
                .entry(event.to_string())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .with_context(|| format!("{}: hooks.{event} is not an array", path.display()))?;
            let mut found = false;
            for group in list.iter_mut() {
                for hook in group["hooks"].as_array_mut().into_iter().flatten() {
                    if hook["command"].as_str().is_some_and(is_memory_hook) {
                        found = true;
                        if hook["command"] != command {
                            hook["command"] = json!(command);
                            changed = true;
                        }
                    }
                }
            }
            if !found {
                list.push(json!({
                    "matcher": "*",
                    "hooks": [{"type": "command", "command": command, "timeout": timeout}]
                }));
                changed = true;
            }
        }
        Ok(changed)
    })
}

#[cfg(feature = "memory")]
/// Add or update the named `"footnote-memory"` hook entry in Antigravity's `~/.gemini/config/hooks.json`
/// (flat handler arrays for `SessionStart`, `PreInvocation` and `Stop`), leaving all other named hooks untouched.
fn add_agy_hooks(path: &Path, command: &str, dry_run: bool) -> Result<&'static str> {
    let desired = json!({
        "SessionStart": [{"type": "command", "command": command, "timeout": 15}],
        "PreInvocation": [{"type": "command", "command": command, "timeout": 10}],
        "Stop": [{"type": "command", "command": command, "timeout": 5}],
    });
    merge_json(path, dry_run, |root| {
        if root.get(MEMORY_SERVER) == Some(&desired) {
            return Ok(false);
        }
        root.insert(MEMORY_SERVER.to_string(), desired);
        Ok(true)
    })
}

/// Quote a path only when it must be: an unquoted command runs the same under bash (Claude Code), PowerShell (Grok) and cmd.
fn shell_word(path: &Path, warnings: &mut Vec<String>) -> String {
    let p = portable(path);
    if p.contains(' ') {
        warnings.push(format!(
            "{p} contains a space: the hook command is quoted for bash; Grok runs hooks under PowerShell, so set GROK_SHELL=bash for it"
        ));
        format!("\"{p}\"")
    } else {
        p
    }
}

const LEAN_SERVER: &str = "footnote-lean";

fn is_lean_hook(command: &str) -> bool {
    command
        .trim()
        .strip_suffix(" hook lean")
        .is_some_and(|exe| {
            let exe = exe.trim_matches(['\'', '"']).replace('\\', "/");
            matches!(exe.rsplit('/').next(), Some("footnote" | "footnote.exe"))
        })
}

fn is_rtk_hook(command: &str) -> bool {
    let trimmed = command.trim();
    let Some((first, rest)) = trimmed.split_once(char::is_whitespace) else {
        return false;
    };
    let bin = first.trim_matches(['\'', '"']).replace('\\', "/");
    let exe = bin.rsplit('/').next().unwrap_or("");
    matches!(exe.to_ascii_lowercase().as_str(), "rtk" | "rtk.exe")
        && rest.trim_start().starts_with("hook")
}

/// Add or update a Claude-format matcher hook entry (e.g. `PreToolUse` with `matcher: "Bash"`).
fn add_matcher_hook(
    path: &Path,
    event: &str,
    matcher: &str,
    timeout: u64,
    command: &str,
    is_ours: fn(&str) -> bool,
    dry_run: bool,
) -> Result<&'static str> {
    merge_json(path, dry_run, |root| {
        let hooks = root
            .entry("hooks".to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .with_context(|| format!("{}: hooks is not an object", path.display()))?;
        let list = hooks
            .entry(event.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .with_context(|| format!("{}: hooks.{event} is not an array", path.display()))?;
        let mut changed = false;
        let mut found = false;
        for group in list.iter_mut() {
            for hook in group["hooks"].as_array_mut().into_iter().flatten() {
                if hook["command"].as_str().is_some_and(is_ours) {
                    found = true;
                    if hook["command"] != command {
                        hook["command"] = json!(command);
                        changed = true;
                    }
                }
            }
        }
        if !found {
            list.push(json!({
                "matcher": matcher,
                "hooks": [{"type": "command", "command": command, "timeout": timeout}]
            }));
            changed = true;
        }
        Ok(changed)
    })
}

fn remove_matching_hooks(
    path: &Path,
    predicate: fn(&str) -> bool,
    dry_run: bool,
) -> Result<&'static str> {
    if !path.exists() {
        return Ok("not present");
    }
    let status = merge_json(path, dry_run, |root| {
        let mut changed = false;
        if let Some(events) = root.get_mut("hooks").and_then(Value::as_object_mut) {
            for list in events.values_mut().filter_map(Value::as_array_mut) {
                for group in list.iter_mut() {
                    if let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                        let before = hooks.len();
                        hooks.retain(|h| !h["command"].as_str().is_some_and(predicate));
                        changed |= hooks.len() != before;
                    }
                }
                list.retain(|g| g["hooks"].as_array().is_none_or(|h| !h.is_empty()));
            }
            events.retain(|_, list| list.as_array().is_none_or(|l| !l.is_empty()));
        }
        Ok(changed)
    })?;
    Ok(if status == "registered" {
        "removed"
    } else {
        "not present"
    })
}

fn remove_mcp_entry(path: &Path, server: &str, dry_run: bool) -> Result<&'static str> {
    if !path.exists() {
        return Ok("not present");
    }
    let status = merge_json(path, dry_run, |root| {
        Ok(root
            .get_mut("mcpServers")
            .and_then(Value::as_object_mut)
            .is_some_and(|s| s.remove(server).is_some()))
    })?;
    Ok(if status == "registered" {
        "removed"
    } else {
        "not present"
    })
}

fn remove_rtk_entries(home: &Path, dry_run: bool) -> Result<Vec<Value>> {
    let mut removed = Vec::new();
    let mut push =
        |agent: &str, what: &str, path: &Path, status: Result<&'static str>| -> Result<()> {
            removed.push(
                json!({"agent": agent, "what": what, "path": portable(path), "status": status?}),
            );
            Ok(())
        };
    let claude_settings = home.join(".claude/settings.json");
    push(
        "claude-code + grok",
        "rtk hook",
        &claude_settings,
        remove_matching_hooks(&claude_settings, is_rtk_hook, dry_run),
    )?;
    let claude_mcp = home.join(".claude.json");
    push(
        "claude-code + grok",
        "rtk mcp server",
        &claude_mcp,
        remove_mcp_entry(&claude_mcp, "rtk", dry_run),
    )?;
    let codex_hooks = home.join(".codex/hooks.json");
    push(
        "codex",
        "rtk hook",
        &codex_hooks,
        remove_matching_hooks(&codex_hooks, is_rtk_hook, dry_run),
    )?;
    let codex_toml = home.join(".codex/config.toml");
    push(
        "codex",
        "rtk mcp server",
        &codex_toml,
        edit_text(&codex_toml, dry_run, |t| without_toml_table(t, "rtk")),
    )?;
    let gemini_settings = home.join(".gemini/settings.json");
    push(
        "gemini-cli",
        "rtk hook",
        &gemini_settings,
        remove_matching_hooks(&gemini_settings, is_rtk_hook, dry_run),
    )?;
    push(
        "gemini-cli",
        "rtk mcp server",
        &gemini_settings,
        remove_mcp_entry(&gemini_settings, "rtk", dry_run),
    )?;
    let mut agy_dirs = antigravity_config_dirs(home);
    if agy_dirs.is_empty() {
        agy_dirs.push(("antigravity".into(), home.join(".gemini/config")));
    }
    for (agent, cfg) in agy_dirs {
        let agy_hooks = cfg.join("hooks.json");
        if agy_hooks.exists() {
            let st = merge_json(&agy_hooks, dry_run, |root| {
                let mut changed = root.remove("rtk").is_some();
                for val in root.values_mut() {
                    if let Some(obj) = val.as_object_mut() {
                        for list in obj.values_mut().filter_map(Value::as_array_mut) {
                            let before = list.len();
                            list.retain(|h| !h["command"].as_str().is_some_and(is_rtk_hook));
                            changed |= list.len() != before;
                        }
                    }
                }
                Ok(changed)
            })?;
            push(
                &agent,
                "rtk hook",
                &agy_hooks,
                Ok(if st == "registered" {
                    "removed"
                } else {
                    "not present"
                }),
            )?;
        }
        let agy_mcp = cfg.join("mcp_config.json");
        push(
            &agent,
            "rtk mcp server",
            &agy_mcp,
            remove_mcp_entry(&agy_mcp, "rtk", dry_run),
        )?;
    }
    Ok(removed)
}

/// `footnote install --lean [--remove-rtk]`: install Footnote Lean PreToolUse hooks across detected agents (#351).
pub fn lean(dry_run: bool, remove_rtk: bool) -> Result<Value> {
    let home = home()?;
    let exe = std::env::current_exe().context("locate this executable")?;
    let mut warnings = Vec::new();
    let command = format!("{} hook lean", shell_word(&exe, &mut warnings));
    let mut report = Vec::new();
    let mut add =
        |agent: &str, what: &str, path: &Path, status: Result<&'static str>| -> Result<()> {
            report.push(
                json!({"agent": agent, "what": what, "path": portable(path), "status": status?}),
            );
            Ok(())
        };

    let claude = home.join(".claude");
    if claude.exists() || home.join(".grok").exists() {
        let hooks = claude.join("settings.json");
        add(
            "claude-code + grok",
            "lean PreToolUse hook",
            &hooks,
            add_matcher_hook(
                &hooks,
                "PreToolUse",
                "Bash",
                5,
                &command,
                is_lean_hook,
                dry_run,
            ),
        )?;
    }
    let codex = home.join(".codex");
    if codex.exists() {
        let hooks = codex.join("hooks.json");
        add(
            "codex",
            "lean PreToolUse hook",
            &hooks,
            add_matcher_hook(
                &hooks,
                "PreToolUse",
                "Bash",
                5,
                &command,
                is_lean_hook,
                dry_run,
            ),
        )?;
    }
    let gemini = home.join(".gemini");
    if gemini.exists() {
        let hooks = gemini.join("settings.json");
        // Migrate away from earlier broken Claude-style PreToolUse/Bash entries in Gemini CLI settings.
        if hooks.exists() {
            let _ = merge_json(&hooks, dry_run, |root| {
                let mut changed = false;
                if let Some(events) = root.get_mut("hooks").and_then(Value::as_object_mut) {
                    if let Some(list) = events.get_mut("PreToolUse").and_then(Value::as_array_mut) {
                        for group in list.iter_mut() {
                            if let Some(hs) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                                let before = hs.len();
                                hs.retain(|h| !h["command"].as_str().is_some_and(is_lean_hook));
                                changed |= hs.len() != before;
                            }
                        }
                        list.retain(|g| g["hooks"].as_array().is_none_or(|h| !h.is_empty()));
                    }
                    events.retain(|_, list| list.as_array().is_none_or(|l| !l.is_empty()));
                }
                Ok(changed)
            });
        }
        add(
            "gemini-cli",
            "lean BeforeTool hook",
            &hooks,
            add_matcher_hook(
                &hooks,
                "BeforeTool",
                "run_shell_command",
                5000,
                &command,
                is_lean_hook,
                dry_run,
            ),
        )?;
    }
    for (agent, cfg) in antigravity_config_dirs(&home) {
        let hooks = cfg.join("hooks.json");
        // Antigravity tool hooks (`PreToolUse`, `PostToolUse`) require grouped `{matcher, hooks}` entries.
        let desired = json!({
            "PreToolUse": [{
                "matcher": "run_command",
                "hooks": [{"type": "command", "command": command, "timeout": 5}],
            }],
        });
        let st = merge_json(&hooks, dry_run, |root| {
            if root.get(LEAN_SERVER) == Some(&desired) {
                return Ok(false);
            }
            root.insert(LEAN_SERVER.to_string(), desired);
            Ok(true)
        });
        add(&agent, "lean PreToolUse hook", &hooks, st)?;
    }

    let rtk_removed = if remove_rtk {
        remove_rtk_entries(&home, dry_run)?
    } else {
        vec![]
    };

    if !crate::run::footnote_on_path() {
        warnings.push(
            "`footnote` is not on PATH yet (hooks fall back to the absolute binary path; run `footnote install --add-to-path` so bare `footnote` works in every shell)".into(),
        );
    }

    Ok(json!({
        "dry_run": dry_run,
        "hook": command,
        "targets": report,
        "rtk_removed": rtk_removed,
        "warnings": warnings,
        "note": "restart each agent to reload hooks; `footnote uninstall --lean` removes only Lean hooks without touching Memory",
    }))
}

/// `footnote install --add-to-path`: add the folder containing this binary to the user's `PATH`
/// (Windows User `PATH` registry entry, or POSIX `~/.profile` / `~/.bashrc` / `~/.zshrc`), idempotently.
pub fn add_to_path(dry_run: bool) -> Result<Value> {
    let exe = std::env::current_exe().context("locate this executable")?;
    let dir = exe
        .parent()
        .context("locate parent folder of this executable")?;

    #[cfg(windows)]
    {
        let dir_str = dir.to_string_lossy().to_string();
        let current = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[Environment]::GetEnvironmentVariable('Path', 'User')",
            ])
            .output()
            .context("read Windows User PATH")?;
        let cur_str = String::from_utf8_lossy(&current.stdout).trim().to_string();
        let already = cur_str
            .split(';')
            .any(|p| p.trim().eq_ignore_ascii_case(dir_str.trim()));
        if already {
            return Ok(json!({
                "dry_run": dry_run,
                "directory": portable(dir),
                "target": "Windows User PATH",
                "status": "already present",
            }));
        }
        if !dry_run {
            let next = if cur_str.is_empty() {
                dir_str.clone()
            } else {
                format!("{cur_str};{dir_str}")
            };
            let status = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "[Environment]::SetEnvironmentVariable('Path', $env:FOOTNOTE_NEXT_PATH, 'User')",
                ])
                .env("FOOTNOTE_NEXT_PATH", &next)
                .status()
                .context("update Windows User PATH")?;
            if !status.success() {
                bail!("failed to update Windows User PATH (exit {status})");
            }
        }
        Ok(json!({
            "dry_run": dry_run,
            "directory": portable(dir),
            "target": "Windows User PATH",
            "status": if dry_run { "would add" } else { "added" },
            "note": "open a new terminal or restart your agent so shells pick up the updated PATH",
        }))
    }

    #[cfg(not(windows))]
    {
        let home = home()?;
        let dir_posix = portable(dir);
        let export_line = format!("export PATH=\"{dir_posix}:$PATH\" # footnote\n");
        let mut candidates = vec![home.join(".profile")];
        for rc in [".bashrc", ".zshrc"] {
            let p = home.join(rc);
            if p.exists() {
                candidates.push(p);
            }
        }
        let mut targets = Vec::new();
        for rc in candidates {
            let st = append_once(&rc, &dir_posix, &export_line, dry_run)?;
            let label = match (dry_run, st) {
                (true, "registered") => "would add",
                (false, "registered") => "added",
                (_, "already registered") => "already present",
                (_, other) => other,
            };
            targets.push(json!({"path": portable(&rc), "status": label}));
        }
        Ok(json!({
            "dry_run": dry_run,
            "directory": dir_posix,
            "targets": targets,
            "note": "open a new shell or source your profile so PATH takes effect",
        }))
    }
}

/// `footnote install --remove-rtk`: disconnect RTK hooks and MCP entries across detected agents (#351).
pub fn remove_rtk(dry_run: bool) -> Result<Value> {
    let home = home()?;
    let removed = remove_rtk_entries(&home, dry_run)?;
    Ok(json!({
        "dry_run": dry_run,
        "rtk_removed": removed,
        "note": "restart each agent to unload removed hooks; backups were kept as *.footnote-backup",
    }))
}

/// `footnote uninstall --lean`: remove Footnote Lean PreToolUse hooks without touching Memory hooks or data (#351).
pub fn uninstall_lean(dry_run: bool) -> Result<Value> {
    let home = home()?;
    let mut report = Vec::new();
    let mut add =
        |agent: &str, what: &str, path: &Path, status: Result<&'static str>| -> Result<()> {
            report.push(
                json!({"agent": agent, "what": what, "path": portable(path), "status": status?}),
            );
            Ok(())
        };
    let claude = home.join(".claude/settings.json");
    add(
        "claude-code + grok",
        "lean PreToolUse hook",
        &claude,
        remove_matching_hooks(&claude, is_lean_hook, dry_run),
    )?;
    let codex = home.join(".codex/hooks.json");
    add(
        "codex",
        "lean PreToolUse hook",
        &codex,
        remove_matching_hooks(&codex, is_lean_hook, dry_run),
    )?;
    let gemini = home.join(".gemini/settings.json");
    add(
        "gemini-cli",
        "lean PreToolUse hook",
        &gemini,
        remove_matching_hooks(&gemini, is_lean_hook, dry_run),
    )?;
    let mut agy_dirs = antigravity_config_dirs(&home);
    if agy_dirs.is_empty() {
        agy_dirs.push(("antigravity".into(), home.join(".gemini/config")));
    }
    for (agent, cfg) in agy_dirs {
        let agy_hooks = cfg.join("hooks.json");
        let agy_st = if !agy_hooks.exists() {
            Ok("not present")
        } else {
            let s = merge_json(&agy_hooks, dry_run, |root| {
                Ok(root.remove(LEAN_SERVER).is_some())
            })?;
            Ok(if s == "registered" {
                "removed"
            } else {
                "not present"
            })
        };
        add(&agent, "lean PreToolUse hook", &agy_hooks, agy_st)?;
    }
    Ok(json!({
        "dry_run": dry_run,
        "targets": report,
        "kept": "Footnote Memory hooks, memories, and usage/lean.jsonl ledger",
    }))
}

/// `install --memory`: one agent memory database for every detected agent, its MCP server, the memory hook and a
/// /remember command. Claude Code's files also serve Grok, which reads ~/.claude.json and ~/.claude/settings.json.
#[cfg(feature = "memory")]
pub fn memory(dry_run: bool) -> Result<Value> {
    let home = home()?;
    let state = crate::state::state_dir()?;
    let db = state.join("memory.db");
    let exe = std::env::current_exe().context("locate this executable")?;
    let mut warnings = Vec::new();
    let command = format!("{} hook", shell_word(&exe, &mut warnings));
    let entry = json!({"command": portable(&exe), "args": ["--db", portable(&db), "mcp"]});
    // The same three events everywhere: pack at start, recall per prompt, and at Stop the request that makes the agent save
    // its memories (Codex and DeepSeek's bridge have no compaction or end-of-session event, and none is needed).
    let events = [("SessionStart", 15), ("UserPromptSubmit", 10), ("Stop", 5)];
    // /remember is the save request itself: the agent writes its memories with the footnote_remember tool.
    let remember = "Footnote memory: save what is worth remembering from this conversation with the footnote_remember tool (MCP server footnote-memory). Each memory is one self-contained sentence with absolute dates. kind: preference (how the user likes to work), fact (a decision, convention or date), fix (a problem, its cause and the fix) or session (what was achieved, what is left). scope: \"global\" for what applies everywhere, otherwise this project's repository name in lower case (its name on its git remote, else its folder name). observed_at: today's date. Skip what is already remembered or readable in the code. Then confirm in one short line.\n";
    let mut report: Vec<Value> = Vec::new();
    let mut add =
        |agent: &str, what: &str, path: &Path, status: Result<&'static str>| -> Result<()> {
            report.push(
                json!({"agent": agent, "what": what, "path": portable(path), "status": status?}),
            );
            Ok(())
        };

    if !dry_run {
        crate::state::ensure_private_dir(&state)?;
        if !db.exists() {
            drop(crate::app::open(&db, true)?); // empty memory database, so read tools work before the first memory
        }
    }
    let settings = state.join("memory.json");
    let defaults = json!({"footnote": portable(&exe), "db": portable(&db)});
    add(
        "all",
        "settings",
        &settings,
        merge_json(&settings, dry_run, |root| {
            let mut changed = false;
            for (k, v) in defaults.as_object().into_iter().flatten() {
                if !root.contains_key(k) {
                    root.insert(k.clone(), v.clone());
                    changed = true;
                }
            }
            Ok(changed)
        }),
    )?;

    let claude = home.join(".claude");
    let grok = home.join(".grok");
    if claude.exists() || grok.exists() {
        let agent = match (claude.exists(), grok.exists()) {
            (true, true) => "claude-code + grok",
            (true, false) => "claude-code",
            _ => "grok (through Claude Code files)",
        };
        let mcp = home.join(".claude.json");
        add(
            agent,
            "mcp server",
            &mcp,
            add_server(&mcp, MEMORY_SERVER, &entry, dry_run),
        )?;
        let hooks = claude.join("settings.json");
        add(
            agent,
            "hooks",
            &hooks,
            add_hooks(&hooks, &events, &command, dry_run),
        )?;
        let cmd = claude.join("commands").join("remember.md");
        add(
            agent,
            "/remember",
            &cmd,
            append_once(&cmd, "Footnote", remember, dry_run),
        )?;
    }
    if grok.exists() {
        // Grok lists only its own config.toml servers; same TOML keys as Codex. Its hooks come from ~/.claude/settings.json.
        let config = grok.join("config.toml");
        add(
            "grok",
            "mcp server",
            &config,
            codex_server(&config, MEMORY_SERVER, &db, dry_run),
        )?;
    }
    let codex = home.join(".codex");
    if codex.exists() {
        let config = codex.join("config.toml");
        add(
            "codex",
            "mcp server",
            &config,
            codex_server(&config, MEMORY_SERVER, &db, dry_run),
        )?;
        let hooks = codex.join("hooks.json");
        add(
            "codex",
            "hooks",
            &hooks,
            add_hooks(&hooks, &events, &command, dry_run),
        )?;
        let prompt = codex.join("prompts").join("remember.md");
        add(
            "codex",
            "/remember",
            &prompt,
            append_once(&prompt, "Footnote", remember, dry_run),
        )?;
    }
    let patch = home.join(".dsh/profiles/desktop/cordis.patch.yml");
    if patch.exists() {
        // DeepSeek Harness: its Claude Code hooks bridge runs a dedicated hooks file (not all of Claude's hooks),
        // and an mcp-client entry serves the memory database.
        // The bridge package itself (@deepseek-ai/dsh-hooks-claude-code, pinned to the app's version) is installed with
        // DeepSeek's plugin manager.
        let hooks = home.join(".dsh").join("footnote-memory-hooks.json");
        add(
            "deepseek-harness",
            "hooks",
            &hooks,
            add_hooks(&hooks, &events, &command, dry_run),
        )?;
        let block = format!(
            "# Footnote agent memory (footnote install --memory)\n- insert:\n    - id: {MEMORY_SERVER}-hooks\n      name: \"@deepseek-ai/dsh-hooks-claude-code\"\n      config:\n        configPath: \"{}\"\n    - id: {MEMORY_SERVER}-mcp\n      name: \"@deepseek-ai/dsh-mcp-client\"\n      config:\n        transport: stdio\n        serverName: {MEMORY_SERVER}\n        command: \"{}\"\n        args: [\"--db\", \"{}\", \"mcp\"]\n        env: {{}}\n        cwd: \"{}\"\n        toolCallTimeoutMs: 60000\n        closeTimeoutMs: 5000\n        expandTemplates: false\n        failOnStartupError: false\n",
            portable(&hooks),
            portable(&exe),
            portable(&db),
            portable(&home)
        );
        let marker = format!("id: {MEMORY_SERVER}-hooks");
        add(
            "deepseek-harness",
            "hooks bridge + mcp server",
            &patch,
            append_once(&patch, &marker, &block, dry_run),
        )?;
    }
    let gemini = home.join(".gemini/settings.json");
    if gemini.parent().is_some_and(Path::exists) {
        add(
            "gemini-cli",
            "mcp server",
            &gemini,
            add_server(&gemini, MEMORY_SERVER, &entry, dry_run),
        )?;
        // Gemini CLI uses BeforeAgent / AfterAgent / SessionStart with wildcard matcher and ms timeouts.
        let gemini_events = [
            ("SessionStart", 15_000),
            ("BeforeAgent", 10_000),
            ("AfterAgent", 5_000),
        ];
        add(
            "gemini-cli",
            "hooks",
            &gemini,
            add_gemini_memory_hooks(&gemini, &gemini_events, &command, dry_run),
        )?;
    }
    for (agent, cfg) in antigravity_config_dirs(&home) {
        let mcp = cfg.join("mcp_config.json");
        add(
            &agent,
            "mcp server",
            &mcp,
            add_server(&mcp, MEMORY_SERVER, &entry, dry_run),
        )?;
        let hooks = cfg.join("hooks.json");
        add(
            &agent,
            "hooks",
            &hooks,
            add_agy_hooks(&hooks, &command, dry_run),
        )?;
        let skill = cfg.join("skills/remember/SKILL.md");
        let agy_skill = format!(
            "---\nname: remember\ndescription: >-\n  Save important preferences, project decisions, fixes and session summaries\n  from this conversation into Footnote Memory using footnote_remember.\n---\n\n{remember}"
        );
        add(
            &agent,
            "/remember",
            &skill,
            append_once(&skill, "Footnote", &agy_skill, dry_run),
        )?;
    }
    Ok(json!({
        "dry_run": dry_run,
        "memory_db": portable(&db),
        "hook": command,
        "targets": report,
        "warnings": warnings,
        "note": "restart each agent to load the hooks; Grok asks once to trust a folder before it runs hooks. The agent writes memories through MCP; hooks and embeddings run in Rust without Python.",
    }))
}

/// Edit a text file through `change` (None: nothing of ours in it); backup, atomic replace, and an empty result deletes the file.
fn edit_text(
    path: &Path,
    dry_run: bool,
    change: impl FnOnce(&str) -> Option<String>,
) -> Result<&'static str> {
    if !path.exists() {
        return Ok("not present");
    }
    let stamp = file_stamp(path);
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let Some(new) = change(&text) else {
        return Ok("not present");
    };
    if !dry_run {
        backup(path)?;
        if new.trim().is_empty() {
            std::fs::remove_file(path)?;
        } else {
            replace_atomically(path, &stamp, new)?;
        }
    }
    Ok("removed")
}

/// Drop a TOML table and its sub-tables (`[mcp_servers.<name>]`, `[mcp_servers.<name>.env]`).
fn without_toml_table(text: &str, name: &str) -> Option<String> {
    let table = format!("[mcp_servers.{name}]");
    let sub = format!("[mcp_servers.{name}.");
    let mut out = String::new();
    let (mut skipping, mut found) = (false, false);
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with('[') {
            skipping = t == table || t.starts_with(&sub);
            found |= skipping;
        }
        if !skipping {
            out.push_str(line);
        }
    }
    found.then(|| out.trim_end().to_string() + "\n")
}

/// `footnote uninstall --memory`: remove what `install --memory` added to every agent. Memories and settings stay.
#[cfg(feature = "memory")]
pub fn uninstall_memory(dry_run: bool) -> Result<Value> {
    let home = home()?;
    let mut report: Vec<Value> = Vec::new();
    let mut add =
        |agent: &str, what: &str, path: &Path, status: Result<&'static str>| -> Result<()> {
            let status = match status? {
                "registered" => "removed",
                "already registered" => "not present",
                s => s,
            };
            report.push(
                json!({"agent": agent, "what": what, "path": portable(path), "status": status}),
            );
            Ok(())
        };
    let remove_server = |path: &Path| {
        if !path.exists() {
            return Ok("not present");
        }
        merge_json(path, dry_run, |root| {
            Ok(root
                .get_mut("mcpServers")
                .and_then(Value::as_object_mut)
                .is_some_and(|s| s.remove(MEMORY_SERVER).is_some()))
        })
    };
    let remove_hooks = |path: &Path| {
        if !path.exists() {
            return Ok("not present");
        }
        merge_json(path, dry_run, |root| {
            let mut changed = false;
            if let Some(events) = root.get_mut("hooks").and_then(Value::as_object_mut) {
                for list in events.values_mut().filter_map(Value::as_array_mut) {
                    for group in list.iter_mut() {
                        if let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                            let before = hooks.len();
                            hooks.retain(|h| !h["command"].as_str().is_some_and(is_memory_hook));
                            changed |= hooks.len() != before;
                        }
                    }
                    list.retain(|g| g["hooks"].as_array().is_none_or(|h| !h.is_empty()));
                }
                events.retain(|_, list| list.as_array().is_none_or(|l| !l.is_empty()));
            }
            Ok(changed)
        })
    };
    // The /remember command is one paragraph appended by install, starting with this sentence.
    let remove_remember = |path: &Path| {
        edit_text(path, dry_run, |text| {
            let kept: String = text
                .split_inclusive('\n')
                .filter(|l| !l.starts_with("Footnote memory: save what is worth remembering"))
                .collect();
            (kept.len() != text.len()).then_some(kept)
        })
    };

    let claude = home.join(".claude");
    let grok = home.join(".grok");
    let codex = home.join(".codex");
    if claude.exists() || grok.exists() {
        let mcp = home.join(".claude.json");
        add(
            "claude-code + grok",
            "mcp server",
            &mcp,
            remove_server(&mcp),
        )?;
        let hooks = claude.join("settings.json");
        add("claude-code + grok", "hooks", &hooks, remove_hooks(&hooks))?;
        let cmd = claude.join("commands").join("remember.md");
        add(
            "claude-code + grok",
            "/remember",
            &cmd,
            remove_remember(&cmd),
        )?;
    }
    for (agent, dir) in [("grok", &grok), ("codex", &codex)] {
        let config = dir.join("config.toml");
        let status = edit_text(&config, dry_run, |t| without_toml_table(t, MEMORY_SERVER));
        add(agent, "mcp server", &config, status)?;
    }
    let hooks = codex.join("hooks.json");
    add("codex", "hooks", &hooks, remove_hooks(&hooks))?;
    let prompt = codex.join("prompts").join("remember.md");
    add("codex", "/remember", &prompt, remove_remember(&prompt))?;
    // DeepSeek Harness: the hooks file is ours alone; the patch holds the one block install appended.
    let dsh_hooks = home.join(".dsh").join("footnote-memory-hooks.json");
    let status = edit_text(&dsh_hooks, dry_run, |_| Some(String::new()));
    add("deepseek-harness", "hooks", &dsh_hooks, status)?;
    let patch = home.join(".dsh/profiles/desktop/cordis.patch.yml");
    let status = edit_text(&patch, dry_run, |text| {
        let start = text.find("# Footnote agent memory (footnote install --memory)\n")?;
        let tail = "failOnStartupError: false\n";
        let end = start + text[start..].find(tail)? + tail.len();
        Some(format!("{}{}", &text[..start], &text[end..]))
    });
    add(
        "deepseek-harness",
        "hooks bridge + mcp server",
        &patch,
        status,
    )?;
    let gemini = home.join(".gemini/settings.json");
    add("gemini-cli", "mcp server", &gemini, remove_server(&gemini))?;
    add("gemini-cli", "hooks", &gemini, remove_hooks(&gemini))?;
    let mut agy_dirs = antigravity_config_dirs(&home);
    if agy_dirs.is_empty() {
        agy_dirs.push(("antigravity".into(), home.join(".gemini/config")));
    }
    for (agent, agy_cfg) in agy_dirs {
        let agy_mcp = agy_cfg.join("mcp_config.json");
        add(&agent, "mcp server", &agy_mcp, remove_server(&agy_mcp))?;
        let agy_hooks = agy_cfg.join("hooks.json");
        let agy_hooks_status = if !agy_hooks.exists() {
            Ok("not present")
        } else {
            merge_json(&agy_hooks, dry_run, |root| {
                Ok(root.remove(MEMORY_SERVER).is_some())
            })
        };
        add(&agent, "hooks", &agy_hooks, agy_hooks_status)?;
        let agy_skill = agy_cfg.join("skills/remember/SKILL.md");
        let agy_skill_status = edit_text(&agy_skill, dry_run, |text| {
            text.contains("Footnote memory: save what is worth remembering")
                .then(String::new)
        });
        add(&agent, "/remember", &agy_skill, agy_skill_status)?;
    }
    Ok(json!({
        "dry_run": dry_run,
        "targets": report,
        "kept": "your memories (memory.db) and settings (memory.json) in the Footnote state folder",
        "note": "restart each agent to unload the hooks; `footnote install --memory` puts everything back",
    }))
}

/// Where Footnote Memory is registered, read-only (`footnote status`).
#[cfg(feature = "memory")]
pub fn memory_registrations() -> Result<Value> {
    let home = home()?;
    let json_file = |p: &Path| -> Value {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null)
    };
    let has_server = |p: &Path| json_file(p)["mcpServers"].get(MEMORY_SERVER).is_some();
    let has_hooks = |p: &Path| {
        json_file(p)["hooks"].as_object().is_some_and(|events| {
            events
                .values()
                .filter_map(Value::as_array)
                .flatten()
                .any(|g| {
                    g["hooks"].as_array().is_some_and(|h| {
                        h.iter()
                            .any(|h| h["command"].as_str().is_some_and(is_memory_hook))
                    })
                })
        })
    };
    let has_table = |p: &Path| {
        std::fs::read_to_string(p)
            .is_ok_and(|t| t.contains(&format!("[mcp_servers.{MEMORY_SERVER}]")))
    };
    let mut out = Vec::new();
    if home.join(".claude").exists() {
        out.push(
            json!({"agent": "claude-code", "mcp_server": has_server(&home.join(".claude.json")),
            "hooks": has_hooks(&home.join(".claude/settings.json"))}),
        );
    }
    if home.join(".codex").exists() {
        out.push(
            json!({"agent": "codex", "mcp_server": has_table(&home.join(".codex/config.toml")),
            "hooks": has_hooks(&home.join(".codex/hooks.json"))}),
        );
    }
    if home.join(".grok").exists() {
        out.push(
            json!({"agent": "grok", "mcp_server": has_table(&home.join(".grok/config.toml")),
            "hooks": has_hooks(&home.join(".claude/settings.json"))}),
        );
    }
    if home.join(".dsh").exists() {
        out.push(json!({"agent": "deepseek-harness",
            "mcp_server": std::fs::read_to_string(home.join(".dsh/profiles/desktop/cordis.patch.yml"))
                .is_ok_and(|t| t.contains(&format!("id: {MEMORY_SERVER}-mcp"))),
            "hooks": has_hooks(&home.join(".dsh/footnote-memory-hooks.json"))}));
    }
    if home.join(".gemini").exists() {
        let g = home.join(".gemini/settings.json");
        out.push(
            json!({"agent": "gemini-cli", "mcp_server": has_server(&g), "hooks": has_hooks(&g)}),
        );
    }
    for (agent, cfg) in antigravity_config_dirs(&home) {
        let agy_hooks = json_file(&cfg.join("hooks.json"));
        let hooks = agy_hooks.get(MEMORY_SERVER).is_some_and(|spec| {
            spec.get("enabled").and_then(Value::as_bool) != Some(false)
                && ["SessionStart", "PreInvocation", "Stop"].iter().any(|ev| {
                    spec[ev].as_array().is_some_and(|list| {
                        list.iter()
                            .any(|h| h["command"].as_str().is_some_and(is_memory_hook))
                    })
                })
        });
        out.push(json!({
            "agent": agent,
            "mcp_server": has_server(&cfg.join("mcp_config.json")),
            "hooks": hooks,
        }));
    }
    Ok(json!(out))
}
