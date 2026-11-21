//! Lean D (#243): preservation-first shell command runner (`footnote run`) and reversible local output store (`footnote output`).
//!
//! Core invariants:
//! - Never emit more bytes than the raw command output (including footer and code-graph links); if not smaller, pass raw through.
//! - On any filter error or panic, pass raw output and the exact child exit code through untouched.
//! - Output containing potential credentials is NEVER written to the local handle store (fail-closed).
//! - Preserve tracebacks, failing assertions, compiler spans, error lines, conflict markers, and byte-exact edit anchors.
use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Default threshold above which generic (unrecognized) command output is summarized.
const GENERIC_THRESHOLD_BYTES: usize = 12 * 1024;
/// Minimum stdout size for uniform JSON-array compaction (#381).
const JSON_ARRAY_MIN_BYTES: usize = 2 * 1024;
/// Minimum object count for uniform JSON-array compaction (#381).
const JSON_ARRAY_MIN_ITEMS: usize = 8;
/// Session window (seconds) during which identical consecutive failures collapse to a 3-line repeat delta (#381).
const REPEAT_WINDOW_SECS: u64 = 30 * 60;
/// Maximum number of `(cwd, command)` repeat-run entries retained on disk (#381).
const MAX_REPEAT_ENTRIES: usize = 64;
/// Expire stored raw output handles older than 24 hours.
const HANDLE_TTL_SECS: u64 = 24 * 3600;
/// Cap the local handle store to at most this many files (oldest evicted first).
const MAX_HANDLES: usize = 200;
/// Cap the raw output stored per handle (8 MiB) to bound disk usage.
const MAX_STORE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandFamily {
    CargoTest,
    CargoBuild,
    TestRunner,
    Git,
    Search,
    Linter,
    JsonArray,
    Generic,
}

impl CommandFamily {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CargoTest => "cargo-test",
            Self::CargoBuild => "cargo-build",
            Self::TestRunner => "tests",
            Self::Git => "git",
            Self::Search => "search",
            Self::Linter => "linter",
            Self::JsonArray => "json-array",
            Self::Generic => "generic",
        }
    }

    #[must_use]
    pub fn parse_override(name: &str) -> Option<Self> {
        match name {
            "cargo-test" => Some(Self::CargoTest),
            "cargo-build" => Some(Self::CargoBuild),
            "tests" => Some(Self::TestRunner),
            "git" => Some(Self::Git),
            "search" => Some(Self::Search),
            "linter" => Some(Self::Linter),
            "json-array" => Some(Self::JsonArray),
            "generic" => Some(Self::Generic),
            _ => None,
        }
    }
}

/// Classify a command invocation (`argv`) into its preservation-first filter family.
#[must_use]
pub fn classify_command(cmd: &[String]) -> CommandFamily {
    let Some(first) = cmd.first() else {
        return CommandFamily::Generic;
    };
    let bin = Path::new(first)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(first.as_str())
        .to_ascii_lowercase();
    let sub = cmd
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(|s| s.to_ascii_lowercase());

    match bin.as_str() {
        "cargo" => match sub.as_deref() {
            Some("test" | "nextest") => CommandFamily::CargoTest,
            Some("build" | "check" | "clippy" | "doc" | "b" | "c") => CommandFamily::CargoBuild,
            _ => CommandFamily::Generic,
        },
        "git" => match sub.as_deref() {
            Some("grep") => CommandFamily::Search,
            Some("status" | "diff" | "log" | "branch" | "show") => CommandFamily::Git,
            _ => CommandFamily::Generic,
        },
        "rg" | "grep" | "egrep" | "fgrep" | "findstr" | "ag" => CommandFamily::Search,
        "pytest" | "jest" | "vitest" | "mocha" => CommandFamily::TestRunner,
        "go" if sub.as_deref() == Some("test") => CommandFamily::TestRunner,
        "node" if cmd.iter().any(|a| a == "--test") => CommandFamily::TestRunner,
        "npm" | "pnpm" | "yarn" | "bun" if matches!(sub.as_deref(), Some("test" | "t")) => {
            CommandFamily::TestRunner
        }
        "python" | "python3" | "py" | "uv" => {
            if cmd.iter().any(|a| a == "pytest" || a == "unittest") {
                CommandFamily::TestRunner
            } else if cmd
                .iter()
                .any(|a| matches!(a.as_str(), "ruff" | "mypy" | "flake8"))
            {
                CommandFamily::Linter
            } else {
                CommandFamily::Generic
            }
        }
        "ruff" | "eslint" | "tsc" | "mypy" | "flake8" | "biome" | "pylint" => CommandFamily::Linter,
        "gh" if matches!(
            sub.as_deref(),
            Some("issue" | "pr" | "run" | "release" | "repo")
        ) && cmd.iter().any(|a| a == "--json") =>
        {
            CommandFamily::JsonArray
        }
        "npx" | "pnpx" | "bunx" => match sub.as_deref() {
            Some("jest" | "vitest" | "mocha" | "playwright") => CommandFamily::TestRunner,
            Some("eslint" | "tsc" | "biome") => CommandFamily::Linter,
            _ => CommandFamily::Generic,
        },
        _ => CommandFamily::Generic,
    }
}

/// Check whether `footnote` (or `footnote.exe` on Windows) is resolvable on the current `PATH`.
#[must_use]
pub fn footnote_on_path() -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    let bin_name = if cfg!(windows) {
        "footnote.exe"
    } else {
        "footnote"
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(bin_name).is_file())
}

/// Format an executable path cleanly for POSIX and Windows shells (quoting only if it contains whitespace).
fn format_exe_for_shell(exe: &Path) -> String {
    let norm = exe.to_string_lossy().replace('\\', "/");
    if norm.chars().any(char::is_whitespace) {
        format!("\"{norm}\"")
    } else {
        norm
    }
}

/// Decide whether a raw shell command string is safe to rewrite to `footnote run -- <cmd>` inside a PreToolUse hook.
/// Never touches pipes, redirects, chains, subshells, interactive tools, or commands already invoking `footnote`.
#[must_use]
pub fn rewrite_command(cmd: &str) -> Option<String> {
    rewrite_command_with_bin(cmd, "footnote")
}

/// Same as `rewrite_command`, using an explicit binary invocation string (e.g. an absolute path when `footnote` is
/// not on `PATH` in a non-interactive agent shell).
#[must_use]
pub fn rewrite_command_with_bin(cmd: &str, bin_cmd: &str) -> Option<String> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() || trimmed.contains('\n') || trimmed.contains('\r') {
        return None;
    }
    // Reject shell operators that pipe, redirect, chain, background, or run subshells.
    for forbidden in ["|", ">", "<", "&&", "||", ";", "$(", "`", "\0"] {
        if trimmed.contains(forbidden) {
            return None;
        }
    }
    let parts = split_simple_words(trimmed)?;
    let first = parts.first()?;
    let bin = Path::new(first)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(first.as_str())
        .to_ascii_lowercase();
    if matches!(
        bin.as_str(),
        "footnote" | "rtk" | "env" | "printenv" | "set" | "export" | "ssh" | "vim" | "less"
    ) {
        return None;
    }
    let family = classify_command(&parts);
    if family == CommandFamily::Generic {
        return None;
    }
    Some(format!("{bin_cmd} run -- {trimmed}"))
}

fn split_simple_words(s: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match (quote, c) {
            (Some(q), ch) if ch == q => quote = None,
            (Some(_), ch) => cur.push(ch),
            (None, '\'' | '"') => quote = Some(c),
            (None, ch) if ch.is_whitespace() => {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
            (None, ch) => cur.push(ch),
        }
    }
    if quote.is_some() {
        return None;
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    (!words.is_empty()).then_some(words)
}

/// Fail-closed detector for secrets/credentials. Any match prevents persisting raw output to disk.
#[must_use]
pub fn contains_secrets(text: &str) -> bool {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    let regexes = PATTERNS.get_or_init(|| {
        [
            r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----",
            r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
            r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{20,}\b",
            r"\bgithub_pat_[A-Za-z0-9_]{20,}\b",
            r"\bsk-(?:ant-|proj-|live-|test-)?[A-Za-z0-9_-]{20,}\b",
            r"(?i)\bAuthorization\s*:\s*(?:Bearer|Basic)\s+[A-Za-z0-9._~+/=-]{8,}",
            r"[a-zA-Z][a-zA-Z0-9+.-]*://[^/\s:@]{1,64}:[^/\s@]{6,}@[^\s/]+",
            r"(?i)\b(?:password|passwd|secret|api[_-]?key|access[_-]?token|private[_-]?key|client[_-]?secret|auth[_-]?token)\s*[:=]\s*['\x22]?[A-Za-z0-9_/+.-]{8,}",
            r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
        ]
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    });
    regexes.iter().any(|rx| rx.is_match(text))
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub raw: bool,
    pub no_store: bool,
    pub stdin_mode: bool,
    pub rewrite: Option<String>,
    pub family_override: Option<CommandFamily>,
    pub command: Vec<String>,
}

/// Execute `footnote run` directly, writing filtered (or raw) output to stdout and exiting with the child's exit code.
pub fn run_cli(opts: &RunOptions) -> i32 {
    if let Some(cmd) = &opts.rewrite {
        if let Some(rewritten) = rewrite_command(cmd) {
            println!("{rewritten}");
            return 0;
        }
        return 1;
    }
    match run_inner(opts) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("footnote run: {e:#}");
            1
        }
    }
}

