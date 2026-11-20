//! `footnote status`: what is installed, on, registered and running, and what memory did this week (the launcher's overview;
//! visibility principle: the user sees that memory is used).
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

pub fn status() -> Result<Value> {
    let state = crate::state::state_dir()?;
    let policy = footnote_core::config::policy_path();
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "state_dir": state,
        "policy_file": policy.as_ref().filter(|p| p.exists()),
        "products": {
            "library": {"built": cfg!(feature = "library"), "enabled": enabled(&state, "library"),
                        "corpora": crate::find::corpora(&state).iter().map(|p| json!({"path": p, "exists": p.exists()})).collect::<Vec<_>>()},
            "memory": memory(&state)?,
            "code": code(&state),
        },
        "models": crate::models::list(&state)?,
    }))
}

/// Footnote Code: built in or not, how many project graphs exist, and whether the Rust indexer it needs is on the PATH.
fn code(state: &Path) -> Value {
    if !cfg!(feature = "code") {
        return json!({"built": false});
    }
    let enabled = enabled(state, "code");
    let graphs = std::fs::read_dir(state.join("code")).map_or(0, |d| {
        d.flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "db"))
            .count()
    });
    let indexer = std::process::Command::new("rust-analyzer")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    json!({"built": true, "enabled": enabled, "graphs": graphs, "rust_indexer": indexer})
}

/// A product's on/off switch as the user or company policy set it (on by default).
fn enabled(state: &Path, product: &str) -> bool {
    footnote_core::config::product_settings(state, product)
        .map_or(true, |v| v["enabled"].as_bool().unwrap_or(true))
}

#[cfg(not(feature = "memory"))]
fn memory(_: &Path) -> Result<Value> {
    Ok(json!({"built": false}))
}

/// Footnote Memory's database: the one its settings name, else `<state>/memory.db`.
#[cfg(feature = "memory")]
pub(crate) fn memory_db(state: &Path) -> Result<std::path::PathBuf> {
    let settings = footnote_core::config::product_settings(state, "memory")?;
    Ok(settings["db"]
        .as_str()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| state.join("memory.db")))
}

#[cfg(feature = "memory")]
fn memory(state: &Path) -> Result<Value> {
    let settings = footnote_core::config::product_settings(state, "memory")?;
    let db = memory_db(state)?;
    let mut out = json!({
        "built": true,
        "enabled": settings["enabled"].as_bool().unwrap_or(true),
        "db": db,
        "encrypted": crate::app::personal_dir(&db).is_some(),
        "agents": crate::install::memory_registrations()?,
        "daemon_running": db.exists() && crate::daemon::connect(&db).is_ok(),
    });
    if db.exists() {
        let c = crate::app::open(&db, false)?;
        let live = "FROM memories m JOIN sources s ON s.id=m.source_id WHERE s.status='indexed'";
        out["bytes"] = json!(std::fs::metadata(&db)?.len());
        out["memories"] = json!(c.query_row(&format!("SELECT count(*) {live}"), [], |r| {
            r.get::<_, i64>(0)
        })?);
        out["saved_last_7_days"] = json!(c.query_row(
            &format!("SELECT count(*) {live} AND m.observed_at >= date('now','-7 days')"),
            [],
            |r| r.get::<_, i64>(0)
        )?);
    }
    out["recalls_last_7_days"] = recalls_last_week(state);
    Ok(out)
}

/// Prompts that received memories this week, from the hook's activity log.
#[cfg(feature = "memory")]
fn recalls_last_week(state: &Path) -> Value {
    let Ok(text) = std::fs::read_to_string(state.join("memory-hook/activity.jsonl")) else {
        return json!(0);
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let since = now_ms.saturating_sub(7 * 24 * 3600 * 1000);
    let n = text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["event"] == "recall" && v["t"].as_u64().is_some_and(|t| t >= since))
        .count();
    json!(n)
}
