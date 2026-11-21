//! `footnote audit-prefix` (#380): measure Turn-1 fixed prompt overhead across instruction files,
//! skills, and registered MCP servers. Read-only, local files only; never prints file contents,
//! environment variables, or credentials.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use walkdir::WalkDir;

const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PROBE_LINE: usize = 4 * 1024 * 1024;
const PROBE_INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"footnote-audit-prefix","version":"0.1"}}}"#;
const PROBE_NOTIFY: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const PROBE_LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;

#[derive(Debug, Clone)]
struct McpEntry {
    agent: &'static str,
    name: String,
    transport: &'static str,
    config_path: PathBuf,
    command: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .context("cannot determine home directory")
}

fn tokens_est(bytes: u64) -> u64 {
    bytes.div_ceil(4)
}

fn portable(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn command_basename(cmd: &str) -> String {
    Path::new(cmd)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cmd.to_string())
}

fn want_agent(filter: &str, agent: &str) -> bool {
    filter == "all" || filter == agent
}

/// Extract the byte length of YAML frontmatter (`--- ... ---`) at the top of a Markdown skill file.
fn frontmatter_bytes(path: &Path) -> u64 {
    let Ok(meta) = fs::metadata(path) else {
        return 0;
    };
    if meta.len() > MAX_READ_BYTES {
        return 0;
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let rest = if let Some(r) = raw.strip_prefix("---\r\n") {
        r
    } else if let Some(r) = raw.strip_prefix("---\n") {
        r
    } else {
        return 0;
    };
    let mut offset = raw.len() - rest.len();
    for line in rest.split_inclusive('\n') {
        offset += line.len();
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            return offset as u64;
        }
    }
    0
}

fn collect_md_files(dir: &Path, filename_filter: Option<&str>) -> Vec<PathBuf> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(dir)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir()
                && e.file_name()
                    .to_str()
                    .is_some_and(|n| n == ".trash" || n == ".git"))
        })
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if let Some(exact) = filename_filter {
            if p.file_name().and_then(|n| n.to_str()) == Some(exact) {
                out.push(p.to_path_buf());
            }
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        {
            out.push(p.to_path_buf());
        }
    }
    out.sort();
    out
}

fn parse_json_mcp_servers(path: &Path, agent: &'static str, out: &mut Vec<McpEntry>) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if !meta.is_file() || meta.len() > MAX_READ_BYTES {
        return;
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    let Ok(root) = serde_json::from_str::<Value>(&raw) else {
        return;
    };
    let Some(servers) = root.get("mcpServers").and_then(Value::as_object) else {
        return;
    };
    for (name, cfg) in servers {
        let Some(obj) = cfg.as_object() else {
            continue;
        };
        let command = obj
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_string);
        let url = obj
            .get("url")
            .or_else(|| obj.get("serverUrl"))
            .and_then(Value::as_str);
        let transport = if command.is_some() {
            "stdio"
        } else if url.is_some() {
            "http"
        } else {
            "unknown"
        };
        let args = obj
            .get("args")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let env = obj
            .get("env")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        out.push(McpEntry {
            agent,
            name: name.clone(),
            transport,
            config_path: path.to_path_buf(),
            command,
            args,
            env,
        });
    }
}

fn parse_toml_string(v: &str) -> Option<String> {
    if let Ok(Value::String(s)) = serde_json::from_str::<Value>(v) {
        return Some(s);
    }
    v.strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .map(str::to_string)
}

