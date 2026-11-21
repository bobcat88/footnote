use assert_cmd::Command;
use serde_json::{Value, json};
use std::fs;

fn bin() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin!("footnote").to_path_buf()
}

#[test]
fn audit_prefix_measures_instructions_skills_and_mcp_without_leaking_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let project = dir.path().join("project");
    fs::create_dir_all(home.join(".claude/skills/demo")).unwrap();
    fs::create_dir_all(home.join(".codex")).unwrap();
    fs::create_dir_all(&project).unwrap();

    // Project & user instruction files
    fs::write(project.join("CLAUDE.md"), "a".repeat(400)).unwrap();
    fs::write(project.join("AGENTS.md"), "b".repeat(200)).unwrap();
    fs::write(
        home.join(".claude/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: Short skill header\n---\n\nFull body that is much longer than the frontmatter.\n",
    )
    .unwrap();

    // Seed a real DB so `--probe-mcp` can probe `footnote --db ... serve --mcp`
    let db = dir.path().join("k.db");
    let notes = dir.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("a.md"), "# Doc\nhello world\n").unwrap();
    Command::new(bin())
        .arg("--db")
        .arg(&db)
        .args(["index", "--root"])
        .arg(&notes)
        .arg("--allow")
        .arg(&notes)
        .assert()
        .success();

    let exe = bin().to_string_lossy().replace('\\', "/");
    let db_str = db.to_string_lossy().replace('\\', "/");
    let secret = "sk-live-SUPER-SECRET-SHOULD-NEVER-APPEAR";

    // Claude JSON with footnote MCP + a secret env var (must never leak!)
    fs::write(
        home.join(".claude.json"),
        serde_json::to_string_pretty(&json!({
            "mcpServers": {
                "footnote-test": {
                    "command": exe,
                    "args": ["--db", db_str, "serve", "--mcp"],
                    "env": { "API_KEY": secret }
                },
                "remote-http": {
                    "url": "https://example.invalid/mcp"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    // Codex TOML config with one server
    fs::write(
        home.join(".codex/config.toml"),
        format!(
            "[mcp_servers.codex-srv]\ncommand = \"{exe}\"\nargs = [\"--db\", \"{db_str}\", \"serve\", \"--mcp\"]\n[mcp_servers.codex-srv.env]\nTOKEN = \"{secret}\"\n"
        ),
    )
    .unwrap();

    // 1. Offline scan (no --probe-mcp)
    let out = Command::new(bin())
        .env("USERPROFILE", &home)
        .env("HOME", &home)
        .args(["audit-prefix", "--root"])
        .arg(&project)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let raw = String::from_utf8(out).unwrap();
    assert!(
        !raw.contains(secret),
        "secret leaked in audit-prefix output"
    );
    let v: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["totals"]["instruction_files"], 2);
    assert_eq!(v["totals"]["instruction_bytes"], 600);
    assert_eq!(v["totals"]["skills"], 1);
    assert!(
        v["totals"]["skill_prefix_bytes"].as_u64().unwrap()
            < v["totals"]["skill_full_bytes"].as_u64().unwrap()
    );
    assert_eq!(v["totals"]["mcp_servers"], 3);
    assert_eq!(v["totals"]["mcp_probed"], 0);

    // 2. Live --probe-mcp scan against our own footnote MCP server
    let out2 = Command::new(bin())
        .env("USERPROFILE", &home)
        .env("HOME", &home)
        .args(["audit-prefix", "--root"])
        .arg(&project)
        .args(["--agent", "claude", "--probe-mcp", "--timeout-secs", "5"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let raw2 = String::from_utf8(out2).unwrap();
    assert!(
        !raw2.contains(secret),
        "secret leaked in --probe-mcp output"
    );
    let v2: Value = serde_json::from_str(&raw2).unwrap();
    assert_eq!(v2["totals"]["mcp_probed"], 1);
    let schema_bytes = v2["totals"]["mcp_schema_bytes"].as_u64().unwrap();
    assert!(
        (2_000..=5_000).contains(&schema_bytes),
        "unexpected footnote tools/list schema_bytes={schema_bytes}"
    );
    assert!(
        v2["totals"]["projected_50_turn_cache_read_tokens"]
            .as_u64()
            .unwrap()
            > 0
    );
    let servers = v2["mcp_servers"].as_array().unwrap();
    let fn_srv = servers
        .iter()
        .find(|s| s["name"] == "footnote-test")
        .unwrap();
    assert_eq!(fn_srv["probe_status"], "ok");
    assert!(fn_srv["tools_count"].as_u64().unwrap() >= 7);
    let http_srv = servers.iter().find(|s| s["name"] == "remote-http").unwrap();
    assert_eq!(http_srv["probe_status"], "skipped");
}