fn run_inner(opts: &RunOptions) -> Result<i32> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (raw_output, exit_code, family) = if opts.stdin_mode {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("read stdin")?;
        let fam = opts.family_override.unwrap_or(CommandFamily::Generic);
        (buf, 0, fam)
    } else {
        if opts.command.is_empty() {
            bail!("expected a command after `--` (e.g. `footnote run -- cargo test`)");
        }
        let fam = opts
            .family_override
            .unwrap_or_else(|| classify_command(&opts.command));
        let (combined, code) = spawn_and_capture(&opts.command)?;
        (combined, code, fam)
    };

    if opts.raw || raw_output.is_empty() {
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(raw_output.as_bytes());
        let _ = stdout.flush();
        return Ok(exit_code);
    }

    let raw_bytes = raw_output.len();
    let has_secrets = contains_secrets(&raw_output);
    let family_sub = opts
        .command
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(String::as_str);

    // Track whether uniform JSON-array compaction fired so `footnote gain` attributes it cleanly (#381).
    let (effective_family, filtered_body) =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if matches!(family, CommandFamily::Generic | CommandFamily::JsonArray)
                && !has_secrets
                && let Some(compact) = filter_json_array(&raw_output)
            {
                return (CommandFamily::JsonArray, compact);
            }
            (family, filter_output(family, family_sub, &raw_output))
        }))
        .unwrap_or_else(|_| (family, raw_output.clone()));

    let repeat_eligible = !opts.stdin_mode
        && !opts.command.is_empty()
        && !has_secrets
        && !opts.no_store
        && matches!(
            effective_family,
            CommandFamily::CargoTest
                | CommandFamily::CargoBuild
                | CommandFamily::TestRunner
                | CommandFamily::Linter
        );

    if repeat_eligible && exit_code == 0 {
        clear_repeat_entry(&cwd, &opts.command);
    }

    // Consecutive identical failing run delta (#381): collapse repeat diagnostics to 3 lines referencing prior handle.
    let fail_sig = if repeat_eligible && exit_code != 0 {
        let sig = failure_signature(&filtered_body);
        if let Some((prior_handle, count)) =
            check_and_bump_repeat(&cwd, &opts.command, &sig, exit_code)
            && let Some(repeat_out) = format_repeat_delta(
                effective_family,
                &opts.command,
                exit_code,
                count,
                &filtered_body,
                raw_bytes,
                &prior_handle,
            )
            && repeat_out.len() < raw_bytes
        {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(repeat_out.as_bytes());
            let _ = stdout.flush();
            log_lean_run(
                effective_family,
                raw_bytes,
                repeat_out.len(),
                None,
                exit_code,
            );
            return Ok(exit_code);
        }
        Some(sig)
    } else {
        None
    };

    if filtered_body.len() >= raw_bytes {
        // On a failing diagnostic run that passed through raw, quietly keep a handle so an identical retry
        // can emit a 3-line repeat delta without losing access to the full diagnostic (#381).
        if let Some(sig) = fail_sig
            && (512..=MAX_STORE_BYTES).contains(&raw_bytes)
            && let Ok(id) = store_raw_output(&raw_output)
        {
            record_repeat_entry(&cwd, &opts.command, sig, id, exit_code, raw_bytes);
        }
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(raw_output.as_bytes());
        let _ = stdout.flush();
        log_lean_run(effective_family, raw_bytes, raw_bytes, None, exit_code);
        return Ok(exit_code);
    }

    #[cfg(feature = "code")]
    let code_links = if exit_code != 0 || has_error_markers(&filtered_body) {
        resolve_code_links(&cwd, &filtered_body)
    } else {
        String::new()
    };
    #[cfg(not(feature = "code"))]
    let code_links = {
        let _ = &cwd;
        String::new()
    };

    let candidate_without_footer = if code_links.is_empty() {
        filtered_body
    } else {
        format!("{}\n{code_links}", filtered_body.trim_end())
    };

    // Estimate footer length before deciding whether to persist a handle.
    let provisional_footer = if has_secrets {
        format!(
            "\n[footnote: {} {}B -> {}B | full output not stored: potential credentials detected]\n",
            effective_family.as_str(),
            raw_bytes,
            candidate_without_footer.len()
        )
    } else if opts.no_store || raw_bytes > MAX_STORE_BYTES {
        format!(
            "\n[footnote: {} {}B -> {}B | full output not stored]\n",
            effective_family.as_str(),
            raw_bytes,
            candidate_without_footer.len()
        )
    } else {
        format!(
            "\n[footnote: {} {}B -> {}B (-99%) | full: footnote output h-000000000000 [--grep pat]]\n",
            effective_family.as_str(),
            raw_bytes,
            candidate_without_footer.len()
        )
    };

    if candidate_without_footer.len() + provisional_footer.len() >= raw_bytes {
        if let Some(sig) = fail_sig
            && (512..=MAX_STORE_BYTES).contains(&raw_bytes)
            && let Ok(id) = store_raw_output(&raw_output)
        {
            record_repeat_entry(&cwd, &opts.command, sig, id, exit_code, raw_bytes);
        }
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(raw_output.as_bytes());
        let _ = stdout.flush();
        log_lean_run(effective_family, raw_bytes, raw_bytes, None, exit_code);
        return Ok(exit_code);
    }

    // Only write handle after confirming shortening saves bytes and output is credential-free.
    let handle_id = if !has_secrets && !opts.no_store && raw_bytes <= MAX_STORE_BYTES {
        store_raw_output(&raw_output).ok()
    } else {
        None
    };

    let body_trimmed = candidate_without_footer.trim_end();
    let pct = 100usize.saturating_sub(
        (body_trimmed.len().saturating_mul(100))
            .checked_div(raw_bytes)
            .unwrap_or(100),
    );
    let footer = match &handle_id {
        Some(id) => format!(
            "\n[footnote: {} {}B -> {}B (-{}%) | full: footnote output {} [--grep pat]]\n",
            effective_family.as_str(),
            raw_bytes,
            body_trimmed.len(),
            pct,
            id
        ),
        None if has_secrets => format!(
            "\n[footnote: {} {}B -> {}B (-{}%) | full output not stored: potential credentials detected]\n",
            effective_family.as_str(),
            raw_bytes,
            body_trimmed.len(),
            pct
        ),
        None => format!(
            "\n[footnote: {} {}B -> {}B (-{}%)]\n",
            effective_family.as_str(),
            raw_bytes,
            body_trimmed.len(),
            pct
        ),
    };

    let final_out = format!("{body_trimmed}{footer}");
    if final_out.len() >= raw_bytes {
        if let Some(id) = &handle_id {
            remove_handle(id);
        }
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(raw_output.as_bytes());
        let _ = stdout.flush();
        log_lean_run(effective_family, raw_bytes, raw_bytes, None, exit_code);
        return Ok(exit_code);
    }

    if let (Some(sig), Some(id)) = (fail_sig, &handle_id) {
        record_repeat_entry(&cwd, &opts.command, sig, id.clone(), exit_code, raw_bytes);
    }

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(final_out.as_bytes());
    let _ = stdout.flush();
    log_lean_run(
        effective_family,
        raw_bytes,
        final_out.len(),
        handle_id.as_deref(),
        exit_code,
    );
    Ok(exit_code)
}

