//! `footnote usage`: tokens per day, agent and model, read from the agents' own session files on this computer (Claude Code
//! and Codex). Only counts are kept: no prompt or answer text is copied. Files are read incrementally: the new bytes since the
//! last run, with a guard that starts a file over when it was rewritten. Design: docs/roadmap/BACKLOG.md, "Spend and savings".
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// List prices in USD per million tokens, from the providers' pricing pages, read 16 Nov 2026:
/// https://platform.claude.com/docs/en/about-claude/pricing and https://developers.openai.com/api/docs/pricing (standard tier).
/// Changing a price: edit the row and `PRICES`; every file is then read again so old counts get the new prices.
/// shortcut: batch, fast tier, US-only inference (x1.1) and regional endpoints are not priced; add them when someone uses them.
const PRICES: &str = "2026-11-16b";

#[derive(Clone, Copy)]
struct Price {
    input: f64,
    write_5m: f64,
    write_1h: f64,
    read: f64,
    output: f64,
}

const fn p(input: f64, write_5m: f64, write_1h: f64, read: f64, output: f64) -> Price {
    Price {
        input,
        write_5m,
        write_1h,
        read,
        output,
    }
}

/// A model id prefix, its price, and the price for prompts above a number of tokens.
type Row = (&'static str, Price, Option<(u64, Price)>);

const TABLE: &[Row] = &[
    ("claude-fable-5-2", p(8.0, 10.0, 16.0, 0.20, 40.0), None),
    ("claude-fable-5-1", p(10.0, 12.5, 20.0, 0.25, 50.0), None),
    ("claude-fable-5", p(10.0, 12.5, 20.0, 1.0, 50.0), None),
    ("claude-opus-5-5", p(4.0, 5.0, 8.0, 0.20, 20.0), None),
    ("claude-opus-5", p(5.0, 6.25, 10.0, 0.50, 25.0), None),
    ("claude-opus-4", p(5.0, 6.25, 10.0, 0.50, 25.0), None),
    ("claude-sonnet-5-5", p(2.0, 2.5, 4.0, 0.10, 10.0), None),
    ("claude-sonnet-5", p(2.0, 2.5, 4.0, 0.20, 10.0), None),
    ("claude-sonnet-4", p(3.0, 3.75, 6.0, 0.30, 15.0), None),
    (
        "claude-haiku-5-5",
        p(0.10, 0.125, 0.20, 0.01, 0.50),
        Some((100_000, p(0.50, 0.625, 1.0, 0.05, 2.50))),
    ),
    ("claude-haiku-4-5", p(1.0, 1.25, 2.0, 0.10, 5.0), None),
    // OpenAI has no cache-write price: written tokens are plain input there.
    (
        "gpt-6.1-sol",
        p(2.0, 2.0, 2.0, 0.10, 10.0),
        Some((272_000, p(4.0, 4.0, 4.0, 0.20, 15.0))),
    ),
    (
        "gpt-6.1-terra",
        p(1.0, 1.0, 1.0, 0.05, 5.0),
        Some((272_000, p(2.0, 2.0, 2.0, 0.10, 7.5))),
    ),
    (
        "gpt-6-astra",
        p(10.0, 10.0, 10.0, 1.0, 50.0),
        Some((272_000, p(20.0, 20.0, 20.0, 2.0, 75.0))),
    ),
    (
        "gpt-6-sol",
        p(2.0, 2.0, 2.0, 0.20, 10.0),
        Some((272_000, p(4.0, 4.0, 4.0, 0.40, 15.0))),
    ),
    (
        "gpt-6-luna",
        p(0.10, 0.10, 0.10, 0.01, 0.50),
        Some((272_000, p(0.20, 0.20, 0.20, 0.02, 0.75))),
    ),
    ("gpt-5.3-codex", p(1.75, 1.75, 1.75, 0.175, 14.0), None),
];

/// One request's tokens. `input` is uncached input only; reads and writes are the cached parts.
#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Debug)]
struct Tokens {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write_5m: u64,
    cache_write_1h: u64,
    requests: u64,
    /// What the priced requests would cost at list API prices; requests of a model not in `TABLE` are counted apart.
    api_price_usd: f64,
    unpriced_requests: u64,
}

impl Tokens {
    fn add(&mut self, o: &Tokens) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_write_5m += o.cache_write_5m;
        self.cache_write_1h += o.cache_write_1h;
        self.requests += o.requests;
        self.api_price_usd += o.api_price_usd;
        self.unpriced_requests += o.unpriced_requests;
    }
}

