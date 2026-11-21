//! `footnote usage`: tokens per day, agent and model, read from the agents' own session files on this computer (Claude Code
//! and Codex). Only counts are kept: no prompt or answer text is copied. Files are read incrementally: the new bytes since the
//! last run, with a guard that starts a file over when it was rewritten. Design: docs/roadmap/BACKLOG.md, "Spend and savings".
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// List prices in USD per million tokens, from the providers' pricing pages, read 16 Nov 2026:
/// https://platform.claude.com/docs/en/about-claude/pricing and https://developers.openai.com/api/docs/pricing (standard tier);
/// gpt-6.2-sol read 18 Nov 2026 from the same OpenAI page (#223);
/// https://api-docs.deepseek.com/quick_start/pricing read 20 Nov 2026 (standard/peak list rates; off-peak 50% discount not modelled).
/// Changing a price: edit the row and `PRICES`; every file is then read again so old counts get the new prices.
/// shortcut: batch, fast tier, US-only inference (x1.1), off-peak discounts and regional endpoints are not priced; add them when someone uses them.
const PRICES: &str = "2026-11-20";

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
        "gpt-6.2-sol",
        p(2.0, 2.0, 2.0, 0.10, 10.0),
        Some((272_000, p(4.0, 4.0, 4.0, 0.20, 15.0))),
    ),
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
    // DeepSeek standard/peak list rates (https://api-docs.deepseek.com/quick_start/pricing, 20 Nov 2026; off-peak 50% discount not modelled).
    ("deepseek-flash", p(0.30, 0.30, 0.30, 0.006, 1.20), None),
    ("deepseek-v4-pro", p(1.32, 1.32, 1.32, 0.044, 3.96), None),
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
    /// `api_price_usd` split by bucket (#238): cache traffic, not output, is most of an agent's bill.
    cost_cache_read: f64,
    cost_cache_write: f64,
    cost_input: f64,
    cost_output: f64,
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
        self.cost_cache_read += o.cost_cache_read;
        self.cost_cache_write += o.cost_cache_write;
        self.cost_input += o.cost_input;
        self.cost_output += o.cost_output;
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
    t.cost_input = t.input as f64 * p.input / 1e6;
    t.cost_cache_write =
        (t.cache_write_5m as f64 * p.write_5m + t.cache_write_1h as f64 * p.write_1h) / 1e6;
    t.cost_cache_read = t.cache_read as f64 * p.read / 1e6;
    t.cost_output = t.output as f64 * p.output / 1e6;
    t.api_price_usd = t.cost_input + t.cost_cache_write + t.cost_cache_read + t.cost_output;
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
    /// "agent|window" -> the newest limit seen in this file.
    #[serde(default)]
    limits: BTreeMap<String, Limit>,
}

/// A usage limit as the agent wrote it: when it was seen, the share used (Claude Code writes a limit only once hit: 100) and
/// when the window resets, in Unix seconds.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
struct Limit {
    seen: i64,
    used_percent: f64,
    resets_at: i64,
}

impl FileState {
    fn limit(&mut self, agent: &str, window: &str, l: Limit) {
        let e = self.limits.entry(format!("{agent}|{window}")).or_insert(l);
        if l.seen >= e.seen {
            *e = l;
        }
    }
}

/// Unix seconds of an RFC 3339 timestamp.
fn unix(ts: &str) -> Option<i64> {
    time::OffsetDateTime::parse(ts, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|t| t.unix_timestamp())
}

/// Bumped when FileState gains what old indexes lack, so every file is read again once.
const FORMAT: u32 = 3; // 3: per-bucket costs, so older indexes are recounted

