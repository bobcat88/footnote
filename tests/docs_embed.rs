//! Keyword index first, embeddings in the background: search works before vectors exist, the state is visible, a dead worker
//! leaves a usable corpus and re-running resumes. Local fixtures only (the model is the repo's tiny minilm).
use assert_cmd::Command;
use serde_json::{Value, json};
use std::{fs, path::Path, time::Duration};

const MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/minilm");

fn run(state: &Path, args: &[&str]) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{args:?}: {out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}

fn row_state(state: &Path) -> String {
    run(state, &["docs", "list"])["docs"][0]["state"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn keyword_search_first_then_background_embedding_resumes_after_a_dead_worker() {
    // Outside the system temp folder: docs registers explicitly, but keep the same layout as docs_cache.
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let notes = t.path().join("notes");
    fs::create_dir(&notes).unwrap();
    fs::write(
        notes.join("wal.md"),
        "# SQLite WAL\nReaders and writers can run concurrently using write-ahead logging.",
    )
    .unwrap();
    fs::write(
        notes.join("http.md"),
        "# HTTP\nETag conditional requests avoid downloading unchanged resources.",
    )
    .unwrap();
    let from = notes.to_str().unwrap();

    // 1. Keyword index only: returns at once, searchable, state keyword-only.
    let first = run(
        &state,
        &[
            "docs",
            "add",
            "--from",
            from,
            "--site",
            "lib",
            "--version",
            "1",
            "--no-embed",
        ],
    );
    assert_eq!(first["state"], "keyword-only", "{first}");
    let db = first["db"].as_str().unwrap().to_string();
    let hit = run(
        &state,
        &["--db", &db, "search", "write-ahead logging", "--limit", "1"],
    );
    assert!(
        hit["results"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("wal.md"),
        "{hit}"
    );
    assert_eq!(row_state(&state), "keyword-only");

    // 2. A live worker's job shows as `embedding n/m`, and a second `add` does not start another one.
    let jobs = state.join("jobs");
    fs::create_dir_all(&jobs).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let job = |pid: u32| {
        json!({"id": "x", "kind": "docs.embed", "label": db, "unit": "chunks", "pid": pid, "state": "running",
            "done": 3, "total": 10, "started": now, "updated": now})
        .to_string()
    };
    fs::write(jobs.join("x.json"), job(std::process::id())).unwrap();
    assert_eq!(row_state(&state), "embedding 3/10");
    let again = run(
        &state,
        &[
            "docs",
            "add",
            "--from",
            from,
            "--site",
            "lib",
            "--version",
            "1",
            "--model",
            MODEL,
        ],
    );
    assert_eq!(again["embedding"], "nothing to embed", "{again}");

    // 3. The worker died: the job reads interrupted, the corpus is keyword-only and still answers.
    fs::write(jobs.join("x.json"), job(4_000_000)).unwrap();
    let listed = run(&state, &["jobs"]);
    assert_eq!(listed["jobs"][0]["state"], "interrupted", "{listed}");
    assert_eq!(row_state(&state), "keyword-only");
    assert!(
        run(&state, &["--db", &db, "search", "ETag", "--limit", "1"])["results"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("http.md")
    );

    // 4. Re-running resumes: the corpus is reused, not re-crawled, and a worker embeds it until `ready`.
    let resumed = run(
        &state,
        &[
            "docs",
            "add",
            "--from",
            from,
            "--site",
            "lib",
            "--version",
            "1",
            "--model",
            MODEL,
        ],
    );
    assert_eq!(resumed["reused"], true, "{resumed}");
    assert_eq!(resumed["embedding"]["started"], 1, "{resumed}");
    let mut waited = 0;
    while row_state(&state) != "ready" {
        assert!(
            waited < 240,
            "embedding never finished: {}",
            run(&state, &["jobs"])
        );
        std::thread::sleep(Duration::from_secs(1));
        waited += 1;
    }
    // Search upgrades to hybrid by itself once the vectors are in (hybrid by name needs the model folder).
    let hybrid = run(
        &state,
        &[
            "--db",
            &db,
            "search",
            "write-ahead logging",
            "--mode",
            "hybrid",
            "--model",
            MODEL,
            "--limit",
            "1",
        ],
    );
    assert_eq!(hybrid["mode"], "hybrid_cc", "{hybrid}");
    assert!(
        hybrid["results"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("wal.md")
    );
    let done = run(&state, &["jobs"]);
    assert!(
        done["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|j| j["kind"] == "docs.embed" && j["state"] == "done"),
        "{done}"
    );
}

#[test]
fn a_library_that_cannot_be_crawled_does_not_stop_the_others() {
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let v = run(
        &state,
        &[
            "docs",
            "ingest",
            "http://127.0.0.1:1/a/",
            "http://127.0.0.1:1/b/",
            "--no-embed",
        ],
    );
    let libs = v["libraries"].as_array().unwrap();
    assert_eq!(libs.len(), 2, "{v}");
    assert!(libs.iter().all(|l| l["error"].is_string()), "{v}");
}

#[test]
fn a_killed_embedding_keeps_its_batches_and_resumes_without_recomputing() {
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let notes = t.path().join("notes");
    fs::create_dir(&notes).unwrap();
    for i in 0..150 {
        fs::write(
            notes.join(format!("p{i}.md")),
            format!(
                "# Page {i}
Unique text about topic number {i} and its details."
            ),
        )
        .unwrap();
    }
    let added = run(
        &state,
        &[
            "docs",
            "add",
            "--from",
            notes.to_str().unwrap(),
            "--site",
            "lib",
            "--version",
            "1",
            "--no-embed",
        ],
    );
    let db = added["db"].as_str().unwrap().to_string();
    let mut c = rusqlite::Connection::open(&db).unwrap();
    let count =
        |c: &rusqlite::Connection, sql: &str| c.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap();
    let chunks = count(&c, "SELECT count(*) FROM chunks");
    assert!(chunks > 128, "{chunks}");
    let models = [std::path::PathBuf::from(MODEL)];

    // Killed after two batches of 64: those vectors are durable, the rest is missing.
    let stopped = footnote_search::semantic::rebuild_stopping(&mut c, &models, &mut |_, _| {}, 2);
    assert!(stopped.is_err());
    assert_eq!(count(&c, "SELECT count(*) FROM embeddings"), 128);
    assert_eq!(row_state(&state), format!("keyword-only 128/{chunks}"));

    // The rerun reports its real start, embeds only the missing chunks and ends ready.
    let mut first = None;
    let r = footnote_search::semantic::rebuild_with(&mut c, &models, &mut |d, _| {
        first.get_or_insert(d);
    })
    .unwrap();
    assert_eq!(first, Some(128));
    assert_eq!(r["models"][0]["embedded"], chunks - 128, "{r}");
    assert_eq!(r["models"][0]["reused"], 128, "{r}");
    assert_eq!(count(&c, "SELECT count(*) FROM embeddings"), chunks);
    assert_eq!(row_state(&state), "ready");
}