fn price(model: &str) -> Option<&'static Row> {
    TABLE
        .iter()
        .filter(|(id, ..)| model.starts_with(id))
        .max_by_key(|(id, ..)| id.len())
}

/// One request, priced: the long-context price applies when the whole prompt (input, reads, writes) passes the threshold.
fn request(model: &str, mut t: Tokens) -> Tokens {
    t.requests = 1;
    let Some((_, base, long)) = price(model) else {
        t.unpriced_requests = 1;
        return t;
    };
    let prompt = t.input + t.cache_read + t.cache_write_5m + t.cache_write_1h;
    let p = match long {
        Some((above, long)) if prompt > *above => long,
        _ => base,
    };
    t.api_price_usd = (t.input as f64 * p.input
        + t.cache_write_5m as f64 * p.write_5m
        + t.cache_write_1h as f64 * p.write_1h
        + t.cache_read as f64 * p.read
        + t.output as f64 * p.output)
        / 1e6;
    t
}

/// What is kept for one session file between runs.
#[derive(Default, Serialize, Deserialize)]
struct FileState {
    /// Bytes read so far (whole lines only), and a hash of the first bytes to notice a rewritten file.
    offset: u64,
    head: String,
    /// Codex: the model of the current turn. Claude Code: the last request ids, as one request is written several times.
    model: String,
    recent: VecDeque<String>,
    /// "date|agent|model" -> tokens.
    rows: BTreeMap<String, Tokens>,
}

#[derive(Default, Serialize, Deserialize)]
struct Index {
    prices: String,
    files: BTreeMap<String, FileState>,
}

const HEAD: u64 = 4096;
const RECENT: usize = 256;

fn head_hash(f: &mut std::fs::File, upto: u64) -> Result<String> {
    let mut buf = vec![0; HEAD.min(upto) as usize];
    f.seek(SeekFrom::Start(0))?;
    f.read_exact(&mut buf)?;
    Ok(footnote_core::hash(&buf))
}

/// Claude Code: assistant lines carry `message.usage`; one request is written once per content block, so dedupe on ids.
fn claude_line(line: &[u8], st: &mut FileState) -> Option<(String, String, Tokens)> {
    if !line.windows(7).any(|w| w == b"\"usage\"") {
        return None;
    }
    let v: Value = serde_json::from_slice(line).ok()?;
    let m = v.get("message")?;
    let (u, model) = (m.get("usage")?, m.get("model")?.as_str()?);
    if v.get("type")?.as_str()? != "assistant" || model.starts_with('<') {
        return None; // `<synthetic>`: messages Claude Code writes itself, not billed
    }
    let key = format!(
        "{}|{}",
        m.get("id")?.as_str()?,
        v["requestId"].as_str().unwrap_or("")
    );
    if st.recent.contains(&key) {
        return None;
    }
    st.recent.push_back(key);
    if st.recent.len() > RECENT {
        st.recent.pop_front();
    }
    let n = |o: &Value, k: &str| o.get(k).and_then(Value::as_u64).unwrap_or(0);
    let write = n(u, "cache_creation_input_tokens");
    let w1h = u
        .get("cache_creation")
        .map_or(0, |c| n(c, "ephemeral_1h_input_tokens"));
    let t = Tokens {
        input: n(u, "input_tokens"),
        output: n(u, "output_tokens"),
        cache_read: n(u, "cache_read_input_tokens"),
        cache_write_1h: w1h,
        cache_write_5m: write.saturating_sub(w1h),
        ..Default::default()
    };
    Some((
        v.get("timestamp")?.as_str()?.get(..10)?.into(),
        model.into(),
        request(model, t),
    ))
}

