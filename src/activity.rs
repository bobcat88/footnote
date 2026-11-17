//! `footnote activity`: what memory did, day by day (the launcher's Activity screen; the visibility principle).
//! Saves come from the memory database; sessions and prompts given memories from the hook's activity log, which holds ids and
//! projects only, never what was asked.
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

/// Recalls listed in `recent`, newest first.
const RECENT: usize = 20;

pub fn activity(days: u16) -> Result<Value> {
    let state = crate::state::state_dir()?;
    let settings = footnote_core::config::product_settings(&state, "memory")?;
    let db = settings["db"]
        .as_str()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| state.join("memory.db"));
    let today = time::OffsetDateTime::now_utc().date();
    let first = today - time::Duration::days(i64::from(days) - 1);
    let mut by_day: BTreeMap<String, Value> = BTreeMap::new();
    for d in 0..days {
        let date = (first + time::Duration::days(i64::from(d))).to_string();
        by_day.insert(
            date.clone(),
            json!({"date": date, "saved": 0, "sessions": 0, "prompts_given_memories": 0}),
        );
    }

    let conn = if db.exists() {
        Some(crate::app::open(&db, false)?)
    } else {
        None
    };
    if let Some(c) = &conn {
        let mut s = c.prepare(
            "SELECT m.observed_at, count(*) FROM memories m JOIN sources s ON s.id=m.source_id WHERE s.status='indexed' AND m.observed_at >= ?1 GROUP BY m.observed_at",
        )?;
        for row in s.query_map([first.to_string()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })? {
            let (date, n) = row?;
            if let Some(day) = by_day.get_mut(&date) {
                day["saved"] = json!(n);
            }
        }
    }

    let since_ms = (first.midnight().assume_utc().unix_timestamp() * 1000).max(0) as u64;
    let mut used: HashMap<i64, u64> = HashMap::new();
    let mut recent: Vec<Value> = Vec::new();
    let log = std::fs::read_to_string(state.join("memory-hook/activity.jsonl")).unwrap_or_default();
    for v in log
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
    {
        let Some(t) = v["t"].as_u64().filter(|t| *t >= since_ms) else {
            continue;
        };
        let date = time::OffsetDateTime::from_unix_timestamp((t / 1000) as i64)
            .map(|d| d.date().to_string())
            .unwrap_or_default();
        let Some(day) = by_day.get_mut(&date) else {
            continue;
        };
        recent.push(json!({"t": t, "event": v["event"], "project": v["project"], "ids": v["ids"]}));
        let field = if v["event"] == "pack" {
            "sessions"
        } else {
            "prompts_given_memories"
        };
        day[field] = json!(day[field].as_u64().unwrap_or(0) + 1);
        for id in v["ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
        {
            *used.entry(id).or_default() += 1;
        }
    }

    // The newest recalls first, each with what reached the agent (never the prompt, which is not recorded).
    recent.reverse();
    recent.truncate(RECENT);
    let about = |id: i64| {
        conn.as_ref().and_then(|c| {
            c.query_row(
                "SELECT m.kind, m.scope, m.observed_at, c.text FROM chunks c JOIN sources s ON s.id=c.source_id JOIN memories m ON m.source_id=s.id WHERE c.id=?1",
                [id],
                |r| Ok(json!({"id": id, "kind": r.get::<_, String>(0)?, "scope": r.get::<_, String>(1)?, "observed_at": r.get::<_, String>(2)?,
                    "text": r.get::<_, String>(3)?.chars().take(120).collect::<String>()})),
            )
            .ok()
        })
        .unwrap_or_else(|| json!({"id": id, "gone": true}))
    };
    for r in &mut recent {
        let ids: Vec<i64> = r["ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
            .collect();
        r["memories"] = json!(ids.into_iter().map(about).collect::<Vec<_>>());
    }
    let mut top: Vec<(i64, u64)> = used.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top.truncate(10);
    let mut most_used = Vec::new();
    for (id, times) in top {
        let memory = conn.as_ref().and_then(|c| {
            c.query_row(
                "SELECT m.kind, m.scope, c.text FROM chunks c JOIN sources s ON s.id=c.source_id JOIN memories m ON m.source_id=s.id WHERE c.id=?1",
                [id],
                |r| Ok(json!({"kind": r.get::<_, String>(0)?, "scope": r.get::<_, String>(1)?, "text": r.get::<_, String>(2)?.chars().take(160).collect::<String>()})),
            )
            .ok()
        });
        most_used.push(json!({"id": id, "times": times, "memory": memory}));
    }
    Ok(json!({
        "days": by_day.into_values().collect::<Vec<_>>(),
        "most_used": most_used,
        "recent": recent,
        "activity_log": settings["activity"].as_bool().unwrap_or(true),
        "note": "dates in UTC; saves by the date the agent observed; prompt text is never recorded",
    }))
}
