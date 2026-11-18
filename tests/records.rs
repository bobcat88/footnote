mod support;
use assert_cmd::Command;
use serde_json::{Value, json};
use std::{fs, path::Path};
fn cmd(db: &Path, args: &[&str]) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("footnote"));
    c.arg("--db").arg(db).args(args);
    c
}
fn run(db: &Path, args: &[&str]) -> Value {
    serde_json::from_slice(&cmd(db, args).assert().success().get_output().stdout).unwrap()
}
#[test]
fn contributions_require_hashed_evidence_and_revision_checked_validation() {
    if !(support::model("minilm")) {
        return;
    }
    let t = tempfile::tempdir_in(
        std::env::var("TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into()),
    )
    .unwrap();
    let db = t.path().join("k.db");
    let input = t.path().join("record.json");
    let root = t.path().to_str().unwrap();
    let proof = t.path().join("proof.txt");
    fs::write(
        &proof,
        "observed 21 evaluations before; 0 after; parity passed",
    )
    .unwrap();
    use sha2::{Digest, Sha256};
    let sha = format!("{:x}", Sha256::digest(fs::read(&proof).unwrap()));
    let record = json!({"technology":"SQLite","project":"synthetic","tested_versions":{"sqlite":"3.53.1"},"environment":"Windows synthetic","symptom":"wasteful receipt bodies","root_cause":"unnecessary substr evaluation","root_cause_status":"proven","remediation":"CASE excludes unused body","applicability":"synthetic receipts only","exclusions":"not production health","protocol":"count evaluations and compare exact output","tradeoffs":"branch overhead","rollback":"restore expression","last_verified":"2026-10-16","evidence":[{"path":proof,"sha256":sha}]});
    fs::write(&input, serde_json::to_vec(&record).unwrap()).unwrap();
    let proposal = run(
        &db,
        &[
            "propose",
            "--file",
            input.to_str().unwrap(),
            "--allow",
            root,
        ],
    );
    assert_eq!(proposal["status"], "proposal");
    let id = proposal["id"].as_i64().unwrap().to_string();
    assert_eq!(
        run(
            &db,
            &[
                "propose",
                "--file",
                input.to_str().unwrap(),
                "--allow",
                root
            ]
        )["id"],
        proposal["id"]
    );
    let evidence = t.path().join("validation.json");
    fs::write(&evidence,serde_json::to_vec(&json!({"record_id":proposal["id"],"record_revision":1,"record_sha256":proposal["payload_sha256"],"verdict":"passed","checks":[{"name":"exact parity and cost observer","passed":true}],"artifacts":[{"path":proof,"sha256":sha}]})).unwrap()).unwrap();
    cmd(
        &db,
        &[
            "validate",
            &id,
            "--revision",
            "2",
            "--evidence",
            evidence.to_str().unwrap(),
            "--allow",
            root,
        ],
    )
    .assert()
    .failure();
    let v = run(
        &db,
        &[
            "validate",
            &id,
            "--revision",
            "1",
            "--evidence",
            evidence.to_str().unwrap(),
            "--allow",
            root,
        ],
    );
    assert_eq!(v["revision"], 2);
    assert_eq!(v["status"], "validated");
    let hit = run(
        &db,
        &[
            "search",
            "substr",
            "--technology",
            "SQLite",
            "--tested-version",
            "sqlite=3.53.1",
            "--status",
            "validated",
        ],
    );
    assert_eq!(hit["records"].as_array().unwrap().len(), 1);
    assert!(
        run(
            &db,
            &["search", "substr", "--tested-version", "sqlite=3.50.0"]
        )["records"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let model = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/models/minilm");
    run(&db, &["rebuild-embeddings", "--model", model]);
    let hybrid = run(
        &db,
        &[
            "search",
            "éviter le texte des reçus inutilisés",
            "--mode",
            "hybrid",
            "--model",
            model,
            "--technology",
            "SQLite",
            "--tested-version",
            "sqlite=3.53.1",
            "--status",
            "validated",
        ],
    );
    assert_eq!(hybrid["records"][0]["id"], proposal["id"]);
    assert!(hybrid["results"].as_array().unwrap().is_empty());
    for (flag, value) in [
        ("--tested-version", "sqlite=3.50.0"),
        ("--status", "superseded"),
        ("--project", "wrong-project"),
        ("--technology", "PostgreSQL"),
    ] {
        let v = run(
            &db,
            &[
                "search", "receipt", "--mode", "hybrid", "--model", model, flag, value,
            ],
        );
        assert!(v["records"].as_array().unwrap().is_empty());
        assert!(v["results"].as_array().unwrap().is_empty());
    }
    fs::write(&proof, "tampered").unwrap();
    assert_eq!(
        run(&db, &["get-record", &id])["evidence_freshness"],
        "stale"
    );
    let _ = sha;
}
