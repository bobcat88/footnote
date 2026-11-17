use assert_cmd::Command;
use serde_json::{Value, json};
use std::{fs, path::Path};

fn cmd(db: &Path, args: &[&str]) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
    c.arg("--db").arg(db).args(args);
    c
}
fn run(db: &Path, args: &[&str], stdin: Option<Value>) -> Value {
    let mut c = cmd(db, args);
    if let Some(v) = stdin {
        c.write_stdin(v.to_string());
    }
    serde_json::from_slice(&c.assert().success().get_output().stdout).unwrap()
}
fn fails(db: &Path, args: &[&str], stdin: Value) -> String {
    let out = cmd(db, args)
        .write_stdin(stdin.to_string())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(out).unwrap()
}
fn mem(text: &str, kind: &str, scope: &str, date: &str) -> Value {
    json!({"text": text, "kind": kind, "scope": scope, "observed_at": date})
}
fn texts(v: &Value) -> Vec<String> {
    v["memories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["text"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn memories_are_scoped_deduplicated_superseded_and_forgotten() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("memory.db");
    let stored = run(
        &db,
        &["remember"],
        Some(json!([
            mem(
                "Johan wants relative % gaps in benchmark tables.",
                "preference",
                "global",
                "2026-11-13"
            ),
            mem(
                "Footnote v1.2 was released on 2026-11-03.",
                "fact",
                "footnote",
                "2026-11-04"
            ),
            mem(
                "Cargo looked missing: Rust is in Program Files, not on the shell PATH.",
                "fix",
                "footnote",
                "2026-11-14"
            ),
            mem(
                "Arkea uses the internal mirror for every package install.",
                "fact",
                "arkea",
                "2026-11-10"
            ),
        ])),
    );
    assert_eq!(stored["stored"].as_array().unwrap().len(), 4);
    // The same memory again is a duplicate, not a second row.
    let again = run(
        &db,
        &["remember"],
        Some(json!([mem(
            "Johan wants relative % gaps in benchmark tables.",
            "preference",
            "global",
            "2026-11-13"
        )])),
    );
    assert_eq!(again["duplicates"], 1);

    // Pack: the global preference first, this project's memories next, never another project's.
    let pack = run(&db, &["memory-pack", "--project", "footnote"], None);
    let p = texts(&pack);
    assert_eq!(p[0], "Johan wants relative % gaps in benchmark tables.");
    assert_eq!(p.len(), 3);
    assert!(!p.iter().any(|x| x.contains("Arkea")));

    // Recall is scoped the same way.
    let r = run(
        &db,
        &[
            "memory-recall",
            "package install mirror",
            "--project",
            "footnote",
        ],
        None,
    );
    assert!(!texts(&r).iter().any(|x| x.contains("Arkea")));
    let r = run(
        &db,
        &[
            "memory-recall",
            "package install mirror",
            "--project",
            "arkea",
        ],
        None,
    );
    assert!(texts(&r)[0].contains("Arkea"));

    // Hits say they are agent memories, with kind, scope and date in the heading.
    let release = run(
        &db,
        &["memory-recall", "released", "--project", "footnote"],
        None,
    );
    let id = release["memories"][0]["id"].as_i64().unwrap();
    let hit = run(&db, &["get", &id.to_string()], None);
    assert_eq!(hit["freshness"], "agent_memory");
    assert_eq!(hit["heading"], "fact · footnote · 2026-11-04");

    // A correction supersedes the old fact: only the new one is recalled.
    run(
        &db,
        &["remember"],
        Some(
            json!([{"text": "Footnote v1.2 was released on 2026-11-05.", "kind": "fact", "scope": "footnote", "observed_at": "2026-11-14", "supersedes": id}]),
        ),
    );
    let r = texts(&run(
        &db,
        &["memory-recall", "released", "--project", "footnote"],
        None,
    ));
    assert!(
        r.iter().any(|x| x.contains("2026-11-05")) && !r.iter().any(|x| x.contains("2026-11-03"))
    );

    // Forget removes it from recall and from the pack.
    let fix = run(
        &db,
        &["memory-recall", "cargo PATH", "--project", "footnote"],
        None,
    )["memories"][0]["id"]
        .as_i64()
        .unwrap();
    run(&db, &["forget", &fix.to_string()], None);
    assert!(
        !texts(&run(&db, &["memory-pack", "--project", "footnote"], None))
            .iter()
            .any(|x| x.contains("Cargo"))
    );

    // Invalid input is refused.
    assert!(
        fails(
            &db,
            &["remember"],
            json!([mem("x", "opinion", "global", "2026-11-14")])
        )
        .contains("kind")
    );
    assert!(
        fails(
            &db,
            &["remember"],
            json!([mem("x", "fact", "global", "14/11/2026")])
        )
        .contains("YYYY-MM-DD")
    );
}

#[test]
fn corpus_and_memory_databases_never_mix() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.md"), "# A\nsome text\n").unwrap();
    let r = root.to_str().unwrap();
    let corpus = t.path().join("corpus.db");
    run(&corpus, &["index", "--root", r, "--allow", r], None);
    let before = fs::read(&corpus).unwrap();
    assert!(
        fails(
            &corpus,
            &["remember"],
            json!([mem("x", "fact", "global", "2026-11-14")])
        )
        .contains("corpus")
    );
    assert_eq!(
        fs::read(&corpus).unwrap(),
        before,
        "a refused remember must not touch the corpus"
    );

    let memory = t.path().join("memory.db");
    run(
        &memory,
        &["remember"],
        Some(json!([mem("y", "fact", "global", "2026-11-14")])),
    );
    let out = cmd(&memory, &["index", "--root", r, "--allow", r])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8(out).unwrap().contains("memory database"));
}