/// Minimal TOML reader for `[mcp_servers.<name>]` sections in `~/.codex/config.toml`.
fn parse_toml_mcp_servers(path: &Path, agent: &'static str, out: &mut Vec<McpEntry>) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if !meta.is_file() || meta.len() > MAX_READ_BYTES {
        return;
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    let mut current: Option<McpEntry> = None;
    let mut in_env = false;
    let flush = |cur: &mut Option<McpEntry>, out: &mut Vec<McpEntry>| {
        if let Some(mut e) = cur.take() {
            if e.command.is_some() {
                e.transport = "stdio";
            }
            out.push(e);
        }
    };
    for line in raw.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let section = inner.trim();
            if let Some(rest) = section.strip_prefix("mcp_servers.") {
                if let Some((name, sub)) = rest.split_once('.') {
                    if sub == "env"
                        && current
                            .as_ref()
                            .is_some_and(|e| e.name == name.trim_matches('"'))
                    {
                        in_env = true;
                        continue;
                    }
                    flush(&mut current, out);
                    in_env = false;
                } else {
                    flush(&mut current, out);
                    in_env = false;
                    current = Some(McpEntry {
                        agent,
                        name: rest.trim_matches('"').to_string(),
                        transport: "unknown",
                        config_path: path.to_path_buf(),
                        command: None,
                        args: Vec::new(),
                        env: Vec::new(),
                    });
                }
            } else {
                flush(&mut current, out);
                in_env = false;
            }
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if in_env {
            if let Some(s) = parse_toml_string(v) {
                entry.env.push((k.trim_matches('"').to_string(), s));
            }
            continue;
        }
        match k {
            "command" => {
                if let Some(s) = parse_toml_string(v) {
                    entry.command = Some(s);
                }
            }
            "url" => {
                entry.transport = "http";
            }
            "args" => {
                if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(v) {
                    entry.args = arr
                        .into_iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect();
                } else if let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    entry.args = inner
                        .split(',')
                        .filter_map(|part| parse_toml_string(part.trim()))
                        .collect();
                }
            }
            _ => {}
        }
    }
    flush(&mut current, out);
}

#[derive(Debug)]
struct ProbeOutcome {
    status: &'static str,
    tools_count: Option<usize>,
    schema_bytes: Option<u64>,
    note: Option<String>,
}

/// Perform a bounded read-only `initialize` + `tools/list` JSON-RPC handshake on one stdio MCP server.
fn probe_stdio_mcp(entry: &McpEntry, timeout: Duration) -> ProbeOutcome {
    let Some(cmd_str) = entry.command.as_deref() else {
        return ProbeOutcome {
            status: "skipped",
            tools_count: None,
            schema_bytes: None,
            note: Some("non-stdio transport".into()),
        };
    };
    let spawn_with = |program: &str| {
        let mut cmd = Command::new(program);
        cmd.args(&entry.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (k, v) in &entry.env {
            cmd.env(k, v);
        }
        cmd.spawn()
    };
    let mut child = match spawn_with(cmd_str) {
        Ok(c) => c,
        #[cfg(windows)]
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !cmd_str.ends_with(".cmd") => {
            match spawn_with(&format!("{cmd_str}.cmd")) {
                Ok(c) => c,
                Err(e2) => {
                    return ProbeOutcome {
                        status: "error",
                        tools_count: None,
                        schema_bytes: None,
                        note: Some(format!("spawn failed: {}", e2.kind())),
                    };
                }
            }
        }
        Err(e) => {
            return ProbeOutcome {
                status: "error",
                tools_count: None,
                schema_bytes: None,
                note: Some(format!("spawn failed: {}", e.kind())),
            };
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let payload = format!("{PROBE_INIT}\n{PROBE_NOTIFY}\n{PROBE_LIST}\n");
        let _ = stdin.write_all(payload.as_bytes());
        let _ = stdin.flush();
    }
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return ProbeOutcome {
            status: "error",
            tools_count: None,
            schema_bytes: None,
            note: Some("no stdout pipe".into()),
        };
    };

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buf = Vec::new();
        let mut outcome = ProbeOutcome {
            status: "error",
            tools_count: None,
            schema_bytes: None,
            note: Some("no tools/list response".into()),
        };
        for _ in 0..64 {
            buf.clear();
            let Ok(n) = (&mut reader)
                .take(MAX_PROBE_LINE as u64)
                .read_until(b'\n', &mut buf)
            else {
                break;
            };
            if n == 0 {
                break;
            }
            let Ok(line) = std::str::from_utf8(&buf) else {
                continue;
            };
            let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            if msg.get("id") == Some(&json!(2))
                && let Some(result) = msg.get("result")
            {
                let tools_count = result
                    .get("tools")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                let bytes = serde_json::to_vec(result).map_or(0, |b| b.len() as u64);
                outcome = ProbeOutcome {
                    status: "ok",
                    tools_count: Some(tools_count),
                    schema_bytes: Some(bytes),
                    note: None,
                };
                break;
            }
        }
        let _ = tx.send(outcome);
    });

    let outcome = match rx.recv_timeout(timeout) {
        Ok(o) => o,
        Err(_) => ProbeOutcome {
            status: "timeout",
            tools_count: None,
            schema_bytes: None,
            note: Some(format!("timed out after {}s", timeout.as_secs())),
        },
    };
    let _ = child.kill();
    let _ = child.wait();
    outcome
}