/// Codex: `turn_context` sets the model; each `token_usage_record` is one response, input including its cached part.
fn codex_line(line: &[u8], st: &mut FileState) -> Option<(String, String, Tokens)> {
    let has = |s: &[u8]| line.windows(s.len()).any(|w| w == s);
    let (turn, record) = (has(b"\"turn_context\""), has(b"\"token_usage_record\""));
    if !turn && !record {
        return None;
    }
    let v: Value = serde_json::from_slice(line).ok()?;
    let kind = v.get("type")?.as_str()?;
    if kind == "turn_context" {
        st.model = v["payload"]["model"].as_str()?.to_string();
        return None;
    }
    if kind != "token_usage_record" {
        return None;
    }
    if st.model.is_empty() {
        st.model = "unknown".into(); // a response logged before its turn's settings: counted, not priced
    }
    let u = &v["payload"]["usage"];
    let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
    let cached = n("cached_input_tokens");
    let t = Tokens {
        input: n("input_tokens").saturating_sub(cached),
        output: n("output_tokens"),
        cache_read: cached,
        cache_write_5m: n("cache_write_input_tokens"),
        ..Default::default()
    };
    Some((
        v.get("timestamp")?.as_str()?.get(..10)?.into(),
        st.model.clone(),
        request(&st.model, t),
    ))
}

type Parse = fn(&[u8], &mut FileState) -> Option<(String, String, Tokens)>;

/// Read what is new in one file. Returns the bytes read.
fn read_file(path: &Path, agent: &str, parse: Parse, st: &mut FileState) -> Result<u64> {
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    if len < st.offset || (st.offset > 0 && head_hash(&mut f, st.offset)? != st.head) {
        *st = FileState::default(); // rewritten or cut: start over
    }
    if len == st.offset {
        return Ok(0);
    }
    f.seek(SeekFrom::Start(st.offset))?;
    let mut buf = Vec::with_capacity((len - st.offset) as usize);
    (&mut f).take(len - st.offset).read_to_end(&mut buf)?;
    let whole = buf.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1); // a line still being written waits
    for line in buf[..whole].split(|&b| b == b'\n') {
        if let Some((date, model, t)) = parse(line, st) {
            st.rows
                .entry(format!("{date}|{agent}|{model}"))
                .or_default()
                .add(&t);
        }
    }
    st.offset += whole as u64;
    if st.offset > 0 {
        st.head = head_hash(&mut f, st.offset)?; // over the same first bytes the next run compares
    }
    Ok(whole as u64)
}

fn sessions(dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "jsonl"))
        .map(|e| e.into_path())
        .collect()
}

/// Where each agent keeps its sessions: Claude Code under `CLAUDE_CONFIG_DIR` (default `~/.claude`) `/projects`, Codex under
/// `CODEX_HOME` (default `~/.codex`) `/sessions`.
fn sources() -> Vec<(&'static str, PathBuf, Parse)> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    let dir = |var: &str, default: &str| {
        std::env::var_os(var)
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(default)))
    };
    let mut out: Vec<(&'static str, PathBuf, Parse)> = Vec::new();
    if let Some(d) = dir("CLAUDE_CONFIG_DIR", ".claude") {
        out.push(("claude-code", d.join("projects"), claude_line));
    }
    if let Some(d) = dir("CODEX_HOME", ".codex") {
        out.push(("codex", d.join("sessions"), codex_line));
    }
    out
}

