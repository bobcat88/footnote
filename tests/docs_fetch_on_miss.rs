#![cfg(feature = "library")]
//! Fetch on miss and `objects.inv` (#301), against a local mock docs site (no real network): a weak `find` fetches the missing
//! page once and then hits; a strong one fetches nothing; nothing outside the entry's scope is requested; at most three pages
//! per query; and a crawl is seeded from the inventory.
use assert_cmd::Command;
use footnote_library::miss::inventory_fixture;
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

type Site = Arc<Mutex<HashMap<String, Vec<u8>>>>;
type Log = Arc<Mutex<Vec<String>>>;

fn page(title: &str, body: &str, links: &[&str]) -> Vec<u8> {
    let a: String = links
        .iter()
        .map(|l| format!("<a href=\"{l}\">x</a> "))
        .collect();
    format!("<html><body><main><h1>{title}</h1><p>{body}</p>{a}</main></body></html>").into_bytes()
}

/// Serves `site` and logs every requested path.
fn serve(site: Site) -> (u16, Log) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let log: Log = Arc::default();
    let seen = log.clone();
    thread::spawn(move || {
        for mut c in l.incoming().flatten() {
            let (site, seen) = (site.clone(), seen.clone());
            thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let n = c.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                seen.lock().unwrap().push(path.clone());
                let body = site.lock().unwrap().get(&path).cloned();
                let (head, body) = match body {
                    None => (
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_string(),
                        vec![],
                    ),
                    Some(b) => (
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            if path.ends_with(".inv") {
                                "application/octet-stream"
                            } else {
                                "text/html"
                            },
                            b.len()
                        ),
                        b,
                    ),
                };
                let _ = c.write_all(head.as_bytes());
                let _ = c.write_all(&body);
            });
        }
    });
    (port, log)
}

const FILLER: &str = "Connections are reused from the pool and returned when the request ends. ";

fn footnote(state: &std::path::Path, args: &[&str]) -> Value {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", state)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}

fn library(state: &std::path::Path, query: &str) -> Value {
    footnote(state, &["find", query])["products"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["product"] == "library")
        .unwrap()
        .clone()
}

fn count(log: &Log, prefix: &str) -> usize {
    log.lock()
        .unwrap()
        .iter()
        .filter(|p| p.starts_with(prefix))
        .count()
}

#[test]
fn a_weak_search_fetches_the_missing_page_once_and_a_strong_one_fetches_nothing() {
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let site: Site = Arc::default();
    let put = |p: &str, b: Vec<u8>| {
        site.lock().unwrap().insert(p.into(), b);
    };
    put("/docs/", page("Home", &FILLER.repeat(30), &["/docs/a"]));
    put("/docs/a", page("Alpha", &FILLER.repeat(30), &["/docs/"]));
    let (port, log) = serve(site.clone());
    footnote(
        &state,
        &[
            "docs",
            "add",
            &format!("http://127.0.0.1:{port}/docs/"),
            "--no-embed",
            "--site",
            "mock",
        ],
    );
    log.lock().unwrap().clear(); // the crawl itself asked for objects.inv (and found none)

    // The API page exists on the site but is not in the cache. The inventory also lists pages outside the scope and six more
    // pages for one word.
    put(
        "/docs/api/",
        page(
            "API",
            "Session.mount registers a transport adapter for a URL prefix, so requests to it use that adapter.",
            &[],
        ),
    );
    let mut inv = String::from(
        "requests.Session.mount py:method 1 api/#$ -\noutside py:function 1 ../outside/page.html -\n",
    );
    for i in 1..=6 {
        inv += &format!("widget.w{i} py:class 1 w{i}/#$ -\n");
        put(
            &format!("/docs/w{i}/"),
            page(
                &format!("Widget {i}"),
                &format!("The widget number {i} draws a box. {}", FILLER.repeat(5)),
                &[],
            ),
        );
    }
    put("/docs/objects.inv", inventory_fixture(&inv));
    put(
        "/outside/page.html",
        page("Secret", "Outside the scope of the entry.", &[]),
    );

    // Strong: a cached page answers, nothing is fetched and the inventory is not even read.
    let r = library(&state, "connections reused pool");
    assert_eq!(r["state"], "ok", "{r}");
    assert!(r.get("live_fetch").is_none(), "{r}");
    assert_eq!(count(&log, "/docs/objects.inv"), 0);

    // Weak: a symbol the cache lacks. objects.inv resolves it to its page, which is fetched, indexed, and found.
    let r = library(&state, "requests.Session.mount");
    assert_eq!(
        r["live_fetch"]["pages"],
        serde_json::json!([format!("http://127.0.0.1:{port}/docs/api/")]),
        "{r}"
    );
    assert!(
        r["live_fetch"]["note"]
            .as_str()
            .unwrap()
            .starts_with("Live fetch:"),
        "{r}"
    );
    let snippets = r["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["snippet"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(snippets.contains("transport adapter"), "{r}");
    assert_eq!(count(&log, "/docs/api/"), 1);

    // The same query again is strong: no second fetch.
    let r = library(&state, "requests.Session.mount");
    assert!(r.get("live_fetch").is_none(), "{r}");
    assert_eq!(count(&log, "/docs/api/"), 1);

    // Cap: six pages match "widget", three are fetched.
    let r = library(&state, "widget");
    assert_eq!(r["live_fetch"]["pages"].as_array().unwrap().len(), 3, "{r}");
    assert_eq!(count(&log, "/docs/w"), 3);

    // Scope: the inventory names a page outside /docs/; it is never requested, and nothing was fetched for the query.
    let r = library(&state, "outside");
    assert!(r.get("live_fetch").is_none(), "{r}");
    assert_eq!(count(&log, "/outside/"), 0);

    // FOOTNOTE_NO_LIVE_FETCH=1: three widget pages are still missing, yet nothing is requested.
    let before = count(&log, "/docs/w");
    let out = Command::new(assert_cmd::cargo::cargo_bin!("footnote"))
        .env("FOOTNOTE_STATE_DIR", &state)
        .env("FOOTNOTE_NO_LIVE_FETCH", "1")
        .args(["find", "widget"])
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("live_fetch"),
        "{out:?}"
    );
    assert_eq!(count(&log, "/docs/w"), before);
}

#[test]
fn a_crawl_is_seeded_from_objects_inv() {
    let t = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let state = t.path().join("state");
    let site: Site = Arc::default();
    let put = |p: &str, b: Vec<u8>| {
        site.lock().unwrap().insert(p.into(), b);
    };
    // Nothing links to the API page and there is no sitemap: only the inventory knows it.
    put("/docs/", page("Home", &FILLER.repeat(30), &[]));
    put(
        "/docs/api.html",
        page(
            "API",
            &format!("The frobnicate function. {}", FILLER.repeat(5)),
            &[],
        ),
    );
    put(
        "/docs/objects.inv",
        inventory_fixture("pkg.frobnicate py:function 1 api.html#$ -\n"),
    );
    let (port, _) = serve(site);
    let r = footnote(
        &state,
        &[
            "docs",
            "add",
            &format!("http://127.0.0.1:{port}/docs/"),
            "--no-embed",
            "--site",
            "seed",
        ],
    );
    assert_eq!(r["index"]["indexed"], 2, "{r}");
}