fn spawn_and_capture(cmd: &[String]) -> Result<(String, i32)> {
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn `{}`", cmd[0]))?;

    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();

    let out_thread = std::thread::spawn(move || -> Vec<u8> {
        let mut buf = Vec::new();
        if let Some(ref mut r) = out_pipe {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    });
    let err_thread = std::thread::spawn(move || -> Vec<u8> {
        let mut buf = Vec::new();
        if let Some(ref mut r) = err_pipe {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    });

    let status = child.wait().context("wait for child process")?;
    let stdout_bytes = out_thread.join().unwrap_or_default();
    let stderr_bytes = err_thread.join().unwrap_or_default();
    let exit_code = status.code().unwrap_or(1);

    let stdout_str = String::from_utf8_lossy(&stdout_bytes);
    let stderr_str = String::from_utf8_lossy(&stderr_bytes);

    let combined = match (stdout_str.is_empty(), stderr_str.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout_str.into_owned(),
        (true, false) => stderr_str.into_owned(),
        (false, false) => {
            if stderr_str.ends_with('\n') {
                format!("{stderr_str}{stdout_str}")
            } else {
                format!("{stderr_str}\n{stdout_str}")
            }
        }
    };
    Ok((combined, exit_code))
}

/// Core preservation-first dispatcher across the shipped command families.
#[must_use]
pub fn filter_output(family: CommandFamily, subcommand: Option<&str>, raw: &str) -> String {
    match family {
        CommandFamily::CargoTest => filter_cargo_test(raw),
        CommandFamily::CargoBuild => filter_cargo_build(raw),
        CommandFamily::TestRunner => filter_test_runner(raw),
        CommandFamily::Git => filter_git(subcommand, raw),
        CommandFamily::Search => filter_search(raw),
        CommandFamily::Linter => filter_linter(raw),
        CommandFamily::JsonArray => filter_json_array(raw).unwrap_or_else(|| filter_generic(raw)),
        CommandFamily::Generic => filter_json_array(raw).unwrap_or_else(|| filter_generic(raw)),
    }
}

/// Compact a uniform JSON array of >= 8 objects (> 2 KB) with zero detected secrets into a schema-preserving
/// summary + key distributions + sample rows (#381). Returns `None` if criteria do not match.
#[must_use]
pub fn filter_json_array(raw: &str) -> Option<String> {
    if raw.len() <= JSON_ARRAY_MIN_BYTES || contains_secrets(raw) {
        return None;
    }
    let trimmed = raw.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    let Value::Array(items) = serde_json::from_str::<Value>(trimmed).ok()? else {
        return None;
    };
    if items.len() < JSON_ARRAY_MIN_ITEMS {
        return None;
    }
    let objs: Vec<&serde_json::Map<String, Value>> = items
        .iter()
        .map(Value::as_object)
        .collect::<Option<Vec<_>>>()?;
    if objs.iter().any(|o| o.is_empty()) {
        return None;
    }

    // Check schema uniformity: keys present in >= 50% of items form the core schema; every item must have
    // >= 60% of its keys in that core set.
    let mut key_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for obj in &objs {
        for k in obj.keys() {
            *key_counts.entry(k.as_str()).or_default() += 1;
        }
    }
    let min_common = objs.len().div_ceil(2);
    let core_keys: std::collections::HashSet<&str> = key_counts
        .iter()
        .filter(|(_, c)| **c >= min_common)
        .map(|(k, _)| *k)
        .collect();
    if core_keys.is_empty() {
        return None;
    }
    for obj in &objs {
        let shared = obj
            .keys()
            .filter(|k| core_keys.contains(k.as_str()))
            .count();
        if shared * 5 < obj.len() * 3 {
            return None;
        }
    }

    const PRIORITY_KEYS: &[&str] = &[
        "number", "id", "key", "name", "title", "state", "status", "kind", "type", "url",
    ];
    let mut ordered_keys: Vec<&str> = Vec::with_capacity(key_counts.len());
    for &pk in PRIORITY_KEYS {
        if key_counts.contains_key(pk) {
            ordered_keys.push(pk);
        }
    }
    for &k in key_counts.keys() {
        if !ordered_keys.contains(&k) {
            ordered_keys.push(k);
        }
    }

    let listed_keys = if ordered_keys.len() > 12 {
        format!(
            "{}, +{} more",
            ordered_keys[..12].join(", "),
            ordered_keys.len() - 12
        )
    } else {
        ordered_keys.join(", ")
    };

    let mut out = Vec::new();
    out.push(format!(
        "json array: {} items, {} keys ({listed_keys})",
        objs.len(),
        ordered_keys.len()
    ));

    // Summarize low-cardinality categorical fields (e.g. `state: OPEN=12, CLOSED=3`).
    let mut dists = Vec::new();
    for &field in &["state", "status", "kind", "type", "conclusion"] {
        if !key_counts.contains_key(field) {
            continue;
        }
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for obj in &objs {
            if let Some(s) = obj.get(field).and_then(Value::as_str) {
                *counts.entry(s).or_default() += 1;
            }
        }
        if !counts.is_empty() && counts.len() <= 6 {
            let parts: Vec<String> = counts
                .into_iter()
                .map(|(k, c)| format!("{k}={c}"))
                .collect();
            dists.push(format!("{field}: {}", parts.join(", ")));
        }
    }
    if !dists.is_empty() {
        out.push(format!("summary: {}", dists.join(" | ")));
    }

    // Pick up to 4 most informative non-blob keys for compact one-line sample rows.
    let display_keys: Vec<&str> = ordered_keys
        .iter()
        .copied()
        .filter(|k| !matches!(*k, "body" | "description" | "content" | "diff" | "raw"))
        .take(4)
        .collect();
    let display_keys = if display_keys.is_empty() {
        ordered_keys.iter().copied().take(3).collect()
    } else {
        display_keys
    };

    let format_row = |idx: usize, obj: &serde_json::Map<String, Value>| -> String {
        let mut parts = Vec::new();
        for &k in &display_keys {
            if let Some(v) = obj.get(k) {
                let rendered = compact_json_val(v);
                if !rendered.is_empty() {
                    parts.push(format!("{k}={rendered}"));
                }
            }
        }
        format!("  [{idx}] {}", parts.join(" "))
    };

    let head = 3usize.min(objs.len());
    for (i, obj) in objs.iter().take(head).enumerate() {
        out.push(format_row(i, obj));
    }
    if objs.len() > 5 {
        out.push(format!("  ... {} more items ...", objs.len() - 5));
        let start = objs.len() - 2;
        for (offset, obj) in objs[start..].iter().enumerate() {
            out.push(format_row(start + offset, obj));
        }
    } else if objs.len() > head {
        for (offset, obj) in objs[head..].iter().enumerate() {
            out.push(format_row(head + offset, obj));
        }
    }

    let rendered = out.join("\n");
    (rendered.len() < raw.len()).then_some(rendered)
}

fn compact_json_val(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            let one_line: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
            if one_line.chars().count() > 64 {
                let cut: String = one_line.chars().take(61).collect();
                format!("\"{cut}...\"")
            } else {
                format!("\"{one_line}\"")
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".into()
            } else {
                let names: Vec<&str> = arr
                    .iter()
                    .take(3)
                    .filter_map(|it| {
                        it.as_str()
                            .or_else(|| it.get("name").and_then(Value::as_str))
                            .or_else(|| it.get("login").and_then(Value::as_str))
                    })
                    .collect();
                if names.is_empty() {
                    format!("[{} items]", arr.len())
                } else if arr.len() > 3 {
                    format!("[{}, +{}]", names.join(","), arr.len() - 3)
                } else {
                    format!("[{}]", names.join(","))
                }
            }
        }
        Value::Object(map) => {
            for k in ["login", "name", "id", "title"] {
                if let Some(s) = map.get(k).and_then(Value::as_str) {
                    return s.to_string();
                }
            }
            format!("{{{} keys}}", map.len())
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RepeatEntry {
    sig: String,
    handle: String,
    exit_code: i32,
    raw_bytes: usize,
    count: u32,
    updated_at_secs: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn repeat_state_path() -> Result<PathBuf> {
    let dir = crate::state::state_dir()?.join("usage");
    crate::state::ensure_private_dir(&dir)?;
    Ok(dir.join("repeat-runs.json"))
}

fn repeat_key(cwd: &Path, cmd: &[String]) -> String {
    let joined = format!("{}|{}", cwd.to_string_lossy(), cmd.join("\0"));
    footnote_core::hash(joined.as_bytes())[..16].to_string()
}

/// Normalize timing/address jitter out of failing diagnostics while keeping every error, path, line number,
/// and assertion value intact (#381).
fn failure_signature(filtered: &str) -> String {
    static TIME_RE: OnceLock<Option<Regex>> = OnceLock::new();
    static HEX_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let time_re = TIME_RE
        .get_or_init(|| Regex::new(r"\b\d+(?:\.\d+)?\s*(?:ms|s)\b").ok())
        .as_ref();
    let hex_re = HEX_RE
        .get_or_init(|| Regex::new(r"\b0x[0-9a-fA-F]{6,}\b").ok())
        .as_ref();

    let clean = strip_ansi(filtered);
    let mut norm = String::new();
    for line in clean.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let s = match time_re {
            Some(re) => re.replace_all(t, "<T>").into_owned(),
            None => t.to_string(),
        };
        let s = match hex_re {
            Some(re) => re.replace_all(&s, "<HEX>").into_owned(),
            None => s,
        };
        norm.push_str(&s);
        norm.push('\n');
    }
    footnote_core::hash(norm.as_bytes())[..24].to_string()
}

fn load_repeat_state(path: &Path) -> BTreeMap<String, RepeatEntry> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_repeat_state(path: &Path, mut map: BTreeMap<String, RepeatEntry>) {
    let now = now_secs();
    map.retain(|_, v| now.saturating_sub(v.updated_at_secs) <= REPEAT_WINDOW_SECS);
    if map.len() > MAX_REPEAT_ENTRIES {
        let mut by_age: Vec<(u64, String)> = map
            .iter()
            .map(|(k, v)| (v.updated_at_secs, k.clone()))
            .collect();
        by_age.sort_by_key(|(t, _)| *t);
        for (_, old_k) in by_age.into_iter().take(map.len() - MAX_REPEAT_ENTRIES) {
            map.remove(&old_k);
        }
    }
    let tmp = path.with_extension("json.tmp");
    if let Ok(bytes) = serde_json::to_vec(&map)
        && fs::write(&tmp, bytes).is_ok()
    {
        let _ = fs::rename(&tmp, path);
    }
}

fn clear_repeat_entry(cwd: &Path, cmd: &[String]) {
    let Ok(path) = repeat_state_path() else {
        return;
    };
    if !path.is_file() {
        return;
    }
    let mut map = load_repeat_state(&path);
    if map.remove(&repeat_key(cwd, cmd)).is_some() {
        save_repeat_state(&path, map);
    }
}

fn check_and_bump_repeat(
    cwd: &Path,
    cmd: &[String],
    sig: &str,
    exit_code: i32,
) -> Option<(String, u32)> {
    let path = repeat_state_path().ok()?;
    let mut map = load_repeat_state(&path);
    let key = repeat_key(cwd, cmd);
    let now = now_secs();
    let entry = map.get_mut(&key)?;
    if now.saturating_sub(entry.updated_at_secs) > REPEAT_WINDOW_SECS
        || entry.sig != sig
        || entry.exit_code != exit_code
    {
        return None;
    }
    let handle_file = outputs_dir().ok()?.join(format!("{}.txt", entry.handle));
    if !handle_file.is_file() {
        return None;
    }
    entry.count = entry.count.saturating_add(1);
    entry.updated_at_secs = now;
    let res = (entry.handle.clone(), entry.count);
    save_repeat_state(&path, map);
    Some(res)
}

fn record_repeat_entry(
    cwd: &Path,
    cmd: &[String],
    sig: String,
    handle: String,
    exit_code: i32,
    raw_bytes: usize,
) {
    let Ok(path) = repeat_state_path() else {
        return;
    };
    let mut map = load_repeat_state(&path);
    map.insert(
        repeat_key(cwd, cmd),
        RepeatEntry {
            sig,
            handle,
            exit_code,
            raw_bytes,
            count: 1,
            updated_at_secs: now_secs(),
        },
    );
    save_repeat_state(&path, map);
}

fn format_repeat_delta(
    family: CommandFamily,
    cmd: &[String],
    exit_code: i32,
    count: u32,
    filtered: &str,
    raw_bytes: usize,
    prior_handle: &str,
) -> Option<String> {
    let cmd_preview: String = cmd.join(" ").chars().take(72).collect();
    let diag_lines = filtered.lines().filter(|l| !l.trim().is_empty()).count();
    let first_marker = filtered
        .lines()
        .find(|l| is_important_line(l))
        .or_else(|| filtered.lines().find(|l| !l.trim().is_empty()))
        .unwrap_or("identical failure")
        .trim();
    let first_marker: String = first_marker.chars().take(100).collect();
    let line1 =
        format!("repeat failure (#{count} identical in session, exit {exit_code}): {cmd_preview}");
    let line2 = format!("same diagnostic ({diag_lines} lines) — {first_marker}");
    let body_len = line1.len() + 1 + line2.len();
    let pct = 100usize.saturating_sub(
        (body_len.saturating_mul(100))
            .checked_div(raw_bytes)
            .unwrap_or(100),
    );
    let line3 = format!(
        "[footnote: {} repeat-delta {}B -> {}B (-{}%) | prior full: footnote output {} [--grep pat]]",
        family.as_str(),
        raw_bytes,
        body_len,
        pct,
        prior_handle
    );
    Some(format!("{line1}\n{line2}\n{line3}\n"))
}

fn strip_ansi(s: &str) -> String {
    static ANSI_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(rx) = ANSI_RE
        .get_or_init(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").ok())
        .as_ref()
    else {
        return s.to_string();
    };
    rx.replace_all(s, "").into_owned()
}

fn is_cargo_progress_line(trimmed: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "Compiling ",
        "Checking ",
        "Downloading ",
        "Downloaded ",
        "Updating ",
        "Locking ",
        "Fresh ",
        "Documenting ",
        "Blocking waiting for file lock",
        "Packaging ",
        "Archiving ",
    ];
    PREFIXES.iter().any(|p| trimmed.starts_with(p))
}

fn is_rustc_diag_start(trimmed: &str) -> bool {
    trimmed.starts_with("error[")
        || trimmed.starts_with("error:")
        || trimmed.starts_with("warning:")
        || trimmed.starts_with("warning[")
}

fn is_rustc_diag_continuation(line: &str, trimmed: &str) -> bool {
    trimmed.starts_with("--> ")
        || trimmed.starts_with("|")
        || trimmed.starts_with("= ")
        || trimmed.starts_with("note:")
        || trimmed.starts_with("help:")
        || trimmed.starts_with("...")
        || line
            .bytes()
            .take_while(|b| b.is_ascii_whitespace() || b.is_ascii_digit())
            .count()
            > 0
            && line.contains(" | ")
        || line.trim_start().starts_with("| ")
}

fn filter_cargo_build(raw: &str) -> String {
    let clean = strip_ansi(raw);
    let mut kept = Vec::new();
    let mut compiled = 0usize;
    let mut in_diag = false;

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_diag {
                kept.push(String::new());
                in_diag = false;
            }
            continue;
        }
        if is_cargo_progress_line(trimmed) {
            if trimmed.starts_with("Compiling ") || trimmed.starts_with("Checking ") {
                compiled += 1;
            }
            in_diag = false;
            continue;
        }
        if is_rustc_diag_start(trimmed) {
            in_diag = true;
            kept.push(line.to_string());
            continue;
        }
        if in_diag && is_rustc_diag_continuation(line, trimmed) {
            kept.push(line.to_string());
            continue;
        }
        in_diag = false;
        if trimmed.starts_with("Finished ")
            || trimmed.starts_with("error")
            || trimmed.contains("could not compile")
            || trimmed.contains("aborting due to")
            || trimmed.contains("generated ")
            || trimmed.starts_with("Caused by:")
            || trimmed.starts_with("note:")
        {
            kept.push(line.to_string());
        }
    }

    if kept.is_empty() && compiled > 0 {
        return format!("cargo: ok ({compiled} crates)");
    }
    if compiled > 0 && !kept.iter().any(|l| l.starts_with("Finished ")) {
        kept.insert(0, format!("cargo: {compiled} crates compiled/checked"));
    }
    kept.join("\n")
}

fn filter_cargo_test(raw: &str) -> String {
    let clean = strip_ansi(raw);
    let mut kept = Vec::new();
    let mut in_diag = false;
    let mut in_failures = false;
    let mut suites_ok = 0usize;
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut total_ignored = 0usize;

    static RESULT_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let result_re = RESULT_RE
        .get_or_init(|| {
            Regex::new(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;").ok()
        })
        .as_ref();

    for line in clean.lines() {
        let trimmed = line.trim();

        if trimmed == "failures:" {
            in_failures = true;
            in_diag = false;
            kept.push(line.to_string());
            continue;
        }

        if let Some(re) = result_re
            && let Some(cap) = re.captures(trimmed)
        {
            in_failures = false;
            in_diag = false;
            let status = cap.get(1).map_or("", |m| m.as_str());
            let passed: usize = cap
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let failed: usize = cap
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let ignored: usize = cap
                .get(4)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            total_passed += passed;
            total_failed += failed;
            total_ignored += ignored;
            if status == "ok" && failed == 0 {
                suites_ok += 1;
            } else {
                kept.push(line.to_string());
            }
            continue;
        }

        if in_failures {
            kept.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            if in_diag {
                kept.push(String::new());
                in_diag = false;
            }
            continue;
        }

        if is_cargo_progress_line(trimmed)
            || trimmed.starts_with("Running ")
            || trimmed.starts_with("Doc-tests ")
            || trimmed.starts_with("running ")
            || (trimmed.starts_with("test ") && trimmed.ends_with(" ... ok"))
            || (trimmed.starts_with("test ") && trimmed.ends_with(" ... ignored"))
            || trimmed.starts_with("Finished ")
        {
            in_diag = false;
            continue;
        }

        if is_rustc_diag_start(trimmed) {
            in_diag = true;
            kept.push(line.to_string());
            continue;
        }

        if in_diag && is_rustc_diag_continuation(line, trimmed) {
            kept.push(line.to_string());
            continue;
        }
        in_diag = false;

        if (trimmed.starts_with("test ") && trimmed.contains("FAILED"))
            || trimmed.contains("panicked at")
            || trimmed.contains("assertion")
            || trimmed.starts_with("error")
            || trimmed.contains("could not compile")
            || trimmed.contains("test failed")
        {
            kept.push(line.to_string());
        }
    }

    if suites_ok > 0 || total_passed > 0 || total_failed > 0 {
        let summary = format!(
            "test summary: {} suites ok, {} passed, {} failed, {} ignored",
            suites_ok, total_passed, total_failed, total_ignored
        );
        kept.push(summary);
    }

    if kept.is_empty() {
        raw.to_string()
    } else {
        kept.join("\n")
    }
}

fn filter_test_runner(raw: &str) -> String {
    let clean = strip_ansi(raw);
    let mut kept = Vec::new();
    let mut passed_count = 0usize;
    let mut in_failure_block = false;

    for line in clean.lines() {
        let trimmed = line.trim();

        // Detect passing individual test lines across pytest, node --test, jest/vitest, go test.
        if !in_failure_block
            && (trimmed.starts_with("✔ ")
                || trimmed.starts_with("✓ ")
                || trimmed.starts_with("PASS ")
                || trimmed.starts_with("--- PASS:")
                || trimmed.starts_with("=== RUN ")
                || trimmed.starts_with("ok  \t")
                || trimmed.ends_with(" PASSED")
                || trimmed.contains(" PASSED [")
                || (trimmed.chars().all(|c| c == '.' || c == 's' || c == ' ')
                    && trimmed.contains('.')))
        {
            passed_count += 1;
            continue;
        }

        // Skip pytest/node boilerplate lines outside failure blocks.
        if !in_failure_block
            && (trimmed.starts_with("platform ")
                || trimmed.starts_with("cachedir:")
                || trimmed.starts_with("rootdir:")
                || trimmed.starts_with("configfile:")
                || trimmed.starts_with("plugins:")
                || trimmed.starts_with("collecting ...")
                || trimmed.starts_with("collected ")
                || trimmed.starts_with("ℹ suites 0")
                || trimmed.starts_with("ℹ cancelled 0")
                || trimmed.starts_with("ℹ skipped 0")
                || trimmed.starts_with("ℹ todo 0")
                || trimmed.starts_with("ℹ duration_ms "))
        {
            continue;
        }

        if trimmed.starts_with("===") && trimmed.contains("FAILURES")
            || trimmed.starts_with("___ ")
            || trimmed.starts_with("FAIL ")
            || trimmed.starts_with("FAILED ")
            || trimmed.starts_with("--- FAIL:")
            || trimmed.starts_with("✖ ")
            || trimmed.starts_with("Traceback (most recent call last):")
        {
            in_failure_block = true;
            kept.push(line.to_string());
            continue;
        }

        if in_failure_block {
            if trimmed.starts_with("===")
                && (trimmed.contains("short test summary") || trimmed.contains(" passed"))
            {
                in_failure_block = false;
            }
            kept.push(line.to_string());
            continue;
        }

        if trimmed.starts_with("E   ")
            || trimmed.contains("FAILED")
            || trimmed.contains("ERROR")
            || trimmed.contains("AssertionError")
            || trimmed.contains("Assert")
            || trimmed.starts_with("ℹ tests ")
            || trimmed.starts_with("ℹ pass ")
            || trimmed.starts_with("ℹ fail ")
            || (trimmed.starts_with("===") && trimmed.ends_with("==="))
            || trimmed.starts_with("Tests:")
            || trimmed.starts_with("Test Suites:")
            || trimmed.starts_with("FAIL\t")
        {
            kept.push(line.to_string());
        }
    }

    if kept.is_empty() && passed_count > 0 {
        return format!("tests: ok ({passed_count} passed, 0 failed)");
    }
    if passed_count > 0
        && !kept
            .iter()
            .any(|l| l.contains("passed") || l.contains("pass "))
    {
        kept.push(format!("({passed_count} passed lines omitted)"));
    }
    if kept.is_empty() {
        raw.to_string()
    } else {
        kept.join("\n")
    }
}

fn filter_git(subcommand: Option<&str>, raw: &str) -> String {
    match subcommand {
        Some("status") => filter_git_status(raw),
        Some("log") => filter_git_log(raw),
        Some("diff" | "show") => filter_git_diff(raw),
        _ => filter_generic(raw),
    }
}

fn filter_git_status(raw: &str) -> String {
    let mut out = Vec::new();
    let mut section = "";
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("(use \"git ") {
            continue;
        }
        if trimmed.starts_with("On branch ")
            || trimmed.starts_with("HEAD detached")
            || trimmed.starts_with("Your branch ")
            || trimmed.starts_with("nothing to commit")
            || trimmed.starts_with("You have unmerged paths")
        {
            out.push(trimmed.to_string());
            continue;
        }
        if trimmed == "Changes to be committed:" {
            section = "staged";
            continue;
        }
        if trimmed == "Changes not staged for commit:" {
            section = "unstaged";
            continue;
        }
        if trimmed == "Untracked files:" {
            section = "untracked";
            continue;
        }
        if trimmed == "Unmerged paths:" {
            section = "conflict";
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("modified:") {
            let code = if section == "staged" { "M " } else { " M" };
            out.push(format!("{code} {}", rest.trim()));
        } else if let Some(rest) = trimmed.strip_prefix("new file:") {
            out.push(format!("A  {}", rest.trim()));
        } else if let Some(rest) = trimmed.strip_prefix("deleted:") {
            let code = if section == "staged" { "D " } else { " D" };
            out.push(format!("{code} {}", rest.trim()));
        } else if let Some(rest) = trimmed.strip_prefix("renamed:") {
            out.push(format!("R  {}", rest.trim()));
        } else if let Some(rest) = trimmed.strip_prefix("both modified:") {
            out.push(format!("UU {}", rest.trim()));
        } else if section == "untracked" {
            out.push(format!("?? {trimmed}"));
        } else if !trimmed.starts_with("no changes added")
            && !trimmed.starts_with("nothing added to commit")
        {
            out.push(trimmed.to_string());
        }
    }
    if out.is_empty() {
        raw.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_git_log(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if !lines.iter().any(|l| l.starts_with("commit ")) {
        return filter_generic(raw);
    }
    let mut out = Vec::new();
    let mut sha = String::new();
    let mut author = String::new();
    let mut date = String::new();
    let mut subject = String::new();
    let flush = |out: &mut Vec<String>,
                 sha: &mut String,
                 author: &mut String,
                 date: &mut String,
                 subject: &mut String| {
        if !sha.is_empty() {
            let short = &sha[..sha.len().min(8)];
            let a = author.split('<').next().unwrap_or(author).trim();
            out.push(format!("{short} {date} [{a}] {subject}"));
            sha.clear();
            author.clear();
            date.clear();
            subject.clear();
        }
    };
    for line in lines {
        if let Some(rest) = line.strip_prefix("commit ") {
            flush(&mut out, &mut sha, &mut author, &mut date, &mut subject);
            sha = rest.split_whitespace().next().unwrap_or("").to_string();
        } else if let Some(rest) = line.strip_prefix("Author:") {
            author = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Date:") {
            // Keep compact date (drop timezone noise when standard git date).
            let tokens: Vec<&str> = rest.split_whitespace().collect();
            date = if tokens.len() >= 5 {
                format!("{} {} {}", tokens[1], tokens[2], tokens[4])
            } else {
                rest.trim().to_string()
            };
        } else if line.starts_with("Merge:") {
            continue;
        } else if subject.is_empty() && !line.trim().is_empty() {
            subject = line.trim().to_string();
        } else if line.contains("files changed") || line.contains("file changed") {
            flush(&mut out, &mut sha, &mut author, &mut date, &mut subject);
            out.push(format!("  {}", line.trim()));
        }
    }
    flush(&mut out, &mut sha, &mut author, &mut date, &mut subject);
    if out.is_empty() {
        raw.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_git_diff(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("index ")
            || line.starts_with("old mode ")
            || line.starts_with("new mode ")
        {
            i += 1;
            continue;
        }
        // Keep headers, hunk markers, additions/deletions, conflict markers verbatim.
        let is_header = line.starts_with("diff --git ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@ ")
            || line.starts_with("Binary files ")
            || line.starts_with("new file mode ")
            || line.starts_with("deleted file mode ")
            || line.starts_with("rename from ")
            || line.starts_with("rename to ");
        let is_change = (line.starts_with('+') && !line.starts_with("+++ "))
            || (line.starts_with('-') && !line.starts_with("--- "))
            || line.starts_with("<<<<<<<")
            || line.starts_with("=======")
            || line.starts_with(">>>>>>>");

        if is_header || is_change {
            out.push(line);
        } else if line.starts_with(' ') {
            // Preserve 1 line of adjacent unchanged context around any edit as byte-exact anchor.
            let prev_change = i > 0 && is_diff_edit_line(lines[i - 1]);
            let next_change = i + 1 < lines.len() && is_diff_edit_line(lines[i + 1]);
            if prev_change || next_change {
                out.push(line);
            }
        } else if !line.is_empty() {
            out.push(line);
        }
        i += 1;
    }
    if out.is_empty() {
        raw.to_string()
    } else {
        out.join("\n")
    }
}

fn is_diff_edit_line(line: &str) -> bool {
    (line.starts_with('+') && !line.starts_with("+++ "))
        || (line.starts_with('-') && !line.starts_with("--- "))
}

fn filter_search(raw: &str) -> String {
    let clean = strip_ansi(raw);
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut ungrouped: Vec<String> = Vec::new();

    for line in clean.lines() {
        if let Some((path, rest)) = split_path_line(line) {
            if !groups.contains_key(path) {
                order.push(path.to_string());
            }
            groups
                .entry(path.to_string())
                .or_default()
                .push(rest.to_string());
        } else if !line.trim().is_empty() {
            ungrouped.push(line.to_string());
        }
    }

    if groups.is_empty() {
        return filter_generic(raw);
    }

    let mut out = Vec::new();
    for path in &order {
        if let Some(hits) = groups.get(path) {
            out.push(format!("{path}:"));
            for hit in hits {
                out.push(format!("  {hit}"));
            }
        }
    }
    for u in ungrouped {
        out.push(u);
    }
    out.join("\n")
}

/// Split `path:line:rest` or `C:\path:line:rest` into `(&str path, &str "line:rest")`.
fn split_path_line(line: &str) -> Option<(&str, &str)> {
    let start = if line.len() > 2 && line.as_bytes()[1] == b':' {
        2
    } else {
        0
    };
    let rel_colon = line[start..].find(':')?;
    let colon = start + rel_colon;
    let path = &line[..colon];
    if path.is_empty() || path.chars().any(char::is_whitespace) {
        return None;
    }
    let after = &line[colon + 1..];
    let digits = after.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || after.as_bytes().get(digits) != Some(&b':') {
        return None;
    }
    Some((path, after))
}

fn filter_linter(raw: &str) -> String {
    static TSC_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let tsc_re = TSC_RE
        .get_or_init(|| Regex::new(r"^([^\s(]+)\((\d+),(\d+)\):\s*(.+)$").ok())
        .as_ref();

    let clean = strip_ansi(raw);
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("help:")
            || trimmed.starts_with("[*]")
            || trimmed.starts_with("= help:")
        {
            continue;
        }
        if let Some(re) = tsc_re
            && let Some(cap) = re.captures(trimmed)
        {
            let path = cap.get(1).map_or("", |m| m.as_str());
            let ln = cap.get(2).map_or("", |m| m.as_str());
            let col = cap.get(3).map_or("", |m| m.as_str());
            let msg = cap.get(4).map_or("", |m| m.as_str());
            if !groups.contains_key(path) {
                order.push(path.to_string());
            }
            groups
                .entry(path.to_string())
                .or_default()
                .push(format!("{ln}:{col} {msg}"));
            continue;
        }
        if let Some((path, rest)) = split_path_line(trimmed) {
            if !groups.contains_key(path) {
                order.push(path.to_string());
            }
            groups
                .entry(path.to_string())
                .or_default()
                .push(rest.to_string());
            continue;
        }
        if trimmed.contains("Found ")
            || trimmed.contains("error")
            || trimmed.contains("warning")
            || trimmed.contains("problem")
        {
            summaries.push(trimmed.to_string());
        }
    }

    if groups.is_empty() {
        return filter_generic(raw);
    }

    let mut out = Vec::new();
    for path in &order {
        if let Some(diags) = groups.get(path) {
            out.push(format!("{path}:"));
            for d in diags {
                out.push(format!("  {d}"));
            }
        }
    }
    for s in summaries {
        out.push(s);
    }
    out.join("\n")
}

fn filter_generic(raw: &str) -> String {
    if raw.len() <= GENERIC_THRESHOLD_BYTES {
        return raw.to_string();
    }
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= 60 {
        return raw.to_string();
    }
    let head = 20usize;
    let tail = 20usize;
    let mut out: Vec<String> = Vec::new();
    for l in &lines[..head] {
        out.push((*l).to_string());
    }
    let mut middle_markers = Vec::new();
    for (idx, l) in lines[head..lines.len() - tail].iter().enumerate() {
        if is_important_line(l) && middle_markers.len() < 120 {
            middle_markers.push(format!("L{}: {l}", head + idx + 1));
        }
    }
    let omitted = lines
        .len()
        .saturating_sub(head + tail + middle_markers.len());
    out.push(format!(
        "[... {omitted} lines omitted, {} marker lines kept ...]",
        middle_markers.len()
    ));
    out.extend(middle_markers);
    out.push("[... tail ...]".to_string());
    for l in &lines[lines.len() - tail..] {
        out.push((*l).to_string());
    }
    out.join("\n")
}

fn is_important_line(line: &str) -> bool {
    let low = line.to_ascii_lowercase();
    low.contains("error")
        || low.contains("failed")
        || low.contains("panic")
        || low.contains("exception")
        || low.contains("traceback")
        || low.contains("assertion")
        || low.contains("warning:")
        || line.contains("--> ")
        || line.starts_with("<<<<<<<")
}

#[cfg(feature = "code")]
fn has_error_markers(text: &str) -> bool {
    text.lines().any(is_important_line)
}

/// Link failing tests or compiler errors to their symbols in the project's Footnote Code graph (read-only).
#[cfg(feature = "code")]
fn resolve_code_links(cwd: &Path, digest: &str) -> String {
    let Ok(db) = crate::app::code_db(cwd) else {
        return String::new();
    };
    if !db.is_file() {
        return String::new();
    }
    let Ok(conn) =
        rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return String::new();
    };

    static FILE_LINE_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let re = FILE_LINE_RE
        .get_or_init(|| {
            Regex::new(r"([A-Za-z0-9_./\\-]+\.(?:rs|py|ts|tsx|js|jsx|go|java)):(\d+)").ok()
        })
        .as_ref();
    let Some(re) = re else {
        return String::new();
    };

    let mut links = Vec::new();
    let mut seen_syms: std::collections::HashSet<String> = std::collections::HashSet::new();

    for cap in re.captures_iter(digest) {
        if links.len() >= 3 {
            break;
        }
        let raw_path = cap.get(1).map_or("", |m| m.as_str()).replace('\\', "/");
        let path = raw_path.trim_start_matches("./");
        let Ok(line) = cap.get(2).map_or("", |m| m.as_str()).parse::<i64>() else {
            continue;
        };
        let row: Option<(String, String, String, i64, i64)> = conn
            .query_row(
                "SELECT s.name, s.kind, f.path, s.line,
                        (SELECT count(DISTINCT r.caller_id) FROM refs r WHERE r.symbol_id = s.id AND r.caller_id != s.id)
                 FROM symbols s
                 JOIN files f ON f.id = s.file_id
                 WHERE replace(f.path, '\\', '/') = ?1
                   AND s.kind != 'module'
                   AND s.line <= ?2
                   AND (s.end_line IS NULL OR s.end_line >= ?2)
                 ORDER BY s.line DESC LIMIT 1",
                rusqlite::params![path, line],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .ok();
        if let Some((name, kind, fpath, sline, callers)) = row {
            let key = format!("{name}@{fpath}:{sline}");
            if seen_syms.insert(key) {
                links.push(format!(
                    "  {} ({}, {}:{}, {} callers) -> `footnote code {}`",
                    name,
                    kind,
                    fpath.replace('\\', "/"),
                    sline,
                    callers,
                    name
                ));
            }
        }
    }

    if links.is_empty() {
        String::new()
    } else {
        format!("code graph:\n{}", links.join("\n"))
    }
}

fn outputs_dir() -> Result<PathBuf> {
    let dir = crate::state::state_dir()?.join("outputs");
    crate::state::ensure_private_dir(&dir)?;
    Ok(dir)
}

pub(crate) fn store_raw_output(raw: &str) -> Result<String> {
    let dir = outputs_dir()?;
    prune_handles(&dir);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seed = format!("{now}:{}:{}", std::process::id(), raw.len());
    let mut hash_input = seed.into_bytes();
    hash_input.extend_from_slice(&raw.as_bytes()[..raw.len().min(4096)]);
    let digest = footnote_core::hash(&hash_input);
    let id = format!("h-{}", &digest[..12]);
    let target = dir.join(format!("{id}.txt"));
    let tmp = dir.join(format!(".{id}.{}.tmp", std::process::id()));
    fs::write(&tmp, raw.as_bytes()).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, &target).with_context(|| format!("persist {}", target.display()))?;
    Ok(id)
}

fn remove_handle(id: &str) {
    if let Ok(dir) = outputs_dir() {
        let _ = fs::remove_file(dir.join(format!("{id}.txt")));
    }
}

fn prune_handles(dir: &Path) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(HANDLE_TTL_SECS))
        .unwrap_or(UNIX_EPOCH);
    let mut entries: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);
        if mtime < cutoff {
            let _ = fs::remove_file(&path);
        } else {
            entries.push((mtime, path));
        }
    }
    if entries.len() > MAX_HANDLES {
        entries.sort_by_key(|(t, _)| *t);
        for (_, path) in &entries[..entries.len() - MAX_HANDLES] {
            let _ = fs::remove_file(path);
        }
    }
}