pub fn usage(days: u16) -> Result<Value> {
    let started = std::time::Instant::now();
    let path = crate::state::state_dir()?.join("usage").join("index.json");
    let mut index: Index = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if index.prices != PRICES {
        index = Index {
            prices: PRICES.into(),
            ..Default::default()
        };
    }
    let (mut files, mut new_bytes) = (0, 0);
    let mut live = std::collections::BTreeSet::new();
    for (agent, dir, parse) in sources() {
        for file in sessions(&dir) {
            let key = file.to_string_lossy().into_owned();
            let st = index.files.entry(key.clone()).or_default();
            match read_file(&file, agent, parse, st) {
                Ok(n) => new_bytes += n,
                Err(_) => continue, // unreadable now (locked, permissions): keep what was counted, try next run
            }
            files += 1;
            live.insert(key);
        }
    }
    // shortcut: a deleted session file drops out of the totals; keep its counts instead if people prune transcripts.
    index.files.retain(|k, _| live.contains(k));
    std::fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(&index)?)?;
    std::fs::rename(&tmp, &path)?;

    let first = (time::OffsetDateTime::now_utc().date()
        - time::Duration::days(i64::from(days) - 1))
    .to_string();
    let mut rows: BTreeMap<(String, String, String), Tokens> = BTreeMap::new();
    for st in index.files.values() {
        for (k, t) in &st.rows {
            let mut p = k.splitn(3, '|');
            let (date, agent, model) = (
                p.next().unwrap_or(""),
                p.next().unwrap_or(""),
                p.next().unwrap_or(""),
            );
            if date >= first.as_str() {
                rows.entry((date.into(), agent.into(), model.into()))
                    .or_default()
                    .add(t);
            }
        }
    }
    let mut totals = Tokens::default();
    let mut by: [HashMap<String, Tokens>; 3] = Default::default();
    for ((date, agent, model), t) in &rows {
        totals.add(t);
        for (i, k) in [date, agent, model].into_iter().enumerate() {
            by[i].entry(k.clone()).or_default().add(t);
        }
    }
    let list = |m: &HashMap<String, Tokens>, name: &str| {
        let mut v: Vec<Value> = m
            .iter()
            .map(|(k, t)| {
                let mut o = json!(t);
                o[name] = json!(k);
                o
            })
            .collect();
        v.sort_by(|a, b| a[name].as_str().cmp(&b[name].as_str()));
        v
    };
    let unpriced: std::collections::BTreeSet<&str> = rows
        .iter()
        .filter(|(_, t)| t.unpriced_requests > 0)
        .map(|((_, _, m), _)| m.as_str())
        .collect();
    Ok(json!({
        "days": days,
        "note": "dates in UTC; counts only, no prompt or answer text is kept; api_price_usd is what the priced requests would cost at list API prices, not what a subscription pays; unpriced_requests are of models without a price yet",
        "prices": PRICES,
        "totals": totals,
        "by_day": list(&by[0], "date"),
        "by_agent": list(&by[1], "agent"),
        "by_model": list(&by[2], "model"),
        "unpriced_models": unpriced,
        "measured_savings": measured_savings(&first),
        "read": {"files": files, "new_bytes": new_bytes, "ms": started.elapsed().as_millis()},
    }))
}

/// Savings measured where the output was filtered, shown apart from the token counts and prices above. Today the only
/// measured source is RTK (output filtering of shell commands): it records the size before and after each filtered command, and
/// what it removes never reaches the agent, so it is not in the session counts above (no double counting).
fn measured_savings(first: &str) -> Value {
    let not_measured = json!([
        "Footnote's own lean results: sizes before and after are not recorded yet",
        "behaviour skills such as ponytail: a saving exists only against a benchmark without them, so it is an estimate, not shown here"
    ]);
    let rtk = run_rtk().map(|v| rtk_days(&v, first));
    match rtk {
        Ok(days) => {
            json!({"source": "rtk gain --daily (RTK's own count of tokens before and after its filters)",
                           "note": "tokens the agent never had to read; dates as RTK records them (local time); not priced, because a trimmed result would also have been re-read at cache rates on later turns",
                           "rtk": days, "not_measured": not_measured})
        }
        Err(e) => {
            json!({"source": "rtk", "available": false, "reason": format!("{e:#}"), "not_measured": not_measured})
        }
    }
}

