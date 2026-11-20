//! Opt-in observability: timings, failures and unfinished steps, one JSON line each in `<state dir>/metrics.jsonl`.
//!
//! Compiled in only with `cargo build --features metrics`. The macros test the feature of the crate that calls them, so
//! every crate using them forwards its own `metrics` feature to `footnote-core/metrics`. A normal build expands every `timed!` to the bare expression and
//! `started!` to nothing: no clock read, no allocation, no write, so release performance is untouched.
//!
//! Lines: `{"t":<unix ms>,"pid":..,"start":"cli.index"}` when a long step begins; `{"t":..,"pid":..,"span":"search.hybrid",
//! "ms":12.3,"ok":true}` when a step ends. A step that returned an error has `"ok":false`; a step with a start line and no end
//! line from the same process stopped (killed, hung or crashed). `footnote metrics` summarizes the file.

/// Time an expression that returns a `Result`, recording whether it succeeded. Without the feature: just the expression.
#[macro_export]
macro_rules! timed {
    ($name:expr, $body:expr) => {{
        #[cfg(feature = "metrics")]
        let __span = $crate::metrics::Span::new($name);
        let __result = $body;
        #[cfg(feature = "metrics")]
        __span.finish(__result.is_ok());
        __result
    }};
}
/// Mark the start of a long step, so a step that never finishes is visible. Without the feature: nothing.
#[macro_export]
macro_rules! started {
    ($name:expr) => {{
        #[cfg(feature = "metrics")]
        $crate::metrics::start($name);
    }};
}
pub use crate::{started, timed};

#[cfg(feature = "metrics")]
mod imp {
    use serde_json::{Value, json};
    use std::{
        fs::{File, OpenOptions},
        io::Write,
        sync::{Mutex, OnceLock},
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    static SINK: OnceLock<Option<Mutex<File>>> = OnceLock::new();

    fn write(mut v: Value) {
        let sink = SINK.get_or_init(|| {
            // Started by `cargo test` without its own state folder: never write into the user's real metrics file.
            if std::env::var_os("CARGO_MANIFEST_DIR").is_some()
                && std::env::var_os("FOOTNOTE_STATE_DIR").is_none()
            {
                return None;
            }
            let dir = crate::state::state_dir().ok()?;
            std::fs::create_dir_all(&dir).ok()?;
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("metrics.jsonl"));
            f.ok().map(Mutex::new)
        });
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        v["t"] = json!(t);
        v["pid"] = json!(std::process::id());
        if let Some(Ok(mut f)) = sink.as_ref().map(|m| m.lock()) {
            // One write per line (see state::append_line); observability never breaks the operation it observes.
            let _ = f.write_all(format!("{v}\n").as_bytes());
        }
    }

    pub fn start(name: impl Into<String>) {
        write(json!({"start": name.into()}));
    }

    pub struct Span {
        name: String,
        at: Instant,
    }
    impl Span {
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                at: Instant::now(),
            }
        }
        pub fn finish(self, ok: bool) {
            let ms = (self.at.elapsed().as_secs_f64() * 1e6).round() / 1e3;
            write(json!({"span": self.name, "ms": ms, "ok": ok}));
        }
    }
}
#[cfg(feature = "metrics")]
pub use imp::{Span, start};

/// `footnote metrics`: per step, how many, failures, p50, p95 and max milliseconds; then steps that started and never ended.
#[cfg(feature = "metrics")]
pub fn summary(top: usize) -> anyhow::Result<serde_json::Value> {
    use std::collections::{BTreeMap, HashMap};
    let path = crate::state::state_dir()?.join("metrics.jsonl");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut spans: BTreeMap<String, (Vec<f64>, usize)> = BTreeMap::new();
    let mut open: HashMap<(u64, String), Vec<u64>> = HashMap::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let pid = v["pid"].as_u64().unwrap_or(0);
        if let Some(name) = v["start"].as_str() {
            open.entry((pid, name.to_string()))
                .or_default()
                .push(v["t"].as_u64().unwrap_or(0));
        } else if let Some(name) = v["span"].as_str() {
            let e = spans.entry(name.to_string()).or_default();
            e.0.push(v["ms"].as_f64().unwrap_or(0.0));
            e.1 += usize::from(!v["ok"].as_bool().unwrap_or(true));
            if let Some(starts) = open.get_mut(&(pid, name.to_string())) {
                starts.pop();
            }
        }
    }
    let pct = |s: &[f64], p: f64| s[((s.len() - 1) as f64 * p).round() as usize];
    let mut rows: Vec<serde_json::Value> = spans
        .into_iter()
        .map(|(name, (mut ms, failed))| {
            ms.sort_by(f64::total_cmp);
            serde_json::json!({"step": name, "count": ms.len(), "failed": failed, "p50_ms": pct(&ms, 0.5),
                "p95_ms": pct(&ms, 0.95), "max_ms": pct(&ms, 1.0), "total_s": (ms.iter().sum::<f64>() / 10.0).round() / 100.0})
        })
        .collect();
    rows.sort_by(|a, b| {
        b["total_s"]
            .as_f64()
            .unwrap_or(0.0)
            .total_cmp(&a["total_s"].as_f64().unwrap_or(0.0))
    });
    rows.truncate(top);
    let mut unfinished: Vec<serde_json::Value> = open
        .into_iter()
        .flat_map(|((pid, name), ts)| {
            ts.into_iter()
                .map(move |t| serde_json::json!({"step": name, "pid": pid, "started": t}))
        })
        .collect();
    unfinished.sort_by_key(|v| std::cmp::Reverse(v["started"].as_u64().unwrap_or(0)));
    unfinished.truncate(top);
    Ok(
        serde_json::json!({"file": path, "steps": rows, "unfinished": unfinished,
        "note": "a step in `unfinished` started and never recorded an end in the same process: killed, hung or crashed (or still running)"}),
    )
}