#[derive(Default, Serialize, Deserialize)]
struct Index {
    prices: String,
    #[serde(default)]
    format: u32,
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
    if line.windows(17).any(|w| w == b"\"rate_limit_info\"") {
        // A refused request: Claude Code writes the limit that was hit and when it resets, never the share used before.
        let v: Value = serde_json::from_slice(line).ok()?;
        let r = &v["apiErrorParams"]["rate_limit_info"];
        let window = match r["rateLimitType"].as_str()? {
            "five_hour" => "5h",
            "seven_day" => "weekly",
            other => other,
        };
        let l = Limit {
            seen: unix(v["timestamp"].as_str()?)?,
            used_percent: 100.0,
            resets_at: r["resetsAt"].as_i64()?,
        };
        st.limit("claude-code", window, l);
        return None;
    }
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

/// DeepSeek Harness: each `assistant/message` line carries its reply's `usage` (input excludes cache reads) and the model in
/// `message.source.model`; `time` is in milliseconds.
fn dsh_line(line: &[u8], _: &mut FileState) -> Option<(String, String, Tokens)> {
    if !line.windows(7).any(|w| w == b"\"usage\"") {
        return None;
    }
    let v: Value = serde_json::from_slice(line).ok()?;
    if v.get("type")?.as_str()? != "assistant/message" {
        return None;
    }
    let d = v.get("data")?;
    let u = d.get("usage")?;
    let model = d["message"]["source"]["model"]
        .as_str()
        .unwrap_or("deepseek");
    let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
    let t = Tokens {
        input: n("inputTokens"),
        output: n("outputTokens"),
        cache_read: n("cacheReadTokens"),
        cache_write_5m: n("cacheWriteTokens"),
        ..Default::default()
    };
    let date = time::OffsetDateTime::from_unix_timestamp(v.get("time")?.as_i64()? / 1000)
        .ok()?
        .date()
        .to_string();
    Some((date, model.into(), request(model, t)))
}

/// Codex: `turn_context` sets the model; each `token_usage_record` is one response, input including its cached part.
fn codex_line(line: &[u8], st: &mut FileState) -> Option<(String, String, Tokens)> {
    let has = |s: &[u8]| line.windows(s.len()).any(|w| w == s);
    let (turn, record) = (has(b"\"turn_context\""), has(b"\"token_usage_record\""));
    if !turn && !record {
        if has(b"\"rate_limits\"") {
            codex_limits(line, st);
        }
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

/// Codex writes its limits with every token count: the share used of each window, its length and its reset time.
fn codex_limits(line: &[u8], st: &mut FileState) -> Option<()> {
    let v: Value = serde_json::from_slice(line).ok()?;
    let seen = unix(v["timestamp"].as_str()?)?;
    let r = &v["payload"]["rate_limits"];
    for w in [&r["primary"], &r["secondary"]] {
        let (Some(used), Some(minutes), Some(resets_at)) = (
            w["used_percent"].as_f64(),
            w["window_minutes"].as_i64(),
            w["resets_at"].as_i64(),
        ) else {
            continue;
        };
        let window = match minutes {
            300 => "5h".to_string(),
            10080 => "weekly".to_string(),
            m => format!("{m}m"),
        };
        let l = Limit {
            seen,
            used_percent: used,
            resets_at,
        };
        st.limit("codex", &window, l);
    }
    Some(())
}

type Parse = fn(&[u8], &mut FileState) -> Option<(String, String, Tokens)>;

/// Read what is new in one file. Returns the bytes read.
fn read_file(path: &Path, agent: &str, parse: Parse, st: &mut FileState) -> Result<u64> {
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    if path.extension().is_some_and(|x| x == "zstd") {
        // Compressed sessions cannot be read from where the last run stopped: a changed file is counted again in full.
        if len == st.offset && st.offset > 0 && head_hash(&mut f, st.offset)? == st.head {
            return Ok(0);
        }
        *st = FileState::default();
        f.seek(SeekFrom::Start(0))?;
        let data = zstd::decode_all((&mut f).take(len))?;
        for line in data.split(|&b| b == b'\n') {
            if let Some((date, model, t)) = parse(line, st) {
                st.rows
                    .entry(format!("{date}|{agent}|{model}"))
                    .or_default()
                    .add(&t);
            }
        }
        st.offset = len;
        st.head = head_hash(&mut f, len)?;
        return Ok(len);
    }
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

/// Price one session file (#223), Claude Code or Codex, found by which reader recognises it: tokens per category, list
/// price, priced and unpriced requests, per model. The benchmark collector calls this instead of copying the price table.
pub fn session(path: &Path) -> Result<Value> {
    let mut best: Option<(&str, Parse, FileState)> = None;
    for (agent, parse) in [("claude-code", claude_line as Parse), ("codex", codex_line)] {
        let mut st = FileState::default();
        read_file(path, agent, parse, &mut st)?;
        if !st.rows.is_empty() {
            best = Some((agent, parse, st));
            break;
        }
    }
    let (agent, parse, st) = best.context("no Claude Code or Codex token counts in this file")?;
    let (mut totals, mut by_model) = (Tokens::default(), BTreeMap::<String, Tokens>::new());
    for (key, t) in &st.rows {
        totals.add(t);
        by_model
            .entry(key.rsplit('|').next().unwrap_or_default().to_string())
            .or_default()
            .add(t);
    }
    let by_model: Vec<Value> = by_model
        .into_iter()
        .map(|(model, t)| {
            let mut v = serde_json::to_value(t).unwrap_or_default();
            v["model"] = json!(model);
            v
        })
        .collect();
    let (fixed, floor) = fixed_overhead(path, parse, totals.requests, totals.cache_read);
    Ok(
        json!({"file": path.display().to_string(), "agent": agent, "prices": PRICES, "totals": totals, "by_model": by_model,
            "fixed_overhead": {"first_prompt_tokens": fixed, "share_of_cache_reads_at_least": floor,
                "note": "the first request's whole prompt: system prompt, tool schemas, CLAUDE.md, skills, hook text and the first message; every later turn reads it again. A resumed session starts larger, so this is only a clean measure on a fresh one."}}),
    )
}

/// The first request's prompt size, and the share of all cache reads that re-reading it every later turn accounts for at
/// least (#238). `None` for a compressed file: shortcut, the readers cannot be asked for one request there, add it if needed.
fn fixed_overhead(
    path: &Path,
    parse: Parse,
    requests: u64,
    cache_read: u64,
) -> (Option<u64>, Option<f64>) {
    let Ok(data) = std::fs::read(path) else {
        return (None, None);
    };
    let mut st = FileState::default();
    let first = data.split(|&b| b == b'\n').find_map(|l| parse(l, &mut st));
    let Some((_, _, t)) = first else {
        return (None, None);
    };
    let prompt = t.input + t.cache_read + t.cache_write_5m + t.cache_write_1h;
    let floor = (cache_read > 0)
        .then(|| ((prompt * requests.saturating_sub(1)) as f64 / cache_read as f64).min(1.0));
    (Some(prompt), floor.map(|f| (f * 1000.0).round() / 1000.0))
}

fn sessions(dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            e.file_type().is_file() && (name.ends_with(".jsonl") || name.ends_with(".jsonl.zstd"))
        })
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
        // Archiving moves a session here; without it the session drops out of the totals (#238 audit: 83 responses in 7 days).
        out.push(("codex", d.join("archived_sessions"), codex_line));
    }
    if let Some(d) = dir("DSH_HOME", ".dsh") {
        out.push(("deepseek-harness", d.join("sessions"), dsh_line));
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
    if index.prices != PRICES || index.format != FORMAT {
        index = Index {
            prices: PRICES.into(),
            format: FORMAT,
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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
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
        "agents_not_measured": [
            "grok: keeps no token counts on this computer (only a general log)",
            "gemini: no session folder found on this computer"
        ],
        "note": "dates in UTC; counts only, no prompt or answer text is kept; api_price_usd is what the priced requests would cost at list API prices, not what a subscription pays, and cost_cache_read, cost_cache_write, cost_input and cost_output split it by bucket; unpriced_requests are of models without a price yet",
        "prices": PRICES,
        "totals": totals,
        "by_day": list(&by[0], "date"),
        "by_agent": list(&by[1], "agent"),
        "by_model": list(&by[2], "model"),
        "unpriced_models": unpriced,
        "limits": limits(&index, time::OffsetDateTime::now_utc().unix_timestamp()),
        "measured_savings": measured_savings(&first),
        "read": {"files": files, "new_bytes": new_bytes, "ms": started.elapsed().as_millis()},
    }))
}

/// The newest limit per agent and window across all files, for windows that have not reset yet (a past reset means the
/// share used is no longer known). Read from local session files only; no account endpoint is called.
fn limits(index: &Index, now: i64) -> Value {
    let mut newest: BTreeMap<&str, Limit> = BTreeMap::new();
    for (k, l) in index.files.values().flat_map(|st| &st.limits) {
        let e = newest.entry(k).or_insert(*l);
        if l.seen >= e.seen {
            *e = *l;
        }
    }
    let current: Vec<Value> = newest
        .into_iter()
        .filter(|(_, l)| l.resets_at > now)
        .map(|(k, l)| {
            let (agent, window) = k.split_once('|').unwrap_or((k, ""));
            json!({"agent": agent, "window": window, "used_percent": l.used_percent, "resets_at": l.resets_at,
                   "resets_in_minutes": (l.resets_at - now + 59) / 60, "seen_at": l.seen})
        })
        .collect();
    json!({"current": current,
           "note": "from the agents' session files only: Codex writes the share used on every turn; Claude Code writes a limit only when one is hit (100 %), so it shows here only until that limit resets; times in Unix seconds"})
}

/// Savings measured directly where Footnote filtered shell output (`footnote run`) or trimmed MCP search results,
/// read from `<state>/usage/lean.jsonl` (+ `.old`). Never shells out to external tools (#351).
fn measured_savings(first: &str) -> Value {
    let not_measured = json!([
        "behaviour skills: a saving exists only against a benchmark without them, so it is an estimate, not shown here"
    ]);
    let ledger = crate::state::state_dir()
        .map(|s| lean_ledger(&s, first))
        .unwrap_or_else(|_| lean_ledger_from_lines(&[], first));
    json!({
        "source": "Footnote Lean (usage/lean.jsonl: shell output filters and trimmed search results)",
        "note": "bytes measured directly before and after filtering; tokens estimated as bytes / 4; not priced, because trimmed output would also have been re-read at cache rates on later turns",
        "shell": ledger["shell"],
        "footnote": ledger["footnote"],
        "total_saved_bytes": ledger["total_saved_bytes"],
        "total_saved_tokens_estimate": ledger["total_saved_tokens_estimate"],
        "not_measured": not_measured,
    })
}

/// `footnote gain [--days N] [--attrib]`: Footnote Lean savings ledger (shell output filters, handle re-retrieval
/// miss rate, trimmed MCP search results) and optional token/cache-bucket attribution from local session transcripts (#351).
pub fn gain(days: u16, attrib: bool) -> Result<Value> {
    let state = crate::state::state_dir()?;
    let first = (time::OffsetDateTime::now_utc().date()
        - time::Duration::days(i64::from(days) - 1))
    .to_string();
    let ledger = lean_ledger(&state, &first);
    let mut out = json!({
        "days": days,
        "from": first,
        "source": "Footnote Lean (usage/lean.jsonl)",
        "shell": ledger["shell"],
        "footnote": ledger["footnote"],
        "total_saved_bytes": ledger["total_saved_bytes"],
        "total_saved_tokens_estimate": ledger["total_saved_tokens_estimate"],
    });
    if attrib {
        let u = usage(days)?;
        let t = &u["totals"];
        let input = t["input"].as_u64().unwrap_or(0);
        let cache_read = t["cache_read"].as_u64().unwrap_or(0);
        let cache_write =
            t["cache_write_5m"].as_u64().unwrap_or(0) + t["cache_write_1h"].as_u64().unwrap_or(0);
        let prompt_total = input + cache_read + cache_write;
        let cache_hit_share = if prompt_total > 0 {
            ((cache_read as f64 / prompt_total as f64) * 1000.0).round() / 1000.0
        } else {
            0.0
        };
        out["attribution"] = json!({
            "prices": u["prices"],
            "totals": u["totals"],
            "cache_hit_share": cache_hit_share,
            "by_agent": u["by_agent"],
            "by_model": u["by_model"],
            "by_day": u["by_day"],
        });
    }
    Ok(out)
}

fn lean_ledger(state: &Path, first: &str) -> Value {
    let dir = state.join("usage");
    let mut lines = Vec::new();
    for file in ["lean.jsonl.old", "lean.jsonl"] {
        if let Ok(text) = std::fs::read_to_string(dir.join(file)) {
            lines.extend(text.lines().map(String::from));
        }
    }
    lean_ledger_from_lines(&lines, first)
}

fn lean_ledger_from_lines(lines: &[String], first: &str) -> Value {
    // MCP search trimming: date -> (calls, before, after)
    let mut mcp_by_day: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    // Shell (`footnote run`): date -> (commands, before, after, stored, retrieved)
    let mut sh_by_day: BTreeMap<String, (u64, u64, u64, u64, u64)> = BTreeMap::new();
    // Shell per command family: family -> (commands, before, after)
    let mut sh_by_family: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();

    for raw in lines {
        let Ok(v) = serde_json::from_str::<Value>(raw) else {
            continue;
        };
        let date = v["t"]
            .as_i64()
            .and_then(|ms| time::OffsetDateTime::from_unix_timestamp(ms / 1000).ok())
            .map(|t| t.date().to_string());
        let Some(date) = date.filter(|d| d.as_str() >= first) else {
            continue;
        };
        match v["tool"].as_str().unwrap_or("") {
            "run" => {
                let before = v["before"].as_u64().unwrap_or(0);
                let after = v["after"].as_u64().unwrap_or(0);
                let stored = u64::from(v["stored"].as_bool().unwrap_or(false));
                let e = sh_by_day.entry(date).or_default();
                e.0 += 1;
                e.1 += before;
                e.2 += after;
                e.3 += stored;
                let fam = v["family"].as_str().unwrap_or("generic").to_string();
                let f = sh_by_family.entry(fam).or_default();
                f.0 += 1;
                f.1 += before;
                f.2 += after;
            }
            "output" => {
                if v["miss"].as_bool().unwrap_or(true) {
                    let e = sh_by_day.entry(date).or_default();
                    e.4 += 1;
                }
            }
            _ => {
                let e = mcp_by_day.entry(date).or_default();
                e.0 += 1;
                e.1 += v["before"].as_u64().unwrap_or(0);
                e.2 += v["after"].as_u64().unwrap_or(0);
            }
        }
    }

    let (sh_cmds, sh_before, sh_after, sh_stored, sh_retrieved) = sh_by_day
        .values()
        .fold((0u64, 0u64, 0u64, 0u64, 0u64), |a, d| {
            (a.0 + d.0, a.1 + d.1, a.2 + d.2, a.3 + d.3, a.4 + d.4)
        });
    let sh_saved = sh_before.saturating_sub(sh_after);
    let sh_share = if sh_before > 0 {
        ((sh_saved as f64 / sh_before as f64) * 1000.0).round() / 1000.0
    } else {
        0.0
    };
    let miss_rate = if sh_stored > 0 {
        ((sh_retrieved as f64 / sh_stored as f64) * 1000.0).round() / 1000.0
    } else {
        0.0
    };

    let (mcp_calls, mcp_before, mcp_after) = mcp_by_day
        .values()
        .fold((0u64, 0u64, 0u64), |a, d| (a.0 + d.0, a.1 + d.1, a.2 + d.2));
    let mcp_saved = mcp_before.saturating_sub(mcp_after);
    let total_saved = sh_saved + mcp_saved;

    let by_family: Vec<Value> = sh_by_family
        .into_iter()
        .map(|(family, (commands, before, after))| {
            let saved = before.saturating_sub(after);
            let share = if before > 0 {
                ((saved as f64 / before as f64) * 1000.0).round() / 1000.0
            } else {
                0.0
            };
            json!({
                "family": family,
                "commands": commands,
                "before_bytes": before,
                "after_bytes": after,
                "saved_bytes": saved,
                "saved_tokens_estimate": saved / 4,
                "saved_share": share,
            })
        })
        .collect();

    let shell_days: Vec<Value> = sh_by_day
        .into_iter()
        .map(|(d, (commands, before, after, stored, retrieved))| {
            json!({
                "date": d,
                "commands": commands,
                "before_bytes": before,
                "after_bytes": after,
                "saved_bytes": before.saturating_sub(after),
                "handles_stored": stored,
                "handles_retrieved": retrieved,
            })
        })
        .collect();

    let mcp_days: Vec<Value> = mcp_by_day
        .into_iter()
        .map(|(d, (c, b, a))| json!({"date": d, "calls": c, "before_bytes": b, "after_bytes": a}))
        .collect();

    json!({
        "shell": {
            "source": "Footnote Lean shell output filters (`footnote run`, usage/lean.jsonl)",
            "commands": sh_cmds,
            "before_bytes": sh_before,
            "after_bytes": sh_after,
            "saved_bytes": sh_saved,
            "saved_tokens_estimate": sh_saved / 4,
            "saved_share": sh_share,
            "handles_stored": sh_stored,
            "handles_retrieved": sh_retrieved,
            "miss_rate": miss_rate,
            "by_family": by_family,
            "by_day": shell_days,
        },
        "footnote": {
            "source": "Footnote's lean search results, measured on each agent call (usage.lean_log)",
            "calls": mcp_calls,
            "before_bytes": mcp_before,
            "after_bytes": mcp_after,
            "saved_bytes": mcp_saved,
            "saved_tokens_estimate": mcp_saved / 4,
            "by_day": mcp_days,
        },
        "total_saved_bytes": total_saved,
        "total_saved_tokens_estimate": total_saved / 4,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_harness_replies_are_counted_with_their_cache_reads_apart() {
        let line = br#"{"type":"assistant/message","time":1792964444792,"data":{"message":{"source":{"model":"deepseek-flash"}},"usage":{"cacheReadTokens":8576,"cacheWriteTokens":0,"inputTokens":4081,"outputTokens":136,"totalTokens":12793}}}"#;
        let (date, model, t) = dsh_line(line, &mut FileState::default()).unwrap();
        assert_eq!(
            (date.as_str(), model.as_str()),
            ("2026-10-25", "deepseek-flash")
        );
        assert_eq!(
            (
                t.input,
                t.output,
                t.cache_read,
                t.requests,
                t.unpriced_requests
            ),
            (4081, 136, 8576, 1, 0)
        );
        let approx = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(approx(t.cost_input, 4081.0 * 0.30 / 1e6));
        assert!(approx(t.cost_cache_read, 8576.0 * 0.006 / 1e6));
        assert!(approx(t.cost_output, 136.0 * 1.20 / 1e6));
        assert!(approx(
            t.api_price_usd,
            (4081.0 * 0.30 + 8576.0 * 0.006 + 136.0 * 1.20) / 1e6
        ));
        let user =
            br#"{"type":"user/message","time":1792964444792,"data":{"usage":{"inputTokens":1}}}"#;
        assert!(dsh_line(user, &mut FileState::default()).is_none());
    }

    #[test]
    fn limits_keep_the_newest_per_window_and_drop_reset_ones() {
        let mut st = FileState::default();
        let codex = |t: &str, used: f64| {
            format!(
                r#"{{"timestamp":"{t}","type":"event_msg","payload":{{"type":"token_count","rate_limits":{{"primary":{{"used_percent":{used},"window_minutes":300,"resets_at":2000}},"secondary":{{"used_percent":85.0,"window_minutes":10080,"resets_at":900}}}}}}}}"#
            )
        };
        // Newer line first: the older one must not overwrite it.
        assert!(codex_line(codex("1970-01-01T00:10:00Z", 8.0).as_bytes(), &mut st).is_none());
        assert!(codex_line(codex("1970-01-01T00:05:00Z", 3.0).as_bytes(), &mut st).is_none());
        let claude = br#"{"type":"assistant","timestamp":"1970-01-01T00:01:40Z","apiError":"usage_limit_reached","apiErrorParams":{"rate_limit_info":{"status":"rejected","resetsAt":1500,"rateLimitType":"five_hour"}}}"#;
        assert!(claude_line(claude, &mut st).is_none());
        let index = Index {
            files: BTreeMap::from([("a".into(), st)]),
            ..Default::default()
        };
        let current = limits(&index, 1000)["current"].as_array().unwrap().clone();
        // The weekly Codex window reset at 900, before now (1000): dropped.
        let got: Vec<_> = current
            .iter()
            .map(|l| {
                (
                    l["agent"].as_str().unwrap(),
                    l["window"].as_str().unwrap(),
                    l["used_percent"].as_f64().unwrap(),
                )
            })
            .collect();
        assert_eq!(got, [("claude-code", "5h", 100.0), ("codex", "5h", 8.0)]);
        assert_eq!(current[1]["resets_in_minutes"], 17);
    }

    #[test]
    fn lean_savings_separate_shell_mcp_and_track_retrieve_misses() {
        // 2026-11-10T00:00:00Z = 1794268800_000, 2026-11-16T12:00:00Z = 1794830400_000
        let lines = vec![
            json!({"t": 1_794_268_800_000_i64, "tool": "run", "family": "git", "before": 1000, "after": 200, "stored": true}).to_string(),
            json!({"t": 1_794_830_400_000_i64, "tool": "run", "family": "cargo-test", "before": 4000, "after": 800, "stored": true}).to_string(),
            json!({"t": 1_794_830_405_000_i64, "tool": "run", "family": "search", "before": 1000, "after": 800, "stored": true}).to_string(),
            json!({"t": 1_794_830_410_000_i64, "tool": "output", "handle": "h-abc", "miss": true, "before": 0, "after": 120}).to_string(),
            json!({"t": 1_794_830_420_000_i64, "tool": "footnote_recall", "before": 2000, "after": 1200}).to_string(),
        ];
        let r = lean_ledger_from_lines(&lines, "2026-11-16");
        assert_eq!(r["shell"]["commands"], 2);
        assert_eq!(r["shell"]["before_bytes"], 5000);
        assert_eq!(r["shell"]["after_bytes"], 1600);
        assert_eq!(r["shell"]["saved_bytes"], 3400);
        assert_eq!(r["shell"]["handles_stored"], 2);
        assert_eq!(r["shell"]["handles_retrieved"], 1);
        assert_eq!(r["shell"]["miss_rate"], 0.5);
        assert_eq!(r["footnote"]["calls"], 1);
        assert_eq!(r["footnote"]["saved_bytes"], 800);
        assert_eq!(r["total_saved_bytes"], 4200);
        assert_eq!(r["total_saved_tokens_estimate"], 1050);
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

    #[test]
    fn fixed_overhead_is_the_first_prompt_and_its_share_of_cache_reads() {
        let line = |id: &str, read: u64, write: u64| {
            format!(
                r#"{{"type":"assistant","timestamp":"2026-11-19T10:00:00Z","requestId":"{id}","message":{{"id":"{id}","model":"claude-sonnet-5-5","usage":{{"input_tokens":5,"output_tokens":1,"cache_read_input_tokens":{read},"cache_creation_input_tokens":{write}}}}}}}"#
            )
        };
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("s.jsonl");
        // First prompt 1,000 (5 + 0 + 995); two later turns each re-read 1,000 and 1,100.
        std::fs::write(
            &f,
            [line("a", 0, 995), line("b", 1000, 100), line("c", 1100, 50)]
                .map(|l| l + "\n")
                .concat(),
        )
        .unwrap();
        let v = session(&f).unwrap();
        assert_eq!(v["fixed_overhead"]["first_prompt_tokens"], 1000);
        // 1000 x 2 later requests / 2100 cache reads.
        assert_eq!(v["fixed_overhead"]["share_of_cache_reads_at_least"], 0.952);
    }
}
