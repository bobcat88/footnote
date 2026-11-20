//! `code-index --deep` (#299): make sure the compiler indexer a project needs is there, never installing anything without a
//! yes, and run every indexer with a timeout and capped output. The index an indexer writes is untrusted data (see `precise_at`).
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// A compiler indexer: its upstream, its licence (checked 20 Nov 2026 with `npm view`) and the one command that installs it.
pub struct Tool {
    pub name: &'static str,
    pub launcher: &'static str,
    pub install: &'static [&'static str],
    pub licence: &'static str,
    pub source: &'static str,
}

pub const TYPESCRIPT: Tool = Tool {
    name: "scip-typescript",
    launcher: "scip-typescript",
    install: &["npm", "install", "-g", "@sourcegraph/scip-typescript"],
    licence: "Apache-2.0",
    source: "https://github.com/sourcegraph/scip-typescript",
};
pub const PYTHON: Tool = Tool {
    name: "scip-python",
    launcher: "scip-python",
    install: &["npm", "install", "-g", "@sourcegraph/scip-python"],
    licence: "MIT",
    source: "https://github.com/sourcegraph/scip-python",
};
pub const RUST: Tool = Tool {
    name: "rust-analyzer",
    launcher: "rust-analyzer",
    install: &["rustup", "component", "add", "rust-analyzer"],
    licence: "MIT OR Apache-2.0",
    source: "https://github.com/rust-lang/rust-analyzer",
};

/// How `--deep` treats a missing indexer.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    /// Only say what is missing and the command that installs it (the default without a terminal, and `--no-install`).
    Report,
    /// Ask on the terminal before running the command.
    Ask,
    /// `--yes`: run the command without asking.
    Yes,
}

/// The world `prepare` touches, so tests can stand in for PATH, the terminal and the installer.
pub trait Env {
    fn present(&mut self, tool: &Tool) -> bool;
    fn ask(&mut self, question: &str) -> bool;
    fn install(&mut self, tool: &Tool) -> Result<()>;
}

/// The indexer `index()` would pick for the project's top folder (same marker files, same order).
pub fn tool_for(root: &Path) -> Option<&'static Tool> {
    let has = |f: &str| root.join(f).is_file();
    // shortcut: top folder only; a Rust crate in a subfolder is not checked here, `index()` still tries it. Upgrade when asked.
    if has("Cargo.toml") {
        Some(&RUST)
    } else if has("tsconfig.json") {
        Some(&TYPESCRIPT)
    } else if [
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "requirements.txt",
    ]
    .iter()
    .any(|f| has(f))
    {
        Some(&PYTHON)
    } else {
        None
    }
}

fn launcher(tool: &Tool) -> String {
    if cfg!(windows) && tool.launcher != "rust-analyzer" {
        format!("{}.cmd", tool.launcher)
    } else {
        tool.launcher.into()
    }
}

/// Whether the indexer runs here. rustup's `rust-analyzer` proxy exists without the component and says "Unknown binary".
pub fn is_present(tool: &Tool) -> bool {
    run_limited(
        Command::new(launcher(tool)).arg("--version"),
        Duration::from_secs(60),
    )
    .is_ok_and(|o| !String::from_utf8_lossy(&o.stderr).contains("Unknown binary"))
}

/// The real system: PATH, the terminal, and an install run exactly as printed (no shell, constant arguments).
pub struct System;

impl Env for System {
    fn present(&mut self, tool: &Tool) -> bool {
        is_present(tool)
    }
    fn ask(&mut self, question: &str) -> bool {
        eprint!("{question} [y/N] ");
        let mut line = String::new();
        io::stdin().read_line(&mut line).is_ok() && matches!(line.trim(), "y" | "Y" | "yes")
    }
    fn install(&mut self, tool: &Tool) -> Result<()> {
        let program = match tool.install[0] {
            "npm" if cfg!(windows) => "npm.cmd",
            p => p,
        };
        let out = run_limited(
            Command::new(program).args(&tool.install[1..]),
            Duration::from_secs(600),
        )?;
        eprint!("{}", String::from_utf8_lossy(&out.stderr));
        anyhow::ensure!(
            out.status.success(),
            "{} exited with {}",
            command(tool),
            out.status
        );
        Ok(())
    }
}

pub fn command(tool: &Tool) -> String {
    tool.install.join(" ")
}

