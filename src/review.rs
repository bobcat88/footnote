//! `footnote memory-review`: a bounded list of memories worth a second look (goal 11), each with its reason. Nothing changes
//! here: the user decides, with `forget <id>` (kept as history) or `memory-restore <id>` (undoes a forget or a supersession,
//! or approves a pending memory: a global preference an agent saved, held back until the user reviews it, #207).
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

/// Memories older than this are judged on use; session summaries older than this are judged as outdated.
const OLD_DAYS: i64 = 30;
/// Two memories of the same scope and kind whose word sets overlap this much read as one.
const SAME: f64 = 0.8;

fn words(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(str::to_lowercase)
        .collect()
}

fn overlap(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

pub fn review(limit: usize) -> Result<Value> {
    let state = crate::state::state_dir()?;
    let db = crate::status::memory_db(&state)?;
    if !db.exists() {
        return Ok(
            json!({"pending": [], "duplicates": [], "old_sessions": [], "unused": [], "counts": {}, "note": "no memories yet"}),
        );
    }
    let c = crate::app::open(&db, false)?;
    let mut s = c.prepare(
        "SELECT c.id,m.kind,m.scope,m.observed_at,c.text FROM memories m JOIN sources s ON s.id=m.source_id
         JOIN chunks c ON c.source_id=s.id WHERE s.status='indexed' ORDER BY c.id",
    )?;
    let live: Vec<(i64, String, String, String, String)> = s
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let old = (time::OffsetDateTime::now_utc().date() - time::Duration::days(OLD_DAYS)).to_string();
    let short = |t: &str| t.chars().take(200).collect::<String>();
    let item = |id: i64, kind: &str, scope: &str, date: &str, text: &str| json!({"id": id, "kind": kind, "scope": scope, "observed_at": date, "text": short(text)});

    // Near-duplicates within one scope and kind: the older one is the candidate to forget (or to supersede by hand).
    let mut groups: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, m) in live.iter().enumerate() {
        groups.entry((&m.1, &m.2)).or_default().push(i);
    }
    let sets: Vec<HashSet<String>> = live.iter().map(|m| words(&m.4)).collect();
    let mut duplicates = Vec::new();
    for idx in groups.values() {
        for (a, &i) in idx.iter().enumerate() {
            for &j in &idx[a + 1..] {
                let o = overlap(&sets[i], &sets[j]);
                if o >= SAME {
                    let (older, newer) = (&live[i], &live[j]); // ids grow with time
                    duplicates.push(json!({"overlap": (o * 100.0).round() / 100.0,
                        "older": item(older.0, &older.1, &older.2, &older.3, &older.4),
                        "newer": item(newer.0, &newer.1, &newer.2, &newer.3, &newer.4)}));
                }
            }
        }
    }

    let old_sessions: Vec<Value> = live
        .iter()
        .filter(|m| m.1 == "session" && m.3 < old)
        .map(|m| item(m.0, &m.1, &m.2, &m.3, &m.4))
        .collect();

    // Unused: given to no prompt in the hook's activity log. The log is capped, so its first date bounds what "unused" means.
    let log = std::fs::read_to_string(state.join("memory-hook/activity.jsonl")).unwrap_or_default();
    let (mut used, mut since) = (HashSet::new(), None::<String>);
    for v in log
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
    {
        if since.is_none() {
            since = v["t"]
                .as_i64()
                .and_then(|ms| time::OffsetDateTime::from_unix_timestamp(ms / 1000).ok())
                .map(|t| t.date().to_string());
        }
        used.extend(
            v["ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_i64),
        );
    }
    let unused: Vec<Value> = if log.is_empty() {
        vec![]
    } else {
        live.iter()
            .filter(|m| m.3 < old && m.1 != "session" && !used.contains(&m.0))
            .map(|m| item(m.0, &m.1, &m.2, &m.3, &m.4))
            .collect()
    };

    let pending = crate::memory::pending(&c)?;
    let counts = json!({"pending": pending.len(), "memories": live.len(), "duplicates": duplicates.len(), "old_sessions": old_sessions.len(), "unused": unused.len()});
    let take = |v: Vec<Value>| v.into_iter().take(limit).collect::<Vec<_>>();
    Ok(json!({
        "pending": take(pending), "duplicates": take(duplicates), "old_sessions": take(old_sessions), "unused": take(unused), "counts": counts,
        "unused_since": since, "older_than": old,
        "how": "Nothing was changed. `footnote memory-restore <id>` approves a pending memory or brings a retired one back; `footnote forget <id>` rejects or retires one (kept as history).",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlap_reads_reworded_copies_as_one_and_different_facts_as_two() {
        let a =
            words("Johan wants branches pruned after their PR is merged, locally and on GitHub.");
        let b = words("Johan wants branches pruned after their PR is merged locally and on GitHub");
        let c = words("The daemon falls back to serving in-process when the relay loses it.");
        assert!(overlap(&a, &b) >= SAME && overlap(&a, &c) < 0.2);
    }
}
