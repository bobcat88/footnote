//! Product options: the catalogue (what each option gains and costs), the user's settings and the company policy.
//!
//! Settings live per product in the state folder (`memory.json` for Footnote Memory). A machine-wide policy file forces options
//! and wins over the user's settings: `%ProgramData%\footnote\policy.json` on Windows, `/etc/footnote/policy.json` elsewhere.
//! Policy shape: `{"options": {"memory.resident": {"value": false, "reason": "no resident processes on managed laptops"}}}`.
use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

pub struct OptionSpec {
    pub key: &'static str,
    pub default: Value,
    /// Inclusive range for integer options; `None` for booleans.
    pub range: Option<(u64, u64)>,
    pub gain: &'static str,
    pub cost: &'static str,
}

pub fn catalogue() -> Vec<OptionSpec> {
    vec![
        OptionSpec {
            key: "memory.enabled",
            default: json!(true),
            range: None,
            gain: "Agents receive your memories and are asked to save new ones",
            cost: "Off: the hook does nothing at once, no agent restart; `footnote uninstall --memory` removes it for good",
        },
        OptionSpec {
            key: "library.enabled",
            default: json!(true),
            range: None,
            gain: "Agents can search your indexed document corpora through Footnote's tools",
            cost: "Off: corpus tools leave agents' tool lists and calls are refused at once; your indexes are kept",
        },
        OptionSpec {
            key: "code.enabled",
            default: json!(true),
            range: None,
            gain: "Agents can look up definitions, callers and callees with footnote_code",
            cost: "Off: footnote_code leaves agents' tool lists and calls are refused at once; your code graphs are kept",
        },
        OptionSpec {
            key: "memory.resident",
            default: json!(true),
            range: None,
            gain: "Recall by meaning on each prompt, not only by keyword",
            cost: "A background process keeps the models in RAM until idle_seconds pass; about 450 MB with the default pair; some companies forbid resident processes. Recall@3 on real memories 0.91 against keyword 0.77, +13 ms per prompt at p95 (MEMORY-RECALL-SWITCH.md)",
        },
        OptionSpec {
            key: "memory.infer_tags",
            default: json!(false),
            range: None,
            gain: "Recall ranks only memories sharing a tag named in the prompt (plus untagged ones): fewer unrelated candidates",
            cost: "A memory tagged without the word the prompt uses is left out; off until bench/experiments/MEMORY-TAGS.md passes",
        },
        OptionSpec {
            key: "memory.idle_seconds",
            default: json!(1800),
            range: Some((60, 86_400)),
            gain: "Longer: fewer model reloads (about a second each)",
            cost: "Longer: the models stay in RAM longer",
        },
        OptionSpec {
            key: "memory.every_turns",
            default: json!(10),
            range: Some((1, 1000)),
            gain: "Lower: more is remembered",
            cost: "Lower: more agent tokens spent on save requests",
        },
        OptionSpec {
            key: "memory.recall_limit",
            default: json!(4),
            range: Some((1, 20)),
            gain: "Higher: more memories offered per prompt",
            cost: "Higher: more tokens per prompt and more noise",
        },
        OptionSpec {
            key: "memory.handoff_tokens",
            default: json!(200_000),
            range: Some((0, 2_000_000)),
            gain: "Lower: shorter sessions; every call re-reads the whole context, so a long one costs most (0: never ask)",
            cost: "Lower: more often asked to save a handoff and clear the session",
        },
        OptionSpec {
            key: "memory.activity",
            default: json!(true),
            range: None,
            gain: "The Activity screen shows which memories reached your agents and when",
            cost: "A local log of memory ids, projects and times (memory-hook/activity.jsonl, capped at 512 KB); never what you asked",
        },
        OptionSpec {
            key: "usage.lean_log",
            default: json!(true),
            range: None,
            gain: "The Usage view shows what Footnote's trimmed search results saved, measured on each call",
            cost: "One line per agent search in usage/lean.jsonl in the state folder (time, tool, two sizes; never text), capped at about 1 MB",
        },
        OptionSpec {
            key: "memory.metrics",
            default: json!(false),
            range: None,
            gain: "Timings of each hook step, for reviews and support",
            cost: "A local file, metrics.jsonl in the state folder; nothing leaves the machine",
        },
    ]
}

fn spec(key: &str) -> Result<OptionSpec> {
    catalogue()
        .into_iter()
        .find(|o| o.key == key)
        .with_context(|| format!("unknown option `{key}`; `footnote options` lists them"))
}

fn settings_file(state: &Path, product: &str) -> PathBuf {
    state.join(format!("{product}.json"))
}

pub fn policy_path() -> Option<PathBuf> {
    // Tests point elsewhere; an installed (release) build never lets the user move the company policy away.
    #[cfg(debug_assertions)]
    if let Some(p) = std::env::var_os("FOOTNOTE_POLICY_FILE") {
        return Some(PathBuf::from(p));
    }
    if cfg!(windows) {
        std::env::var_os("ProgramData").map(|d| PathBuf::from(d).join("footnote/policy.json"))
    } else {
        Some(PathBuf::from("/etc/footnote/policy.json"))
    }
}

