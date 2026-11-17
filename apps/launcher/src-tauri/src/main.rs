//! Footnote One's window. It draws from the `footnote` engine, run as a child process for each read: the window holds no
//! data of its own and can only run the commands in `allowed`: reads, plus turning a product on or off.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

/// The engine commands the window may run: read-only ones, and one write, a product's on/off switch (reversible; a company
/// policy still wins, enforced by the engine).
fn allowed(args: &[String]) -> bool {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["status"] | ["projects"] | ["activity"] | ["models", "list"] => true,
        ["code", name, "--root", _] => !name.starts_with('-'),
        ["activity" | "usage", "--days", d] => d.parse::<u16>().is_ok(),
        ["code-map", "--root", _] => true,
        // The query follows `--`, so text starting with a dash is never read as an option.
        ["find", "--", q] => !q.trim().is_empty() && q.chars().count() <= 500,
        [
            "config",
            "set",
            "memory.enabled" | "library.enabled" | "code.enabled",
            "true" | "false",
        ] => true,
        ["code-map", "--root", _, "--depth", d] => d.parse::<u8>().is_ok(),
        _ => false,
    }
}

/// Where the engine is: `FOOTNOTE_BIN`, beside this app, the per-user install folder on Windows, then PATH.
fn engine() -> PathBuf {
    if let Some(p) = std::env::var_os("FOOTNOTE_BIN") {
        return p.into();
    }
    let exe = if cfg!(windows) {
        "footnote.exe"
    } else {
        "footnote"
    };
    let beside = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(exe)));
    let installed = std::env::var_os("LOCALAPPDATA")
        .map(|l| PathBuf::from(l).join("Programs").join("footnote").join(exe));
    [beside, installed.filter(|_| cfg!(windows))]
        .into_iter()
        .flatten()
        .find(|p| p.is_file())
        .unwrap_or_else(|| exe.into())
}

/// Run one allowed engine command and return its JSON. Errors are the engine's own message, or `ENGINE_MISSING`.
#[tauri::command(async)]
fn footnote(args: Vec<String>) -> Result<Value, String> {
    if !allowed(&args) {
        return Err(format!(
            "not allowed from the window: footnote {}",
            args.join(" ")
        ));
    }
    let mut cmd = Command::new(engine());
    cmd.args(&args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashes over the app
    }
    let out = cmd.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => "ENGINE_MISSING".to_string(),
        _ => e.to_string(),
    })?;
    let json: Value = serde_json::from_slice(&out.stdout)
        .map_err(|_| String::from_utf8_lossy(&out.stderr).trim().to_string())?;
    match json.get("error").and_then(Value::as_str) {
        Some(e) => Err(e.to_string()),
        None if !out.status.success() => {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
        None => Ok(json),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![footnote])
        // Closing the full window quits, even while the hidden mini window exists.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event
                && window.label() == "main"
            {
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("run the Footnote window");
}

#[cfg(test)]
mod tests {
    use super::allowed;

    fn ok(a: &str) -> bool {
        allowed(&a.split(' ').map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn only_read_commands_pass() {
        assert!(
            ok("status")
                && ok("models list")
                && ok("code lookup --root D:/p")
                && ok("code-map --root . --depth 2")
        );
        assert!(!ok("forget 12") && !ok("install --memory") && !ok("models install jina-v5-nano"));
        assert!(!ok("code --root . --root") && !ok("code-map --root . --depth 2;rm"));
        assert!(ok("usage --days 30") && ok("activity --days 7") && !ok("usage --days x"));
        assert!(ok("config set code.enabled false") && ok("config set memory.enabled true"));
        assert!(
            ok("find -- daemon") && ok("find -- --db") && !ok("find daemon") && !ok("find -- ")
        );
        assert!(
            !ok("config set memory.resident false")
                && !ok("config set code.enabled maybe")
                && !ok("config set code.enabled")
        );
    }
}
