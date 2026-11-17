use anyhow::{ensure, Result};
use rusqlite::{Connection, OpenFlags};
use std::{path::PathBuf, time::Instant};

fn differences(a: &serde_json::Value, b: &serde_json::Value, path: &str, out: &mut std::collections::BTreeSet<String>) {
    match (a, b) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            for (k, v) in a { if let Some(w) = b.get(k) { differences(v, w, &format!("{path}.{k}"), out); } else { out.insert(format!("{path}.{k}:missing")); } }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            if a.len() != b.len() { out.insert(format!("{path}:length")); }
            for (v,w) in a.iter().zip(b) { differences(v,w,&format!("{path}[]"),out); }
        }
        _ => { if a != b { out.insert(path.to_owned()); } }
    }
}

fn main() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let plain = tmp.path().join("plain.db");
    let encrypted = tmp.path().join("encrypted.db");
    let db_path = std::env::args_os().nth(1).expect("usage: encryption_bench <memory.db>");
    let live = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    live.backup(rusqlite::MAIN_DB, &plain, None)?;
    drop(live);
    let p = Connection::open(&plain)?;
    let version: String = p.query_row("PRAGMA cipher_version", [], |r| r.get(0))?;
    let key = "benchmark-only-ephemeral-key";
    p.execute("ATTACH DATABASE ?1 AS encrypted KEY ?2", (encrypted.to_str().unwrap(), key))?;
    p.execute_batch("SELECT sqlcipher_export('encrypted'); DETACH DATABASE encrypted;")?;
    let denied = Connection::open(&encrypted)?.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).is_err();
    ensure!(denied, "encrypted database opened without key");
    let e = Connection::open(&encrypted)?;
    e.pragma_update(None, "key", key)?;
    let count: i64 = e.query_row("SELECT count(*) FROM memories", [], |r| r.get(0))?;
    ensure!(count == p.query_row("SELECT count(*) FROM memories", [], |r| r.get::<_, i64>(0))?, "snapshot differs");
    let queries = ["memory project", "benchmark results", "code search", "database", "configuration", "agent context", "test performance", "design", "model", "security"];
    let models: Vec<PathBuf> = vec![];
    let mut times = [Vec::<u128>::new(), Vec::<u128>::new()];
    let mut reference = vec![];
    for q in queries { reference.push(footnote_memory::recall(&p, q, Some("footnote"), 10, &models, &[])?); }
    let result_counts: Vec<usize> = reference.iter().map(|v| v["memories"].as_array().map_or(0, Vec::len)).collect();
    let mut mismatches = [0, 0];
    let mut set_mismatches = [0, 0];
    let mut diff_paths = std::collections::BTreeSet::new();
    // Alternate blocks; warm the rebuilt RAM index outside the timed interval.
    for round in 0..20 {
        for condition in if round % 2 == 0 { [0, 1] } else { [1, 0] } {
            let db = if condition == 0 { &p } else { &e };
            let _ = footnote_memory::recall(db, queries[0], Some("footnote"), 10, &models, &[])?;
            for (i, q) in queries.iter().enumerate() {
                let start = Instant::now();
                let value = footnote_memory::recall(db, q, Some("footnote"), 10, &models, &[])?;
                times[condition].push(start.elapsed().as_micros());
                if value != reference[i] {
                    mismatches[condition] += 1;
                    let ids = |v: &serde_json::Value| -> std::collections::BTreeSet<i64> { v["memories"].as_array().unwrap().iter().filter_map(|m| m["id"].as_i64()).collect() };
                    if ids(&value) != ids(&reference[i]) { set_mismatches[condition] += 1; }
                    differences(&reference[i], &value, "result", &mut diff_paths);
                }
            }
        }
    }
    for (name, values) in ["plain", "encrypted"].into_iter().zip(times.iter_mut()) {
        values.sort_unstable();
        println!("{name}: n={} p50_us={} p95_us={}", values.len(), values[values.len()/2], values[(values.len()*95/100)-1]);
    }
    println!("sqlcipher={version} memories={count} no_key_rejected={denied} ordered_mismatches_plain_encrypted={mismatches:?} id_set_mismatches_plain_encrypted={set_mismatches:?} results_per_query={result_counts:?} different_fields={diff_paths:?}");
    Ok(())
}