fn validate_handle_id(id: &str) -> Result<()> {
    let valid = (3..=32).contains(&id.len())
        && id.starts_with("h-")
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !valid {
        bail!("invalid output handle id `{id}` (expected `h-<hex>`)");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct OutputQuery {
    pub id: String,
    pub grep: Option<String>,
    pub head: Option<usize>,
    pub tail: Option<usize>,
    pub lines: Option<String>,
}

/// Read or search a stored full output handle (`footnote output <id>`).
pub fn output_cli(q: &OutputQuery) -> Result<Value> {
    validate_handle_id(&q.id)?;
    let path = outputs_dir()?.join(format!("{}.txt", q.id));
    if !path.is_file() {
        bail!(
            "output handle `{}` not found (expired, pruned, or not stored)",
            q.id
        );
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let all_lines: Vec<&str> = text.lines().collect();
    let total_lines = all_lines.len();

    let mut selected: Vec<(usize, &str)> = all_lines
        .iter()
        .enumerate()
        .map(|(i, l)| (i + 1, *l))
        .collect();

    if let Some(range) = &q.lines {
        let (a, b) = parse_line_range(range)?;
        selected.retain(|(ln, _)| *ln >= a && *ln <= b);
    }

    if let Some(pat) = &q.grep {
        if let Ok(re) = Regex::new(pat) {
            selected.retain(|(_, l)| re.is_match(l));
        } else {
            let needle = pat.to_ascii_lowercase();
            selected.retain(|(_, l)| l.to_ascii_lowercase().contains(&needle));
        }
    }

    if let Some(n) = q.head {
        selected.truncate(n);
    } else if let Some(n) = q.tail
        && selected.len() > n
    {
        selected = selected.split_off(selected.len() - n);
    }

    let rendered = selected
        .iter()
        .map(|(ln, l)| format!("{ln}: {l}"))
        .collect::<Vec<_>>()
        .join("\n");

    log_lean_retrieve(&q.id, rendered.len());

    Ok(json!({
        "id": q.id,
        "total_lines": total_lines,
        "returned_lines": selected.len(),
        "content": rendered,
    }))
}

fn parse_line_range(spec: &str) -> Result<(usize, usize)> {
    let (a, b) = spec
        .split_once(':')
        .or_else(|| spec.split_once('-'))
        .with_context(|| format!("invalid --lines range `{spec}` (expected start:end)"))?;
    let start: usize = a.trim().parse().context("invalid start line")?;
    let end: usize = b.trim().parse().context("invalid end line")?;
    if start == 0 || end < start {
        bail!("invalid --lines range `{spec}` (1-based start <= end required)");
    }
    Ok((start, end))
}

fn log_lean_run(
    family: CommandFamily,
    before: usize,
    after: usize,
    handle: Option<&str>,
    exit_code: i32,
) {
    let Ok(state) = crate::state::state_dir() else {
        return;
    };
    let on = footnote_core::config::product_settings(&state, "usage")
        .map_or(true, |v| v["lean_log"].as_bool().unwrap_or(true));
    if !on {
        return;
    }
    let dir = state.join("usage");
    let path = dir.join("lean.jsonl");
    if fs::metadata(&path).is_ok_and(|m| m.len() > 512 * 1024) {
        let _ = fs::rename(&path, path.with_extension("jsonl.old"));
    }
    let row = json!({
        "t": footnote_core::audit::now_ms(),
        "tool": "run",
        "family": family.as_str(),
        "before": before,
        "after": after,
        "stored": handle.is_some(),
        "handle": handle,
        "exit_code": exit_code,
    });
    let _ =
        fs::create_dir_all(&dir).and_then(|()| crate::state::append_line(&path, &row.to_string()));
}

fn log_lean_retrieve(handle: &str, returned_bytes: usize) {
    let Ok(state) = crate::state::state_dir() else {
        return;
    };
    let on = footnote_core::config::product_settings(&state, "usage")
        .map_or(true, |v| v["lean_log"].as_bool().unwrap_or(true));
    if !on {
        return;
    }
    let dir = state.join("usage");
    let path = dir.join("lean.jsonl");
    let row = json!({
        "t": footnote_core::audit::now_ms(),
        "tool": "output",
        "handle": handle,
        "miss": true,
        "before": 0,
        "after": returned_bytes,
    });
    let _ =
        fs::create_dir_all(&dir).and_then(|()| crate::state::append_line(&path, &row.to_string()));
}

/// Handle a `PreToolUse` (Claude Code, Codex, Antigravity) or `BeforeTool` (Gemini CLI) hook event JSON payload:
/// rewrite eligible shell commands to `footnote run -- <cmd>` using each CLI's native wire format.
/// Returns `None` when the event is not a rewritable shell call (passthrough untouched).
#[must_use]
pub fn pre_tool_use_hook(event: &Value) -> Option<Value> {
    let ev_name = event["hook_event_name"]
        .as_str()
        .or_else(|| event["hookEventName"].as_str())
        .unwrap_or("");
    if !ev_name.is_empty() && !matches!(ev_name, "PreToolUse" | "BeforeTool") {
        return None;
    }
    let is_agy = event.get("toolCall").is_some()
        || event.get("conversationId").is_some()
        || event.get("hookEventName").is_some();
    let is_gemini_cli = ev_name == "BeforeTool";

    let tool = event["tool_name"]
        .as_str()
        .or_else(|| event["toolName"].as_str())
        .or_else(|| event["toolCall"]["name"].as_str())
        .unwrap_or("");
    let is_shell = matches!(
        tool.to_ascii_lowercase().as_str(),
        "bash" | "shell" | "run_command" | "run_shell_command" | "command" | "terminal"
    );
    if !is_shell {
        return None;
    }
    let input = event
        .get("tool_input")
        .or_else(|| event.get("toolInput"))
        .or_else(|| event.get("toolCall").and_then(|tc| tc.get("args")))
        .and_then(Value::as_object)?;
    let (cmd_key, cmd_str) = if let Some(s) = input.get("command").and_then(Value::as_str) {
        ("command", s)
    } else {
        let s = input.get("CommandLine").and_then(Value::as_str)?;
        ("CommandLine", s)
    };
    let bin_cmd = if footnote_on_path() {
        "footnote".to_string()
    } else if let Ok(exe) = std::env::current_exe() {
        format_exe_for_shell(&exe)
    } else {
        "footnote".to_string()
    };
    let rewritten = rewrite_command_with_bin(cmd_str, &bin_cmd)?;
    if is_agy {
        // Antigravity PreToolUse contract: shallow merge into `toolCall.args` via `overwrite`.
        let mut overwrite = serde_json::Map::new();
        overwrite.insert(cmd_key.to_string(), json!(rewritten));
        return Some(json!({
            "decision": "allow",
            "overwrite": overwrite,
        }));
    }
    if is_gemini_cli {
        // Gemini CLI BeforeTool contract: override args via `hookSpecificOutput.tool_input`.
        let mut tool_input = serde_json::Map::new();
        tool_input.insert(cmd_key.to_string(), json!(rewritten));
        return Some(json!({
            "hookSpecificOutput": {
                "tool_input": tool_input,
            }
        }));
    }
    let mut updated = input.clone();
    updated.insert(cmd_key.to_string(), json!(rewritten));
    Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "updatedInput": updated,
        }
    }))
}

