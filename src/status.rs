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
            "library": {"built": cfg!(feature = "library")},
            "memory": memory(&state)?,
        },
        "models": crate::models::list(&state)?,
    }))
}

#[cfg(not(feature = "memory"))]
fn memory(_: &Path) -> Result<Value> {
    Ok(json!({"built": false}))
}

/// Footnote Memory's database: the one its settings name, else `<state>/memory.db`.
#[cfg(feature = "memory")]
pub(crate) fn memory_db(state: &Path) -> Result<std::path::PathBuf> {
    let settings = footnote_core::config::product_settings(state, "memory")?;
    Ok(settings["db"].as_str().map(std::path::PathBuf::from).unwrap_or_else(|| state.join("memory.db")))
}

#[cfg(feature = "memory")]
fn memory(state: &Path) -> Result<Value> {
    let settings = footnote_core::config::product_settings(state, "memory")?;
    let db = memory_db(state)?;
    let mut out = json!({
        "built": true,
        "enabled": settings["enabled"].as_bool().unwrap_or(true),
        "db": db,
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

/// Prompts that received memories this week, from the metrics file; null when metrics are off (nothing else records them).
#[cfg(feature = "memory")]
fn recalls_last_week(state: &Path) -> Value {
    let Ok(text) = std::fs::read_to_string(state.join("metrics.jsonl")) else {
        return Value::Null;
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let since = now_ms.saturating_sub(7 * 24 * 3600 * 1000);
    let n = text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| {
            v["span"] == "hook.recall"
                && v["ok"] == true
                && v["t"].as_u64().is_some_and(|t| t >= since)
        })
        .count();
    json!(n)
}
