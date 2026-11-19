//! Explicit experimental personal-store commands; never used by the live agent hooks.
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use footnote_memory::{personal_store, recovery};
use std::{
    io::{self, IsTerminal, Read, Write},
    path::PathBuf,
};
use zeroize::Zeroizing;

#[derive(Subcommand)]
pub enum PersonalAction {
    /// Create a new encrypted store. Existing paths are never modified.
    Init {
        #[arg(long)]
        store: PathBuf,
    },
    /// Make Footnote Memory use this encrypted store (hook, agents, launcher). Re-run `footnote install --memory` afterwards so
    /// agents' MCP entries point at it. The plain memory database is left as it was.
    Use {
        #[arg(long)]
        store: PathBuf,
    },
    /// Go back to the plain memory database. The encrypted store is left as it was.
    StopUsing,
    /// Display the recovery key as a QR code in an interactive terminal.
    RecoveryQr {
        #[arg(long)]
        store: PathBuf,
    },
    /// Explicitly print the recovery key as text. Protect terminal history and redirected output.
    RecoveryText {
        #[arg(long)]
        store: PathBuf,
    },
    /// Write an encrypted backup to a new file.
    Backup {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Restore to a new store; read the recovery text from a pipe on standard input.
    Restore {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        backup: PathBuf,
    },
    /// Check that an existing store can be unlocked.
    Status {
        #[arg(long)]
        store: PathBuf,
    },
    /// Save memories from a JSON array on standard input into the encrypted store.
    Remember {
        #[arg(long)]
        store: PathBuf,
    },
    /// Recall memories using a query supplied on standard input (lexical search).
    Recall {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=20))]
        limit: u8,
        #[arg(long)]
        tag: Vec<String>,
    },
}

fn read_input(limit: usize) -> Result<Zeroizing<Vec<u8>>> {
    let mut input = Zeroizing::new(Vec::new());
    io::stdin()
        .lock()
        .take((limit + 1) as u64)
        .read_to_end(&mut input)?;
    if input.len() > limit {
        bail!("input exceeds the allowed size");
    }
    Ok(input)
}

fn render_qr(output: &mut impl Write, width: usize, modules: &[u8]) -> Result<()> {
    for y in 0..width + 8 {
        for x in 0..width + 8 {
            let dark = x >= 4
                && y >= 4
                && x < width + 4
                && y < width + 4
                && modules[(y - 4) * width + x - 4] == 1;
            write!(output, "{}  ", if dark { "\x1b[40m" } else { "\x1b[47m" })?;
        }
        writeln!(output, "\x1b[0m")?;
    }
    output.flush()?;
    Ok(())
}

pub fn run(action: &PersonalAction) -> Result<()> {
    match action {
        PersonalAction::Use { store } => {
            drop(personal_store::open_existing(store, false)?); // unlocks now, or fails before anything changes
            let db = std::path::absolute(store.join("memory.db"))?;
            let state = crate::state::state_dir()?;
            footnote_core::config::set_value(&state, "memory", "db", serde_json::json!(db))?;
            println!(
                "{}",
                serde_json::json!({"memory_db": db, "encrypted": true, "next": "run `footnote install --memory` so agents use it"})
            );
        }
        PersonalAction::StopUsing => {
            let state = crate::state::state_dir()?;
            footnote_core::config::set_value(&state, "memory", "db", serde_json::Value::Null)?;
            println!(
                "{}",
                serde_json::json!({"memory_db": state.join("memory.db"), "encrypted": false, "next": "run `footnote install --memory` so agents use it"})
            );
        }
        PersonalAction::Init { store } => {
            drop(personal_store::create_new(store)?);
            println!(
                "{}",
                serde_json::json!({"created": true, "store": store, "next": "Save the recovery key with personal recovery-qr or recovery-text. This experimental store is not connected to agent hooks."})
            );
        }
        PersonalAction::RecoveryText { store } => {
            let secret = personal_store::recovery_text(store)?;
            let mut output = io::stdout().lock();
            writeln!(output, "{}", secret.as_str())?;
        }
        PersonalAction::RecoveryQr { store } => {
            if !io::stdout().is_terminal() {
                bail!("recovery QR requires an interactive terminal; it contains the secret key");
            }
            let text = personal_store::recovery_text(store)?;
            let key = recovery::decode(&text)?;
            let (width, modules) = recovery::qr(&key)?;
            let mut output = io::stdout().lock();
            render_qr(&mut output, width, &modules)?;
        }
        PersonalAction::Backup { store, output } => {
            personal_store::backup(store, output)?;
            println!(
                "{}",
                serde_json::json!({"backup": output, "encrypted": true})
            );
        }
        PersonalAction::Restore { store, backup } => {
            if io::stdin().is_terminal() {
                bail!(
                    "pipe recovery text through standard input; do not put the key in a command argument"
                );
            }
            let mut input = Zeroizing::new(Vec::with_capacity(105));
            io::stdin().lock().take(105).read_to_end(&mut input)?;
            let text = std::str::from_utf8(&input).context("recovery input is not UTF-8")?;
            personal_store::restore_new(store, backup, text.trim_end_matches(['\r', '\n']))?;
            println!(
                "{}",
                serde_json::json!({"restored": true, "store": store, "note": "This experimental store is not connected to agent hooks."})
            );
        }
        PersonalAction::Status { store } => {
            drop(personal_store::open_existing(store, false)?);
            println!(
                "{}",
                serde_json::json!({"store": store, "unlocked": true, "experimental": true})
            );
        }
        PersonalAction::Remember { store } => {
            let input = read_input(512 * 1024)?;
            let items = serde_json::from_slice(&input).context("invalid memory JSON")?;
            let mut c = personal_store::open_existing(store, true)?;
            let receipt = footnote_memory::remember(&mut c, &items, &[], "cli")?;
            println!("{receipt}");
        }
        PersonalAction::Recall {
            store,
            project,
            limit,
            tag,
        } => {
            let input = read_input(514)?;
            let query = std::str::from_utf8(&input)
                .context("query is not UTF-8")?
                .trim_end_matches(['\r', '\n']);
            if query.trim().is_empty() || query.len() > 512 {
                bail!("query must be between 1 and 512 bytes");
            }
            let c = personal_store::open_existing(store, false)?;
            let result = footnote_memory::recall(
                &c,
                query,
                project.as_deref(),
                usize::from(*limit),
                &[],
                tag,
            )?;
            println!("{result}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_qr_has_a_quiet_zone_and_preserves_modules() {
        let (width, modules) = recovery::qr(&[0x29; 32]).unwrap();
        let mut rendered = Vec::new();
        render_qr(&mut rendered, width, &modules).unwrap();
        let text = std::str::from_utf8(&rendered).unwrap();
        let rows: Vec<_> = text.lines().collect();
        assert_eq!(rows.len(), width + 8);
        for (y, row) in rows.iter().enumerate() {
            let body = row.strip_suffix("\x1b[0m").unwrap();
            let (cells, rest) = body.as_bytes().as_chunks::<7>();
            assert!(rest.is_empty());
            assert_eq!(cells.len(), width + 8);
            for (x, cell) in cells.iter().enumerate() {
                let inside = x >= 4 && y >= 4 && x < width + 4 && y < width + 4;
                let dark = inside && modules[(y - 4) * width + x - 4] == 1;
                assert_eq!(cell, if dark { b"\x1b[40m  " } else { b"\x1b[47m  " });
            }
        }
    }
}