/// Standalone `footnote hook lean` entrypoint: fast, SQLite-free PreToolUse/BeforeTool command rewriter.
/// Never fails closed against the user's shell; any parse or I/O error passes through silently.
pub fn run_lean_hook() {
    const MAX_HOOK_EVENT: u64 = 2 * 1024 * 1024;
    if let Ok(state) = crate::state::state_dir() {
        let on = footnote_core::config::product_settings(&state, "usage")
            .map_or(true, |v| v["lean_log"].as_bool().unwrap_or(true));
        // Respect explicit company/user opt-out if usage.lean_enabled is set to false.
        let enabled = footnote_core::config::product_settings(&state, "usage")
            .map_or(true, |v| v["lean_enabled"].as_bool().unwrap_or(true));
        let _ = on;
        if !enabled {
            return;
        }
    }
    let mut raw = String::new();
    if std::io::stdin()
        .take(MAX_HOOK_EVENT + 1)
        .read_to_string(&mut raw)
        .is_err()
        || raw.len() as u64 > MAX_HOOK_EVENT
        || raw.trim().is_empty()
    {
        return;
    }
    let Ok(event) = serde_json::from_str::<Value>(&raw) else {
        return;
    };
    let wants_empty_json = event.get("toolCall").is_some()
        || event.get("conversationId").is_some()
        || event.get("hookEventName").is_some()
        || event["hook_event_name"].as_str() == Some("BeforeTool");
    if let Some(out) = pre_tool_use_hook(&event) {
        println!("{out}");
    } else if wants_empty_json {
        println!("{{}}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_all_five_core_families() {
        let v = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
        assert_eq!(
            classify_command(&v("cargo test --workspace")),
            CommandFamily::CargoTest
        );
        assert_eq!(
            classify_command(&v("cargo clippy --all-targets")),
            CommandFamily::CargoBuild
        );
        assert_eq!(classify_command(&v("pytest -q")), CommandFamily::TestRunner);
        assert_eq!(
            classify_command(&v("node --test a.test.js")),
            CommandFamily::TestRunner
        );
        assert_eq!(classify_command(&v("git status")), CommandFamily::Git);
        assert_eq!(classify_command(&v("git diff")), CommandFamily::Git);
        assert_eq!(classify_command(&v("rg TODO src")), CommandFamily::Search);
        assert_eq!(classify_command(&v("git grep foo")), CommandFamily::Search);
        assert_eq!(classify_command(&v("ruff check .")), CommandFamily::Linter);
        assert_eq!(classify_command(&v("tsc --noEmit")), CommandFamily::Linter);
    }

    #[test]
    fn rewrite_command_skips_pipes_redirects_chains_and_footnote() {
        assert_eq!(
            rewrite_command("cargo test --workspace").as_deref(),
            Some("footnote run -- cargo test --workspace")
        );
        assert_eq!(
            rewrite_command("git status").as_deref(),
            Some("footnote run -- git status")
        );
        assert_eq!(rewrite_command("rg foo src | head -n 5"), None);
        assert_eq!(rewrite_command("cargo test > out.txt"), None);
        assert_eq!(rewrite_command("git status && git diff"), None);
        assert_eq!(rewrite_command("footnote run -- cargo test"), None);
        assert_eq!(rewrite_command("rtk cargo test"), None);
        assert_eq!(rewrite_command("echo $(whoami)"), None);
        assert_eq!(rewrite_command("env"), None);
    }

    #[test]
    fn secret_detector_catches_common_credentials_and_ignores_benign_logs() {
        assert!(contains_secrets("AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE"));
        assert!(contains_secrets(
            "token ghp_1234567890abcdef1234567890abcdef"
        ));
        assert!(contains_secrets(
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.abcdef12345.sig12345678"
        ));
        assert!(contains_secrets("-----BEGIN OPENSSH PRIVATE KEY-----\nabc"));
        assert!(contains_secrets(
            "postgres://admin:supersecret99@db.example.com/app"
        ));
        assert!(contains_secrets(
            "api_key = \"sk-ant-api03-abcdefghijklmnopqrstuvwx\""
        ));
        assert!(!contains_secrets(
            "test result: ok. 42 passed; 0 failed; 0 ignored;"
        ));
        assert!(!contains_secrets(
            "error[E0308]: mismatched types\n --> src/main.rs:10:5"
        ));
    }

    #[test]
    fn cargo_test_preserves_failure_blocks_and_shrinks_passing_noise() {
        let mut raw = String::new();
        for i in 0..40 {
            raw.push_str(&format!("   Compiling crate-{i} v0.1.0\n"));
        }
        raw.push_str("    Finished `test` profile in 2.1s\n     Running unittests src/lib.rs\n\nrunning 30 tests\n");
        for i in 0..29 {
            raw.push_str(&format!("test suite::case_{i} ... ok\n"));
        }
        raw.push_str("test suite::broken_case ... FAILED\n\nfailures:\n\n---- suite::broken_case stdout ----\nthread 'suite::broken_case' panicked at src/lib.rs:42:9:\nassertion `left == right` failed\n  left: 1\n right: 2\n\nfailures:\n    suite::broken_case\n\ntest result: FAILED. 29 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n");

        let out = filter_output(CommandFamily::CargoTest, Some("test"), &raw);
        assert!(
            out.len() < raw.len() / 2,
            "expected >50% shrink: {} vs {}",
            out.len(),
            raw.len()
        );
        assert!(out.contains("suite::broken_case ... FAILED"));
        assert!(out.contains("panicked at src/lib.rs:42:9"));
        assert!(out.contains("left: 1"));
        assert!(out.contains("right: 2"));
        assert!(out.contains("29 passed; 1 failed"));
    }

    #[test]
    fn search_groups_by_file_preserving_every_line_and_match() {
        let raw = "crates/search/src/semantic.rs:10:fn alpha() {}\ncrates/search/src/semantic.rs:20:fn beta() {}\ncrates/search/src/semantic.rs:30:fn gamma() {}\ncrates/search/src/store.rs:5:fn open() {}\n";
        let out = filter_output(CommandFamily::Search, None, raw);
        assert!(out.len() < raw.len());
        assert!(out.contains("crates/search/src/semantic.rs:\n  10:fn alpha() {}\n  20:fn beta() {}\n  30:fn gamma() {}"));
        assert!(out.contains("crates/search/src/store.rs:\n  5:fn open() {}"));
    }

    #[test]
    fn git_diff_preserves_hunks_edits_and_adjacent_anchors() {
        let raw = "diff --git a/src/a.rs b/src/a.rs\nindex 1111111..2222222 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,9 +1,9 @@\n ctx1\n ctx2\n ctx3\n anchor_before\n-old_line\n+new_line\n anchor_after\n ctx4\n ctx5\n";
        let out = filter_output(CommandFamily::Git, Some("diff"), raw);
        assert!(out.len() < raw.len());
        assert!(out.contains("@@ -1,9 +1,9 @@"));
        assert!(out.contains(" anchor_before"));
        assert!(out.contains("-old_line"));
        assert!(out.contains("+new_line"));
        assert!(out.contains(" anchor_after"));
        assert!(!out.contains("index 1111111..2222222"));
        assert!(!out.contains("ctx1"));
    }

    #[test]
    fn handle_id_validation_rejects_traversal() {
        assert!(validate_handle_id("h-0123456789ab").is_ok());
        assert!(validate_handle_id("../memory").is_err());
        assert!(validate_handle_id("h-../../etc/passwd").is_err());
        assert!(validate_handle_id("memory.db").is_err());
    }
}