#[test]
fn agents_can_remember_over_mcp_but_never_into_a_corpus() {
    let t = tempfile::tempdir().unwrap();
    let call = |db: &Path, tool: &str, args: Value| -> Value {
        let lines = [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":tool,"arguments":args}}),
        ];
        let input: String = lines.iter().map(|v| format!("{v}\n")).collect();
        let out = cmd(db, &["serve", "--mcp"])
            .write_stdin(input)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let last = String::from_utf8(out)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .to_string();
        serde_json::from_str::<Value>(&last).unwrap()["result"].clone()
    };
    let memory = t.path().join("memory.db");
    let ok = call(
        &memory,
        "footnote_remember",
        json!({"memories": [mem("Prefers French answers.", "preference", "global", "2026-11-14")]}),
    );
    assert_eq!(ok["isError"], false, "{ok}");
    assert_eq!(
        texts(&run(&memory, &["memory-pack"], None)),
        ["Prefers French answers."]
    );

    let root = t.path().join("notes");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.md"), "# A\nsome text\n").unwrap();
    let r = root.to_str().unwrap();
    let corpus = t.path().join("corpus.db");
    run(&corpus, &["index", "--root", r, "--allow", r], None);
    let before = fs::read(&corpus).unwrap();
    let refused = call(
        &corpus,
        "footnote_remember",
        json!({"memories": [mem("x", "fact", "global", "2026-11-14")]}),
    );
    assert_eq!(refused["isError"], true);
    let refused = call(&corpus, "footnote_forget", json!({"id": 1}));
    assert_eq!(refused["isError"], true);
    assert_eq!(fs::read(&corpus).unwrap(), before);
}

#[test]
fn rescope_moves_a_project_and_keeps_duplicate_detection() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("memory.db");
    run(
        &db,
        &["remember"],
        Some(json!([
            mem("Builds use cargo xtask.", "fact", "oldname", "2026-11-15"),
            mem("Prefers tables.", "preference", "oldname", "2026-11-15"),
            mem(
                "Always answer in French.",
                "preference",
                "global",
                "2026-11-15"
            ),
            mem("Prefers tables.", "preference", "newname", "2026-11-15"),
        ])),
    );
    let moved = run(
        &db,
        &["memory-rescope", "--from", "oldname", "--to", "newname"],
        None,
    );
    assert_eq!(moved["moved"], 1, "{moved}");
    assert_eq!(
        moved["kept_duplicates"].as_array().unwrap().len(),
        1,
        "the twin under newname stays apart"
    );
    let pack = run(&db, &["memory-pack", "--project", "newname"], None);
    let all = texts(&pack).join("\n");
    assert!(
        all.contains("cargo xtask") && all.contains("French"),
        "{pack}"
    );
    // Saving the moved memory again under its new scope is recognised as a duplicate.
    let again = run(
        &db,
        &["remember"],
        Some(json!([mem(
            "Builds use cargo xtask.",
            "fact",
            "newname",
            "2026-11-15"
        )])),
    );
    assert_eq!(again["duplicates"], 1, "{again}");
    assert!(
        fails(
            &db,
            &["memory-rescope", "--from", "a", "--to", "a"],
            json!(null)
        )
        .contains("different")
    );
}

