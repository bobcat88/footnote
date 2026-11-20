//! `footnote docs refresh`: unchanged sites cost no re-indexing, and only changed, removed and new pages are touched.
use assert_cmd::Command;
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

type Site = Arc<Mutex<HashMap<String, String>>>;

fn page(title: &str, body: &str, links: &[&str]) -> String {
    let a: String = links
        .iter()
        .map(|l| format!("<a href=\"{l}\">x</a> "))
        .collect();
    format!("<html><body><main><h1>{title}</h1><p>{body}</p>{a}</main></body></html>")
}

/// A local docs site with ETags. Returns its port and how many full (200) bodies it has served.
fn serve(site: Site) -> (u16, Arc<AtomicUsize>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let full = Arc::new(AtomicUsize::new(0));
    let served = full.clone();
    thread::spawn(move || {
        for mut c in l.incoming().flatten() {
            let (site, served) = (site.clone(), served.clone());
            thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let n = c.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let body = site.lock().unwrap().get(&path).cloned();
                let reply = match body {
                    None => {
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .into()
                    }
                    Some(b) => {
                        let etag = format!(
                            "\"{:x}\"",
                            b.len() * 31 + b.bytes().map(u64::from).sum::<u64>() as usize
                        );
                        if req
                            .to_lowercase()
                            .contains(&format!("if-none-match: {etag}"))
                        {
                            format!(
                                "HTTP/1.1 304 Not Modified\r\nETag: {etag}\r\nConnection: close\r\n\r\n"
                            )
                        } else {
                            served.fetch_add(1, Ordering::SeqCst);
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nETag: {etag}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}",
                                b.len()
                            )
                        }
                    }
                };
                let _ = c.write_all(reply.as_bytes());
            });
        }
    });
    (port, full)
}

#[test]
fn refresh_touches_only_what_changed() {
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let run = |args: &[&str]| -> Value {
        let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
            .env("FOOTNOTE_STATE_DIR", &state)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let filler =
        "Connections are reused from the pool and returned when the request ends. ".repeat(30);
    let site: Site = Arc::default();
    let set = |path: &str, html: String| {
        site.lock().unwrap().insert(path.into(), html);
    };
    set("/docs/", page("Home", &filler, &["/docs/a", "/docs/b"]));
    set("/docs/a", page("Alpha", &filler, &["/docs/"]));
    set(
        "/docs/b",
        page(
            "Beta",
            &format!("The zebra setting is off. {filler}"),
            &["/docs/"],
        ),
    );
    let (port, full) = serve(site.clone());
    let url = format!("http://127.0.0.1:{port}/docs/");

    let added = run(&["docs", "add", &url, "--no-embed", "--site", "mock"]);
    assert_eq!(added["index"]["indexed"], 3, "{added}");
    let db = added["db"].as_str().unwrap().to_string();
    let chunks = |needle: &str| -> i64 {
        rusqlite::Connection::open(&db)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM chunks c JOIN sources s ON s.id=c.source_id WHERE s.status='indexed' AND c.text LIKE ?1",
                [format!("%{needle}%")],
                |r| r.get(0),
            )
            .unwrap()
    };
    let refresh = || run(&["docs", "refresh", "mock", "--no-embed"])["refreshed"][0].clone();

    // Unchanged: every page answers 304, nothing is re-indexed.
    let before = full.load(Ordering::SeqCst);
    let r = refresh();
    assert_eq!(
        r["summary"], "3 unchanged, 0 changed, 0 added, 0 removed",
        "{r}"
    );
    assert_eq!(
        full.load(Ordering::SeqCst),
        before,
        "no page body was downloaded again"
    );

    // b changes materially, a gets a spelling fix (cosmetic), and b's new link finds the new page c.
    set(
        "/docs/b",
        page(
            "Beta",
            &format!("The zebra setting is on by default now. {filler}"),
            &["/docs/", "/docs/c"],
        ),
    );
    set(
        "/docs/a",
        page(
            "Alpha",
            &filler.replacen("reused", "re-used", 1),
            &["/docs/"],
        ),
    );
    set(
        "/docs/c",
        page("Gamma", &format!("The giraffe flag is new. {filler}"), &[]),
    );
    let r = refresh();
    assert_eq!(
        r["summary"], "1 unchanged, 1 changed, 1 added, 0 removed",
        "{r}"
    );
    assert_eq!(r["cosmetic_skipped"], 1, "{r}");
    assert_eq!(chunks("zebra setting is on by default"), 1);
    assert_eq!(chunks("zebra setting is off"), 0);
    assert_eq!(chunks("giraffe flag"), 1);

    // Punctuation only: cosmetic, the indexed text stays.
    set(
        "/docs/c",
        page("Gamma", &format!("The giraffe flag is new! {filler}"), &[]),
    );
    let r = refresh();
    assert_eq!(r["cosmetic_skipped"], 1, "{r}");
    assert_eq!(r["changed"], 0, "{r}");

    // A removed page leaves the index.
    site.lock().unwrap().remove("/docs/c");
    let r = refresh();
    assert_eq!(r["removed"], 1, "{r}");
    assert_eq!(chunks("giraffe flag"), 0);
    assert_eq!(r["pages"], 3, "{r}");
}
