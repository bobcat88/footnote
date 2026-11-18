use assert_cmd::Command;
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path};

fn usage(dir: &Path) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", dir.join("state"))
        .env("FOOTNOTE_POLICY_FILE", dir.join("no-policy.json"))
        .env("CLAUDE_CONFIG_DIR", dir.join("claude"))
        .env("CODEX_HOME", dir.join("codex"))
        .env("DSH_HOME", dir.join("dsh"))
        .args(["usage", "--days", "1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn claude(id: &str, model: &str, ts: &str, usage: Value) -> String {
    json!({"type": "assistant", "timestamp": ts, "requestId": format!("req-{id}"),
        "message": {"id": id, "model": model, "usage": usage, "content": [{"type": "text", "text": "never kept"}]}})
    .to_string()
        + "\n"
}

fn model<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["by_model"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["model"] == name)
        .unwrap()
}

fn close(a: &Value, b: f64) -> bool {
    (a.as_f64().unwrap() - b).abs() < 1e-9
}

/// Exact tokens and prices from hand-written session files; a request written twice counts once, a half-written line
/// waits for the next run, and a rewritten file is read again instead of added twice.
#[test]
fn usage_counts_tokens_and_list_prices_from_session_files() {
    let dir = tempfile::tempdir().unwrap();
    let ts = format!("{}T09:00:00.000Z", time::OffsetDateTime::now_utc().date());
    let session = dir.path().join("claude/projects/demo/s1.jsonl");
    fs::create_dir_all(session.parent().unwrap()).unwrap();
    let opus = json!({"input_tokens": 10, "output_tokens": 100, "cache_read_input_tokens": 1000,
        "cache_creation_input_tokens": 300, "cache_creation": {"ephemeral_1h_input_tokens": 200, "ephemeral_5m_input_tokens": 100}});
    let first = format!(
        "{}{}{}{}",
        json!({"type": "user", "timestamp": ts, "message": {"content": "a question"}}).to_string()
            + "\n",
        claude("m1", "claude-opus-5-5", &ts, opus.clone()),
        claude("m1", "claude-opus-5-5", &ts, opus), // the same request, written again for its next content block
        claude(
            "m0",
            "<synthetic>",
            &ts,
            json!({"input_tokens": 9, "output_tokens": 9})
        ),
    );
    fs::write(&session, &first).unwrap();
    let rollout = dir.path().join("codex/sessions/2026/11/17/rollout-a.jsonl");
    fs::create_dir_all(rollout.parent().unwrap()).unwrap();
    fs::write(
        &rollout,
        json!({"timestamp": ts, "type": "turn_context", "payload": {"model": "gpt-6.1-sol"}}).to_string()
            + "\n"
            + &json!({"timestamp": ts, "type": "token_usage_record", "payload": {"response_id": "r1",
                "usage": {"input_tokens": 1000, "cached_input_tokens": 400, "output_tokens": 20}}})
            .to_string()
            + "\n",
    )
    .unwrap();

    let v = usage(dir.path());
    let o = model(&v, "claude-opus-5-5");
    assert_eq!(
        (
            &o["requests"],
            &o["input"],
            &o["output"],
            &o["cache_read"],
            &o["cache_write_1h"],
            &o["cache_write_5m"]
        ),
        (
            &json!(1),
            &json!(10),
            &json!(100),
            &json!(1000),
            &json!(200),
            &json!(100)
        ),
        "{v}"
    );
    // Opus 5.5: 10 x $4 + 100 x $5 (5 min writes) + 200 x $8 (1 h writes) + 1000 x $0.20 (reads) + 100 x $20, per million.
    assert!(close(&o["api_price_usd"], 4340e-6), "{v}");
    // The same price by bucket: reads 1000 x $0.20, writes 100 x $5 + 200 x $8, input 10 x $4, output 100 x $20.
    for (k, usd) in [
        ("cost_cache_read", 200e-6),
        ("cost_cache_write", 2100e-6),
        ("cost_input", 40e-6),
        ("cost_output", 2000e-6),
    ] {
        assert!(close(&o[k], usd), "{k}: {v}");
    }
    let g = model(&v, "gpt-6.1-sol");
    assert_eq!(
        (&g["input"], &g["cache_read"]),
        (&json!(600), &json!(400)),
        "{v}"
    );
    assert!(close(&g["api_price_usd"], 1440e-6), "{v}"); // 600 x $2 + 400 x $0.10 + 20 x $10
    assert_eq!(v["totals"]["requests"], 2, "{v}");
    assert_eq!(v["by_agent"].as_array().unwrap().len(), 2, "{v}");
    assert!(
        !fs::read_to_string(dir.path().join("state/usage/index.json"))
            .unwrap()
            .contains("never kept")
    );

    // A line still being written is left for the next run, then counted once.
    let more = claude(
        "m2",
        "claude-sonnet-5-5",
        &ts,
        json!({"input_tokens": 5, "output_tokens": 50}),
    );
    let (head, tail) = more.split_at(40);
    fs::OpenOptions::new()
        .append(true)
        .open(&session)
        .unwrap()
        .write_all(head.as_bytes())
        .unwrap();
    assert_eq!(usage(dir.path())["totals"]["requests"], 2);
    fs::OpenOptions::new()
        .append(true)
        .open(&session)
        .unwrap()
        .write_all(tail.as_bytes())
        .unwrap();
    let v = usage(dir.path());
    assert_eq!(v["totals"]["requests"], 3, "{v}");
    assert!(
        close(&model(&v, "claude-sonnet-5-5")["api_price_usd"], 510e-6),
        "{v}"
    );

    // Rewritten shorter (an agent compacting its log): read again from the start, not added to the old counts.
    fs::write(
        &session,
        claude(
            "m9",
            "claude-sonnet-5-5",
            &ts,
            json!({"input_tokens": 1, "output_tokens": 1}),
        ),
    )
    .unwrap();
    let v = usage(dir.path());
    assert_eq!(v["totals"]["requests"], 2, "{v}");

    // Codex archives a session by moving it: still counted, once.
    let archived = dir.path().join("codex/archived_sessions/rollout-a.jsonl");
    fs::create_dir_all(archived.parent().unwrap()).unwrap();
    fs::rename(&rollout, &archived).unwrap();
    let v = usage(dir.path());
    assert_eq!(model(&v, "gpt-6.1-sol")["requests"], 1, "{v}");
    assert_eq!(model(&v, "claude-sonnet-5-5")["requests"], 1, "{v}");
}