#[test]
fn tags_narrow_recall_before_ranking_and_untagged_memories_stay_candidates() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("memory.db");
    let tagged = |text: &str, tags: Value| {
        let mut m = mem(text, "fix", "footnote", "2026-11-16");
        m["tags"] = tags;
        m
    };
    let stored = run(
        &db,
        &["remember"],
        Some(json!([
            tagged(
                "The daemon socket on Windows needs a named pipe",
                json!(["windows", "daemon"])
            ),
            tagged(
                "The daemon socket on Linux is a Unix domain socket",
                json!(["linux", "daemon"])
            ),
            mem(
                "The daemon socket path lives in the state folder",
                "fact",
                "footnote",
                "2026-11-16"
            ),
            mem(
                "The daemon socket of the other project is a TCP port",
                "fact",
                "other",
                "2026-11-16"
            ),
        ])),
    );
    assert_eq!(stored["stored"][0]["tags"], json!(["daemon", "windows"]));
    let linux_id = stored["stored"][1]["id"].as_i64().unwrap().to_string();

    let recall = |extra: &[&str]| {
        let mut args = vec![
            "memory-recall",
            "daemon socket",
            "--project",
            "footnote",
            "--limit",
            "5",
        ];
        args.extend_from_slice(extra);
        texts(&run(&db, &args, None))
    };
    // Other projects never compete; the tag keeps windows and the untagged memory, drops linux.
    let all = recall(&[]);
    assert_eq!(all.len(), 3, "{all:?}");
    let windows = recall(&["--tag", "windows"]);
    assert_eq!(windows.len(), 2, "{windows:?}");
    assert!(windows.iter().all(|t| !t.contains("Linux")), "{windows:?}");

    // Tags change later: the Linux fix also turns out to apply on Windows.
    let retagged = run(
        &db,
        &[
            "memory-tag",
            &linux_id,
            "--add",
            "windows",
            "--remove",
            "linux",
        ],
        None,
    );
    assert_eq!(retagged["tags"], json!(["daemon", "windows"]));
    assert_eq!(recall(&["--tag", "windows"]).len(), 3);
    let listed = run(&db, &["memory-list"], None);
    assert!(
        listed["memories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["tags"] == json!(["daemon", "windows"]))
    );

    // Tag engine: tags named as whole words in the query filter on their own; none named, no filter.
    let inferred = run(
        &db,
        &[
            "memory-recall",
            "why does the daemon fail on Windows?",
            "--project",
            "footnote",
            "--limit",
            "5",
            "--infer-tags",
        ],
        None,
    );
    assert_eq!(
        inferred["tags_used"],
        json!(["daemon", "windows"]),
        "{inferred}"
    );
    let plain = run(
        &db,
        &[
            "memory-recall",
            "socket path",
            "--project",
            "footnote",
            "--limit",
            "5",
            "--infer-tags",
        ],
        None,
    );
    assert_eq!(plain["tags_used"], json!([]));
    assert_eq!(plain["candidates"], 3);

    // Validation: bad shape and more than ten tags are refused; nothing is stored.
    assert!(
        fails(
            &db,
            &["remember"],
            json!([tagged("x", json!(["Not Valid"]))])
        )
        .contains("lowercase")
    );
    let eleven: Vec<String> = (0..11).map(|i| format!("t{i}")).collect();
    assert!(fails(&db, &["remember"], json!([tagged("y", json!(eleven))])).contains("at most 10"));
}

#[cfg(feature = "memory")]
#[test]
fn memories_anchored_to_code_are_found_by_symbol_or_file() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("memory.db");
    let mut a = mem(
        "The usage reader counts Codex limits.",
        "fact",
        "footnote",
        "2026-11-17",
    );
    a["code"] = json!(["src\\usage.rs", "./claude_line"]);
    let stored = run(
        &db,
        &["remember"],
        Some(json!([
            a,
            mem("Unanchored note.", "fact", "footnote", "2026-11-17")
        ])),
    );
    assert_eq!(
        stored["stored"][0]["code"],
        json!(["claude_line", "src/usage.rs"])
    );
    let c = rusqlite::Connection::open(&db).unwrap();
    let by = |anchors: &[&str]| {
        let anchors: Vec<String> = anchors.iter().map(|s| s.to_string()).collect();
        footnote_memory::anchored(&c, &anchors, 5).unwrap()
    };
    assert_eq!(by(&["src/usage.rs"]).len(), 1);
    assert_eq!(by(&["claude_line", "src/usage.rs"]).len(), 1); // one memory, listed once
    assert!(by(&["src/find.rs"]).is_empty());
    let under = |f: &str| footnote_memory::anchored_under(&c, f, 5).unwrap().0;
    assert_eq!(
        (
            under("src"),
            under("sr"),
            under("src/usage.rs"),
            under("crates")
        ),
        (1, 0, 1, 0)
    );
    let id = stored["stored"][0]["id"].as_i64().unwrap().to_string();
    run(&db, &["forget", &id], None);
    assert!(
        by(&["src/usage.rs"]).is_empty(),
        "a forgotten memory is not returned"
    );
    let mut bad = mem("x", "fact", "footnote", "2026-11-17");
    bad["code"] = json!("src/usage.rs");
    assert!(fails(&db, &["remember"], json!([bad])).contains("`code` must be an array"));
}