pub fn run(root: &Path, agent_filter: &str, probe_mcp: bool, timeout_secs: u64) -> Result<Value> {
    let home = home_dir()?;
    run_with_home(root, &home, agent_filter, probe_mcp, timeout_secs)
}

pub(crate) fn run_with_home(
    root: &Path,
    home: &Path,
    agent_filter: &str,
    probe_mcp: bool,
    timeout_secs: u64,
) -> Result<Value> {
    let timeout = Duration::from_secs(timeout_secs.clamp(1, 30));
    let mut seen_paths: BTreeSet<String> = BTreeSet::new();
    let mut instructions: Vec<Value> = Vec::new();
    let mut skills: Vec<Value> = Vec::new();
    let mut mcp_entries: Vec<McpEntry> = Vec::new();

    let mut add_instruction = |scope: &str, agent: &str, path: &Path| {
        if !path.is_file() {
            return;
        }
        let key = portable(path);
        if !seen_paths.insert(key.clone()) {
            return;
        }
        let bytes = fs::metadata(path).map_or(0, |m| m.len());
        instructions.push(json!({
            "kind": "instruction",
            "scope": scope,
            "agent": agent,
            "path": key,
            "bytes": bytes,
            "tokens_est": tokens_est(bytes),
        }));
    };

    // Project-level instruction files
    for (rel, agent) in [
        ("CLAUDE.md", "claude"),
        (".claude/CLAUDE.md", "claude"),
        ("AGENTS.md", "codex"),
        ("GEMINI.md", "gemini"),
        (".cursorrules", "all"),
        (".github/copilot-instructions.md", "all"),
    ] {
        if agent == "all" || want_agent(agent_filter, agent) {
            add_instruction("project", agent, &root.join(rel));
        }
    }
    if want_agent(agent_filter, "claude") {
        for p in collect_md_files(&root.join(".claude/rules"), None) {
            add_instruction("project", "claude", &p);
        }
    }

    // User-level instruction files
    if want_agent(agent_filter, "claude") {
        add_instruction("user", "claude", &home.join(".claude/CLAUDE.md"));
        for p in collect_md_files(&home.join(".claude/rules"), None) {
            add_instruction("user", "claude", &p);
        }
    }
    if want_agent(agent_filter, "codex") {
        add_instruction("user", "codex", &home.join(".codex/AGENTS.md"));
        add_instruction("user", "codex", &home.join(".codex/instructions.md"));
    }
    if want_agent(agent_filter, "gemini") {
        add_instruction("user", "gemini", &home.join(".gemini/GEMINI.md"));
    }
    if want_agent(agent_filter, "antigravity") {
        for p in collect_md_files(&home.join(".gemini/config/rules"), None) {
            add_instruction("user", "antigravity", &p);
        }
    }

    // Skills and slash commands
    let mut add_skill = |scope: &str, agent: &str, path: &Path| {
        if !path.is_file() {
            return;
        }
        let key = portable(path);
        if !seen_paths.insert(key.clone()) {
            return;
        }
        let bytes = fs::metadata(path).map_or(0, |m| m.len());
        let fm = frontmatter_bytes(path);
        let name = if path.file_name().and_then(|n| n.to_str()) == Some("SKILL.md") {
            path.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "skill".into())
        } else {
            path.file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "command".into())
        };
        skills.push(json!({
            "kind": "skill",
            "scope": scope,
            "agent": agent,
            "name": name,
            "path": key,
            "bytes": bytes,
            "tokens_est": tokens_est(bytes),
            "frontmatter_bytes": fm,
            "frontmatter_tokens_est": tokens_est(fm),
        }));
    };

    if want_agent(agent_filter, "claude") {
        for p in collect_md_files(&root.join(".claude/commands"), None) {
            add_skill("project", "claude", &p);
        }
        for p in collect_md_files(&root.join(".claude/skills"), Some("SKILL.md")) {
            add_skill("project", "claude", &p);
        }
        for p in collect_md_files(&home.join(".claude/commands"), None) {
            add_skill("user", "claude", &p);
        }
        for p in collect_md_files(&home.join(".claude/skills"), Some("SKILL.md")) {
            add_skill("user", "claude", &p);
        }
    }
    if want_agent(agent_filter, "codex") {
        for p in collect_md_files(&home.join(".codex/skills"), Some("SKILL.md")) {
            add_skill("user", "codex", &p);
        }
    }
    if want_agent(agent_filter, "gemini") {
        for p in collect_md_files(&home.join(".gemini/commands"), None) {
            add_skill("user", "gemini", &p);
        }
        for p in collect_md_files(&home.join(".gemini/skills"), Some("SKILL.md")) {
            add_skill("user", "gemini", &p);
        }
    }
    if want_agent(agent_filter, "antigravity") {
        for p in collect_md_files(&home.join(".gemini/config/skills"), Some("SKILL.md")) {
            add_skill("user", "antigravity", &p);
        }
        for p in collect_md_files(&home.join(".gemini/antigravity/skills"), Some("SKILL.md")) {
            add_skill("user", "antigravity", &p);
        }
    }

    // Registered MCP servers
    if want_agent(agent_filter, "claude") {
        parse_json_mcp_servers(&root.join(".mcp.json"), "claude", &mut mcp_entries);
        parse_json_mcp_servers(&home.join(".claude.json"), "claude", &mut mcp_entries);
        parse_json_mcp_servers(
            &home.join(".claude/settings.json"),
            "claude",
            &mut mcp_entries,
        );
    }
    if want_agent(agent_filter, "codex") {
        parse_toml_mcp_servers(&home.join(".codex/config.toml"), "codex", &mut mcp_entries);
    }
    if want_agent(agent_filter, "gemini") {
        parse_json_mcp_servers(
            &home.join(".gemini/settings.json"),
            "gemini",
            &mut mcp_entries,
        );
    }
    if want_agent(agent_filter, "antigravity") {
        parse_json_mcp_servers(
            &home.join(".gemini/config/mcp_config.json"),
            "antigravity",
            &mut mcp_entries,
        );
    }

    // Dedup MCP servers by (agent, name)
    let mut seen_mcp: BTreeSet<(&str, String)> = BTreeSet::new();
    mcp_entries.retain(|e| seen_mcp.insert((e.agent, e.name.clone())));

    let mut mcp_servers: Vec<Value> = Vec::with_capacity(mcp_entries.len());
    let mut mcp_schema_bytes: u64 = 0;
    let mut mcp_probed: usize = 0;
    for entry in &mcp_entries {
        let mut row = json!({
            "kind": "mcp_server",
            "agent": entry.agent,
            "name": entry.name,
            "transport": entry.transport,
            "command": entry.command.as_deref().map(command_basename),
            "config_path": portable(&entry.config_path),
            "probe_status": "not_probed",
        });
        if probe_mcp {
            let outcome = probe_stdio_mcp(entry, timeout);
            row["probe_status"] = json!(outcome.status);
            if let Some(n) = outcome.tools_count {
                row["tools_count"] = json!(n);
            }
            if let Some(b) = outcome.schema_bytes {
                row["schema_bytes"] = json!(b);
                row["tokens_est"] = json!(tokens_est(b));
                mcp_schema_bytes += b;
                mcp_probed += 1;
            }
            if let Some(note) = outcome.note {
                row["note"] = json!(note);
            }
        }
        mcp_servers.push(row);
    }

    // Build unified top-contributors ranking (descending by estimated Turn-1 tokens)
    let mut ranked: Vec<Value> = Vec::new();
    for item in &instructions {
        ranked.push(json!({
            "kind": "instruction",
            "agent": item["agent"],
            "name": item["path"],
            "bytes": item["bytes"],
            "tokens_est": item["tokens_est"],
        }));
    }
    for item in &skills {
        let fm = item["frontmatter_bytes"].as_u64().unwrap_or(0);
        let bytes = if fm > 0 {
            fm
        } else {
            item["bytes"].as_u64().unwrap_or(0)
        };
        ranked.push(json!({
            "kind": "skill",
            "agent": item["agent"],
            "name": item["name"],
            "bytes": bytes,
            "tokens_est": tokens_est(bytes),
        }));
    }
    for item in &mcp_servers {
        if let Some(b) = item["schema_bytes"].as_u64() {
            ranked.push(json!({
                "kind": "mcp_server",
                "agent": item["agent"],
                "name": item["name"],
                "tools_count": item["tools_count"],
                "bytes": b,
                "tokens_est": tokens_est(b),
            }));
        }
    }
    ranked.sort_by(|a, b| {
        b["tokens_est"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&a["tokens_est"].as_u64().unwrap_or(0))
            .then_with(|| {
                a["name"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["name"].as_str().unwrap_or(""))
            })
    });

    let instruction_bytes: u64 = instructions
        .iter()
        .filter_map(|i| i["bytes"].as_u64())
        .sum();
    let skill_full_bytes: u64 = skills.iter().filter_map(|s| s["bytes"].as_u64()).sum();
    let skill_prefix_bytes: u64 = skills
        .iter()
        .map(|s| {
            let fm = s["frontmatter_bytes"].as_u64().unwrap_or(0);
            if fm > 0 {
                fm
            } else {
                s["bytes"].as_u64().unwrap_or(0)
            }
        })
        .sum();
    let measured_prefix_bytes = instruction_bytes + skill_prefix_bytes + mcp_schema_bytes;
    let measured_prefix_tokens = tokens_est(measured_prefix_bytes);

    Ok(json!({
        "agent_filter": agent_filter,
        "probe_mcp": probe_mcp,
        "totals": {
            "instruction_files": instructions.len(),
            "instruction_bytes": instruction_bytes,
            "instruction_tokens_est": tokens_est(instruction_bytes),
            "skills": skills.len(),
            "skill_prefix_bytes": skill_prefix_bytes,
            "skill_full_bytes": skill_full_bytes,
            "skill_prefix_tokens_est": tokens_est(skill_prefix_bytes),
            "mcp_servers": mcp_servers.len(),
            "mcp_probed": mcp_probed,
            "mcp_schema_bytes": mcp_schema_bytes,
            "mcp_tokens_est": tokens_est(mcp_schema_bytes),
            "measured_prefix_bytes": measured_prefix_bytes,
            "measured_prefix_tokens_est": measured_prefix_tokens,
            "projected_50_turn_cache_read_tokens": measured_prefix_tokens * 50,
        },
        "top_contributors": ranked.into_iter().take(25).collect::<Vec<_>>(),
        "instructions": instructions,
        "skills": skills,
        "mcp_servers": mcp_servers,
        "note": "Read-only local scan; counts and basenames only — no file contents, arguments, or env values are printed. Pass --probe-mcp to measure exact stdio MCP tools/list bytes."
    }))
}
