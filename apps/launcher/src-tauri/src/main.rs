//! Footnote One's window. It draws from the `footnote` engine, run as a child process for each read: the window holds no
//! data of its own and can only run the commands in `allowed`: reads, turning a product on or off, and installing a model
//! (the one network step, started by a click, cancellable).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::Value;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// The model download in progress, so `cancel_install` can stop it.
static INSTALL: Mutex<Option<Child>> = Mutex::new(None);
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

/// The engine commands the window may run: read-only ones, and one write, a product's on/off switch (reversible; a company
/// policy still wins, enforced by the engine).
fn allowed(args: &[String]) -> bool {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["status"] | ["projects"] | ["activity"] | ["jobs"] | ["gain"] | ["models", "list"] => true,
        ["code", name, "--root", _] => !name.starts_with('-'),
        ["activity" | "usage" | "gain", "--days", d] => d.parse::<u16>().is_ok(),
        ["code-map", "--root", _] => true,
        // The impact radius before a change: a read, like `code`.
        ["code-impact", name, "--root", _] => !name.starts_with('-'),
        // The query follows `--`, so text starting with a dash is never read as an option.
        ["find", "--", q] => !q.trim().is_empty() && q.chars().count() <= 500,
        // One model by its catalogue name; the engine refuses names it does not know.
        ["models", "install", name] => {
            !name.is_empty()
                && name.len() <= 64
                && !name.starts_with('-')
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        }
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
    let missing = |e: std::io::Error| match e.kind() {
        std::io::ErrorKind::NotFound => "ENGINE_MISSING".to_string(),
        _ => e.to_string(),
    };
    let out = if args.first().is_some_and(|a| a == "models")
        && args.get(1).is_some_and(|a| a == "install")
    {
        install(cmd).map_err(missing)?
    } else {
        cmd.output().map_err(missing)?
    };
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

/// Run a model install so it can be cancelled: the child stays in `INSTALL` while it runs. The engine downloads to `.partial`
/// files renamed only when complete and hash-checked, so a cancelled install leaves no model half-installed.
fn install(mut cmd: Command) -> std::io::Result<std::process::Output> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let (out, err) = (
        child.stdout.take().expect("piped"),
        child.stderr.take().expect("piped"),
    );
    let read = |mut r: Box<dyn std::io::Read + Send>| {
        std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = r.read_to_end(&mut b);
            b
        })
    };
    let (read_out, read_err) = (read(Box::new(out)), read(Box::new(err)));
    *INSTALL.lock().unwrap_or_else(|p| p.into_inner()) = Some(child);
    let status = loop {
        let mut slot = INSTALL.lock().unwrap_or_else(|p| p.into_inner());
        match slot.as_mut().map(Child::try_wait) {
            Some(Ok(Some(status))) => {
                *slot = None;
                break status;
            }
            Some(Ok(None)) => {}
            Some(Err(e)) => {
                *slot = None;
                return Err(e);
            }
            None => return Err(std::io::Error::other("cancelled")),
        }
        drop(slot);
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    Ok(std::process::Output {
        status,
        stdout: read_out.join().unwrap_or_default(),
        stderr: read_err.join().unwrap_or_default(),
    })
}

/// Stop the model download in progress, if any. Returns whether one was running.
/// The support link: only these two fixed addresses can be opened, in the system browser or mail app (decided 17 Nov).
#[tauri::command]
fn open_support(to: String) -> Result<(), String> {
    let url = match to.as_str() {
        "issues" => "https://github.com/bobcat88/footnote/issues/new",
        "email" => "mailto:johan.velocity.lab@pm.me?subject=Footnote",
        _ => return Err("unknown support address".into()),
    };
    #[cfg(windows)]
    let mut cmd = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = std::process::Command::new("xdg-open");
    // explorer.exe returns 1 even when it opened the address, so only a failed start counts.
    cmd.arg(url).spawn().map(drop).map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_install() -> bool {
    let child = INSTALL.lock().unwrap_or_else(|p| p.into_inner()).take();
    child.is_some_and(|mut c| {
        let _ = c.kill();
        let _ = c.wait();
        true
    })
}

/// Show one of the two windows and hide the other, as the in-app switch does: one Footnote window at a time.
fn show(app: &tauri::AppHandle, label: &str) {
    let other = if label == "main" { "mini" } else { "main" };
    if let Some(w) = app.get_webview_window(other) {
        let _ = w.hide();
    }
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            footnote,
            cancel_install,
            open_support
        ])
        // The tray (decided 15 Nov: a compact tray panel plus the full window): a left click opens the mini panel, the menu
        // opens either window or quits. Quitting is only ever from this menu, so closing a window never loses the app.
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Open Footnote", true, None::<&str>)?;
            let mini = MenuItem::with_id(app, "mini", "Mini mode", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Footnote", true, None::<&str>)?;
            let mut tray = TrayIconBuilder::with_id("footnote")
                .tooltip("Footnote")
                .menu(&Menu::with_items(app, &[&open, &mini, &quit])?)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app, "main"),
                    "mini" => show(app, "mini"),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show(tray.app_handle(), "mini");
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        // Closing a window hides it; the app stays in the tray until "Quit Footnote".
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
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
                && ok("jobs")
                && ok("models list")
                && ok("code lookup --root D:/p")
                && ok("code-map --root . --depth 2")
        );
        assert!(!ok("jobs --bad"));
        assert!(!ok("forget 12") && !ok("install --memory") && !ok("models remove jina-v5-nano"));
        assert!(!ok("code --root . --root") && !ok("code-map --root . --depth 2;rm"));
        assert!(
            ok("usage --days 30")
                && ok("activity --days 7")
                && ok("gain")
                && ok("gain --days 7")
                && !ok("usage --days x")
        );
        assert!(ok("config set code.enabled false") && ok("config set memory.enabled true"));
        assert!(
            ok("models install jina-v5-nano")
                && !ok("models install --all")
                && !ok("models install a b")
                && !ok("models install ../x")
                && !ok("models remove gte-small")
                && !ok("models use gte-small")
        );
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