/// Options forced by the company: key -> {value, reason}. No file: nothing forced. An unreadable file is an error, so a broken
/// policy never silently unlocks everything.
pub fn policy() -> Result<Map<String, Value>> {
    let Some(path) = policy_path().filter(|p| p.exists()) else {
        return Ok(Map::new());
    };
    let v: Value = serde_json::from_slice(&crate::bounded_read(&path)?)
        .with_context(|| format!("company policy {}", path.display()))?;
    Ok(v["options"].as_object().cloned().unwrap_or_default())
}

/// A product's settings as its code sees them: the user's file, then the policy on top.
pub fn product_settings(state: &Path, product: &str) -> Result<Value> {
    let path = settings_file(state, product);
    let mut v: Value = if path.exists() {
        serde_json::from_slice(&crate::bounded_read(&path)?)?
    } else {
        json!({})
    };
    let Some(obj) = v.as_object_mut() else {
        bail!("{} must hold a JSON object", path.display());
    };
    let prefix = format!("{product}.");
    for (key, forced) in policy()? {
        if let Some(name) = key.strip_prefix(&prefix) {
            obj.insert(name.to_string(), forced["value"].clone());
        }
    }
    Ok(v)
}

/// `footnote options`: every option with its value, where the value comes from, gain and cost.
pub fn options(state: &Path) -> Result<Value> {
    let policy = policy()?;
    let mut out = Vec::new();
    for o in catalogue() {
        let (product, name) = o.key.split_once('.').unwrap_or(("", o.key));
        let user = read_user(state, product)?;
        let (value, source, reason) = match policy.get(o.key) {
            Some(f) => (f["value"].clone(), "policy", f["reason"].clone()),
            None => match user.get(name) {
                Some(v) => (v.clone(), "user", Value::Null),
                None => (o.default.clone(), "default", Value::Null),
            },
        };
        out.push(json!({"key": o.key, "value": value, "source": source, "locked": source == "policy", "reason": reason,
            "default": o.default, "range": o.range, "gain": o.gain, "cost": o.cost}));
    }
    Ok(json!({"options": out, "policy_file": policy_path()}))
}

fn read_user(state: &Path, product: &str) -> Result<Map<String, Value>> {
    let path = settings_file(state, product);
    if !path.exists() {
        return Ok(Map::new());
    }
    let v: Value = serde_json::from_slice(&crate::bounded_read(&path)?)?;
    v.as_object()
        .cloned()
        .with_context(|| format!("{} must hold a JSON object", path.display()))
}

/// `footnote config set`: checks the key, the type, the range and the policy; keeps every other setting of the file.
pub fn set(state: &Path, key: &str, raw: &str) -> Result<Value> {
    let o = spec(key)?;
    if let Some(f) = policy()?.get(key) {
        bail!(
            "`{key}` is set by your company policy: {}",
            f["reason"].as_str().unwrap_or("no reason given")
        );
    }
    let value = match o.range {
        None => match raw {
            "true" | "on" => json!(true),
            "false" | "off" => json!(false),
            _ => bail!("`{key}` takes true or false"),
        },
        Some((lo, hi)) => match raw.parse::<u64>() {
            Ok(n) if (lo..=hi).contains(&n) => json!(n),
            _ => bail!("`{key}` takes a whole number from {lo} to {hi}"),
        },
    };
    let (product, name) = key.split_once('.').unwrap_or(("", key));
    set_value(state, product, name, value.clone())?;
    Ok(json!({"key": key, "value": value}))
}

/// Write one setting of a product's file (also used for values outside the catalogue, such as the model list), unless the
/// company policy sets it; every other setting in the file is kept.
pub fn set_value(state: &Path, product: &str, name: &str, value: Value) -> Result<()> {
    let key = format!("{product}.{name}");
    if let Some(f) = policy()?.get(&key) {
        bail!(
            "`{key}` is set by your company policy: {}",
            f["reason"].as_str().unwrap_or("no reason given")
        );
    }
    let mut user = read_user(state, product)?;
    user.insert(name.to_string(), value);
    crate::state::ensure_private_dir(state)?;
    // Write beside, then rename over: a crash never leaves a half-written settings file for the hook to read.
    let tmp = state.join(format!(".{product}.json.{}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_string_pretty(&user)?)?;
    std::fs::rename(&tmp, settings_file(state, product))?;
    Ok(())
}

/// `footnote config get`: one option's effective value, or every option.
pub fn get(state: &Path, key: Option<&str>) -> Result<Value> {
    let all = options(state)?;
    match key {
        None => Ok(all),
        Some(k) => {
            spec(k)?;
            Ok(all["options"]
                .as_array()
                .and_then(|list| list.iter().find(|o| o["key"] == k).cloned())
                .unwrap_or(Value::Null))
        }
    }
}