/// Check the indexer the project needs; offer or run its install as `mode` allows. One entry in the answer per indexer.
pub fn prepare(root: &Path, mode: Mode, env: &mut dyn Env) -> Vec<Value> {
    let Some(tool) = tool_for(root) else {
        return vec![];
    };
    let mut entry = json!({"indexer": tool.name, "licence": tool.licence, "source": tool.source, "install": command(tool)});
    entry["status"] = if env.present(tool) {
        "ready".into()
    } else {
        let go = match mode {
            Mode::Yes => true,
            Mode::Ask => env.ask(&format!(
                "{} ({}, {}) is not installed. Run `{}`?",
                tool.name,
                tool.licence,
                tool.source,
                command(tool)
            )),
            Mode::Report => false,
        };
        if !go {
            entry["next"] = json!(format!(
                "run `{}` yourself, or run again with --yes to let Footnote run it",
                command(tool)
            ));
            "missing".into()
        } else if let Err(e) = env.install(tool) {
            entry["error"] = json!(format!("{e:#}"));
            "install_failed".into()
        } else if env.present(tool) {
            "installed".into()
        } else {
            entry["error"] = json!("installed, but not found on PATH (open a new terminal)");
            "install_failed".into()
        }
    };
    vec![entry]
}

/// Longest an indexer may run, `FOOTNOTE_CODE_INDEXER_TIMEOUT_SECS` to change (default 30 minutes).
pub fn timeout() -> Duration {
    let secs = std::env::var("FOOTNOTE_CODE_INDEXER_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800);
    Duration::from_secs(secs)
}

/// An index larger than this is not read: it is data from another program, and is read whole into memory.
pub const MAX_INDEX_BYTES: u64 = 512 << 20;
const MAX_OUTPUT: usize = 64 << 10;

/// Keep the first `MAX_OUTPUT` bytes of a stream and drop the rest, reading to the end so the child never blocks on a full pipe.
fn drain(mut r: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let (mut keep, mut buf) = (Vec::new(), [0u8; 8192]);
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            let room = MAX_OUTPUT.saturating_sub(keep.len());
            keep.extend_from_slice(&buf[..n.min(room)]);
        }
        keep
    })
}

/// Run a command with no stdin, capped output and a time limit; a command past its limit is killed with its children.
pub fn run_limited(cmd: &mut Command, limit: Duration) -> io::Result<Output> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (out, err) = (
        drain(child.stdout.take().expect("piped")),
        drain(child.stderr.take().expect("piped")),
    );
    let started = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if started.elapsed() > limit {
            if cfg!(windows) {
                // The npm launcher is a .cmd that starts node: kill the whole tree, not only the launcher.
                let _ = Command::new("taskkill")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .output();
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("timed out after {} s", limit.as_secs()),
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    Ok(Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        present: bool,
        yes: bool,
        asked: u32,
        installed: u32,
    }
    impl Env for Fake {
        fn present(&mut self, _: &Tool) -> bool {
            self.present
        }
        fn ask(&mut self, _: &str) -> bool {
            self.asked += 1;
            self.yes
        }
        fn install(&mut self, _: &Tool) -> Result<()> {
            self.installed += 1;
            self.present = true;
            Ok(())
        }
    }

    fn ts_project() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("tsconfig.json"), "{}").unwrap();
        d
    }

    fn run(mode: Mode, present: bool, yes: bool) -> (Fake, Value) {
        let d = ts_project();
        let mut f = Fake {
            present,
            yes,
            asked: 0,
            installed: 0,
        };
        let v = prepare(d.path(), mode, &mut f).remove(0);
        (f, v)
    }

    #[test]
    fn a_missing_indexer_is_reported_and_never_installed_without_a_yes() {
        let (f, v) = run(Mode::Report, false, true);
        assert_eq!(
            (f.asked, f.installed, v["status"].as_str()),
            (0, 0, Some("missing"))
        );
        assert_eq!(v["install"], "npm install -g @sourcegraph/scip-typescript");
        let (f, v) = run(Mode::Ask, false, false);
        assert_eq!(
            (f.asked, f.installed, v["status"].as_str()),
            (1, 0, Some("missing"))
        );
    }

    #[test]
    fn confirmation_or_yes_runs_the_install_once() {
        let (f, v) = run(Mode::Ask, false, true);
        assert_eq!(
            (f.asked, f.installed, v["status"].as_str()),
            (1, 1, Some("installed"))
        );
        let (f, v) = run(Mode::Yes, false, false);
        assert_eq!(
            (f.asked, f.installed, v["status"].as_str()),
            (0, 1, Some("installed"))
        );
    }

    #[test]
    fn an_installed_indexer_is_left_alone() {
        let (f, v) = run(Mode::Yes, true, true);
        assert_eq!(
            (f.asked, f.installed, v["status"].as_str()),
            (0, 0, Some("ready"))
        );
    }

    #[test]
    fn a_command_past_its_limit_is_killed() {
        let mut c = if cfg!(windows) {
            let mut c = Command::new("ping");
            c.args(["-n", "30", "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("30");
            c
        };
        let started = Instant::now();
        let e = run_limited(&mut c, Duration::from_millis(300)).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