fn run_rtk() -> Result<Value> {
    use wait_timeout::ChildExt;
    let mut cmd = std::process::Command::new("rtk");
    cmd.args(["gain", "--daily", "--format", "json"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x08000000); // CREATE_NO_WINDOW
    let mut child = cmd
        .spawn()
        .map_err(|_| anyhow::anyhow!("RTK is not installed (rtk not on PATH)"))?;
    let mut out = Vec::new();
    let mut stdout = child.stdout.take().expect("piped");
    let reader =
        std::thread::spawn(move || std::io::Read::read_to_end(&mut stdout, &mut out).map(|_| out));
    if child
        .wait_timeout(std::time::Duration::from_secs(5))?
        .is_none()
    {
        let _ = child.kill();
        anyhow::bail!("rtk gain did not answer within 5 s");
    }
    let out = reader
        .join()
        .map_err(|_| anyhow::anyhow!("rtk output reader failed"))??;
    serde_json::from_slice(&out).map_err(|_| {
        anyhow::anyhow!("rtk gain gave no JSON (an RTK version without --format json?)")
    })
}

/// RTK's daily rows from `first` on, plus their totals.
fn rtk_days(v: &Value, first: &str) -> Value {
    let days: Vec<Value> = v["daily"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["date"].as_str().is_some_and(|date| date >= first))
        .map(|d| {
            json!({"date": d["date"], "commands": d["commands"], "before_tokens": d["input_tokens"],
                        "after_tokens": d["output_tokens"], "saved_tokens": d["saved_tokens"]})
        })
        .collect();
    let sum = |k: &str| days.iter().map(|d| d[k].as_u64().unwrap_or(0)).sum::<u64>();
    let (before, saved) = (sum("before_tokens"), sum("saved_tokens"));
    json!({"by_day": days, "commands": sum("commands"), "before_tokens": before, "after_tokens": sum("after_tokens"),
           "saved_tokens": saved, "saved_share": if before > 0 { saved as f64 / before as f64 } else { 0.0 }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtk_savings_keep_the_window_and_sum_it() {
        let v = json!({"daily": [
            {"date": "2026-11-10", "commands": 5, "input_tokens": 100, "output_tokens": 90, "saved_tokens": 10},
            {"date": "2026-11-16", "commands": 2, "input_tokens": 400, "output_tokens": 100, "saved_tokens": 300},
            {"date": "2026-11-17", "commands": 1, "input_tokens": 100, "output_tokens": 0, "saved_tokens": 100}]});
        let r = rtk_days(&v, "2026-11-16");
        assert_eq!(r["by_day"].as_array().unwrap().len(), 2);
        assert_eq!(
            (
                r["commands"].as_u64(),
                r["saved_tokens"].as_u64(),
                r["before_tokens"].as_u64()
            ),
            (Some(3), Some(400), Some(500))
        );
        assert_eq!(r["saved_share"], 0.8);
        assert_eq!(rtk_days(&json!({}), "2026-11-16")["saved_tokens"], 0);
    }

    #[test]
    fn prices_pick_the_longest_prefix_and_the_long_context_tier() {
        assert_eq!(price("claude-sonnet-5-5").unwrap().0, "claude-sonnet-5-5");
        assert_eq!(price("claude-sonnet-5").unwrap().0, "claude-sonnet-5");
        assert_eq!(
            price("claude-sonnet-4-5-20250929").unwrap().0,
            "claude-sonnet-4"
        );
        assert!(price("some-new-model").is_none());
        let t = |input| Tokens {
            input,
            output: 0,
            ..Default::default()
        };
        // Haiku 5.5: $0.10 per million up to 100k prompt tokens, $0.50 above.
        let usd = |m: &str, n| request(m, t(n)).api_price_usd;
        assert!((usd("claude-haiku-5-5", 100_000) - 0.01).abs() < 1e-9);
        assert!((usd("claude-haiku-5-5", 200_000) - 0.10).abs() < 1e-9);
        assert_eq!(request("some-new-model", t(1)).unpriced_requests, 1);
    }
}
