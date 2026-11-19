//! Documentation crawler (backlog item 14; study: docs/research/DOCS-INGESTION-STUDY.md). Cheapest discovery first: the site's
//! `llms.txt`, its sitemap, then links within the start URL's path. Each page's main content (Readability) becomes Markdown in
//! `<out>/<n>.md`; `<out>/urls.json` maps every file to its URL. Polite by construction: robots.txt (rules and `Crawl-delay`),
//! an identifying user agent, and adaptive concurrency per host: a few requests in flight, more while the site answers, half as
//! many after a 429, a 503 or a timeout (`Retry-After` honoured). JavaScript is not run.
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    path::Path,
    thread,
    time::Duration,
};
use ureq::ResponseExt;
use url::Url;

const AGENT: &str =
    "FootnoteDocs/0.1 (+https://github.com/bobcat88/footnote; local documentation index)";
const MAX_BYTES: u64 = 8 * 1024 * 1024;
/// Most requests in flight to one host.
pub const MAX_CONCURRENCY: usize = 8;

pub struct Options<'a> {
    pub start: &'a str,
    pub max_pages: usize,
    /// Pages whose URL matches are skipped (for example translated mirrors).
    pub exclude: Option<regex::Regex>,
    /// Requests in flight at the start (1 to MAX_CONCURRENCY), for example the level this host reached last time.
    pub concurrency: usize,
}

const TLS_PROVIDER: ureq::tls::TlsProvider = ureq::tls::TlsProvider::Rustls;

fn agent() -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig};
    // System roots, not ureq's bundled Mozilla list: fastapi.tiangolo.com fails against the bundle, and companies add their own roots.
    // rustls, not the Windows TLS stack: Cloudflare answers the latter with a challenge (HTTP 429, `cf-mitigated`) on Read the Docs and
    // Pallets sites (jinja, flask), so no page could be crawled (DOCS-SPHINX-2026-11-19.md).
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TLS_PROVIDER)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_global(Some(Duration::from_secs(30)))
        .user_agent(AGENT)
        .http_status_as_error(false)
        .build()
        .into()
}

enum Fetch {
    /// Body, final URL after redirects, content type.
    Page(String, Url, String),
    /// The site asks us to slow down (429, 503) or did not answer in time; wait at least this long.
    SlowDown(Duration),
    Failed,
}

fn fetch(agent: &ureq::Agent, url: &Url) -> Fetch {
    let mut r = match agent.get(url.as_str()).call() {
        Ok(r) => r,
        Err(ureq::Error::Timeout(_)) => return Fetch::SlowDown(Duration::from_secs(2)),
        Err(_) => return Fetch::Failed,
    };
    let status = r.status().as_u16();
    if status == 429 || status == 503 {
        let wait = r
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(2)
            .clamp(1, 60);
        return Fetch::SlowDown(Duration::from_secs(wait));
    }
    match page(&mut r, url) {
        Some((body, last, kind)) => Fetch::Page(body, last, kind),
        None => Fetch::Failed,
    }
}

/// Body and final URL (after redirects) of a successful text response.
fn get(agent: &ureq::Agent, url: &Url) -> Option<(String, Url, String)> {
    match fetch(agent, url) {
        Fetch::Page(body, last, kind) => Some((body, last, kind)),
        _ => None,
    }
}

fn page(r: &mut ureq::http::Response<ureq::Body>, url: &Url) -> Option<(String, Url, String)> {
    if !r.status().is_success() {
        return None;
    }
    let kind = r
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let last = r
        .get_uri()
        .to_string()
        .parse::<Url>()
        .unwrap_or_else(|_| url.clone());
    let body = r
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_string()
        .ok()?;
    Some((body, last, kind))
}

/// Same host, under the start path, http(s), no fragment, not excluded.
fn in_scope(u: &Url, start: &Url, exclude: &Option<regex::Regex>) -> bool {
    // "/3/" and "/docs" keep their folder; "/guide/index.html" keeps "/guide".
    let p = start.path();
    let last = p.rsplit('/').next().unwrap_or("");
    let dir = if p.ends_with('/') || !last.contains('.') {
        p.trim_end_matches('/')
    } else {
        p.rsplit_once('/').map_or("", |(d, _)| d)
    };
    matches!(u.scheme(), "http" | "https")
        && u.host_str() == start.host_str()
        && (u.path() == dir || u.path().starts_with(&format!("{dir}/"))) // "/3" must not take "/3.11/"
        && !exclude.as_ref().is_some_and(|re| re.is_match(u.as_str()))
}

fn normalize(mut u: Url) -> Url {
    u.set_fragment(None);
    u.set_query(None);
    u
}

/// The address a reader would cite: a Markdown copy listed by llms.txt (`/guide/features.md`, `/concepts/config/index.md`)
/// stands for its HTML page (`/guide/features`, `/concepts/config/`).
fn page_url(u: &Url) -> String {
    let s = u.as_str();
    match s.strip_suffix(".md") {
        Some(page) => page
            .strip_suffix("/index")
            .map_or(page.to_string(), |dir| format!("{dir}/")),
        None => s.to_string(),
    }
}

/// One key per page whatever its form: `/guide/features`, `/guide/features/`, `.md`, `.html` and `/index.html` are the same page.
fn page_key(u: &Url) -> String {
    let s = u.as_str().trim_end_matches('/');
    let s = s
        .strip_suffix(".md")
        .or_else(|| s.strip_suffix(".html"))
        .unwrap_or(s);
    s.strip_suffix("/index").unwrap_or(s).to_string()
}

/// Links of an HTML page or of an llms.txt (Markdown links), resolved against `base`.
fn links(text: &str, base: &Url) -> Vec<Url> {
    static HREF: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = HREF.get_or_init(|| {
        regex::Regex::new(r#"(?:href=["']([^"'#][^"']*)["']|\]\((https?://[^)\s]+|/[^)\s]*)\))"#)
            .expect("valid pattern")
    });
    re.captures_iter(text)
        .filter_map(|c| c.get(1).or_else(|| c.get(2)))
        .filter_map(|m| base.join(m.as_str()).ok())
        .map(normalize)
        .collect()
}

/// Page URLs listed by the sitemap (`<loc>` entries; nested sitemap indexes are followed once).
fn sitemap(agent: &ureq::Agent, root: &Url) -> Vec<Url> {
    let loc = regex::Regex::new(r"<loc>\s*([^<\s]+)\s*</loc>").expect("valid pattern");
    let mut out = Vec::new();
    let Some((xml, _, _)) = root.join("/sitemap.xml").ok().and_then(|u| get(agent, &u)) else {
        return out;
    };
    for c in loc.captures_iter(&xml) {
        let Ok(u) = Url::parse(&c[1]) else { continue };
        if u.path().ends_with(".xml") {
            if let Some((inner, _, _)) = get(agent, &u) {
                out.extend(
                    loc.captures_iter(&inner)
                        .filter_map(|c| Url::parse(&c[1]).ok()),
                );
            }
        } else {
            out.push(u);
        }
    }
    out.into_iter().map(normalize).collect()
}

/// Main content as Markdown: Readability first; the page's main landmark (`main`, `role=main`) when Readability finds
/// nothing or picks text outside it (it prefers a long footer to a short page: Python's index and removed-module pages);
/// the whole body when there is no landmark. Not `article`: on Tailwind's pages the first one is a demo card.
fn markdown(html: &str, url: &Url) -> String {
    // Inside = at least half of Readability's words occur in the landmark; exact text differs (permalinks, spacing).
    let words = |s: &str| -> std::collections::HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() >= 3)
            .map(str::to_lowercase)
            .collect()
    };
    let doc = dom_query::Document::from(html);
    let land = doc.select("main, [role=main]").first();
    let landmark = land
        .exists()
        .then(|| (land.html().to_string(), words(&land.text())));
    let main = dom_smoothie::Readability::with_document(doc, Some(url.as_str()), None)
        .ok()
        .and_then(|mut r| r.parse().ok())
        .filter(|a| a.content.len() > 200)
        .filter(|a| {
            let mine = words(&a.text_content);
            landmark.as_ref().is_none_or(|(_, w)| {
                2 * mine.iter().filter(|x| w.contains(*x)).count() >= mine.len()
            })
        })
        .map(|a| a.content.to_string())
        .or_else(|| landmark.map(|(h, _)| h))
        .unwrap_or_else(|| html.to_string());
    htmd::HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "nav", "footer", "noscript", "svg"])
        .build()
        .convert(&main)
        .unwrap_or_default()
}

pub fn crawl(out: &Path, o: &Options, progress: &mut dyn FnMut(u64, u64)) -> Result<Value> {
    let start = normalize(Url::parse(o.start).context("start URL")?);
    if !matches!(start.scheme(), "http" | "https") {
        bail!("only http and https URLs can be crawled");
    }
    fs::create_dir_all(out)?;
    let agent = agent();
    // The scope follows a redirect to another host (vitejs.dev/guide/ answers from vite.dev/guide/; docs.pydantic.dev/latest/
    // from pydantic.dev/docs/validation/latest/) but not one within the site: tailwindcss.com/docs redirects to one page,
    // /docs/installation/using-vite, and the scope must stay /docs.
    let start = match get(&agent, &start) {
        Some((_, last, _)) if last.host_str() != start.host_str() => normalize(last),
        _ => start,
    };
    let robots = start
        .join("/robots.txt")
        .ok()
        .and_then(|u| get(&agent, &u))
        .filter(|(_, _, kind)| !kind.contains("html")) // some sites answer a missing robots.txt with an HTML page
        .and_then(|(txt, _, _)| texting_robots::Robot::new("FootnoteDocs", txt.as_bytes()).ok());
    let allowed = |u: &Url| robots.as_ref().is_none_or(|r| r.allowed(u.as_str()));

    // Discovery: llms.txt beside the start page and at the root, then the sitemap, then the start page itself.
    let mut seeds = Vec::new();
    let mut via_llms = 0;
    for llms in [start.join("llms.txt"), start.join("/llms.txt")]
        .into_iter()
        .flatten()
    {
        if let Some((txt, _, kind)) = get(&agent, &llms)
            && !kind.contains("html")
        {
            let found = links(&txt, &llms);
            via_llms += found.len();
            seeds.extend(found);
        }
    }
    let from_sitemap = sitemap(&agent, &start);
    let via_sitemap = from_sitemap.len();
    seeds.push(start.clone());
    seeds.extend(from_sitemap);

    let mut queue: VecDeque<Url> = VecDeque::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut written: HashSet<String> = HashSet::new(); // page keys already saved (llms.txt `.md` first, then HTML)
    for u in seeds {
        if in_scope(&u, &start, &o.exclude) && seen.insert(u.to_string()) {
            queue.push_back(u);
        }
    }
    let (mut pages, mut bytes, mut skipped, mut slowed) = (0usize, 0u64, 0usize, 0usize);
    let mut urls = serde_json::Map::new();
    // robots.txt Crawl-delay means one request at a time with that pause between them.
    let delay = robots
        .as_ref()
        .and_then(|r| r.delay)
        .map(|d| Duration::from_secs_f32(d.clamp(0.0, 60.0)));
    let mut level = if delay.is_some() {
        1
    } else {
        o.concurrency.clamp(1, MAX_CONCURRENCY)
    };
    while !queue.is_empty() && pages < o.max_pages {
        let mut batch = Vec::new();
        while batch.len() < level
            && let Some(u) = queue.pop_front()
        {
            if allowed(&u) {
                batch.push(u);
            } else {
                skipped += 1;
            }
        }
        let fetched: Vec<Fetch> = thread::scope(|s| {
            let jobs: Vec<_> = batch.iter().map(|u| s.spawn(|| fetch(&agent, u))).collect();
            jobs.into_iter()
                .map(|j| j.join().unwrap_or(Fetch::Failed))
                .collect()
        });
        let mut wait = None;
        for (u, f) in batch.into_iter().zip(fetched) {
            let (body, last, kind) = match f {
                Fetch::Page(body, last, kind) => (body, last, kind),
                Fetch::SlowDown(w) => {
                    slowed += 1;
                    wait = Some(wait.map_or(w, |x: Duration| x.max(w)));
                    queue.push_back(u); // asked again later, at a lower level
                    continue;
                }
                Fetch::Failed => {
                    skipped += 1;
                    continue;
                }
            };
            if pages >= o.max_pages {
                break;
            }
            let last = normalize(last);
            if last != u && (!in_scope(&last, &start, &o.exclude) || !seen.insert(last.to_string()))
            {
                continue; // a redirect out of scope or to a page already taken
            }
            let text = if kind.contains("html") {
                for l in links(&body, &last) {
                    if in_scope(&l, &start, &o.exclude) && seen.insert(l.to_string()) {
                        queue.push_back(l);
                    }
                }
                markdown(&body, &last)
            } else if kind.contains("markdown") || kind.contains("text/plain") {
                body
            } else {
                continue;
            };
            if text.trim().len() < 50 {
                skipped += 1; // empty main content: probably rendered by JavaScript
                continue;
            }
            if !written.insert(page_key(&last)) {
                continue; // the same page in another form (Markdown and HTML both listed)
            }
            pages += 1;
            bytes += text.len() as u64;
            let file = format!("{pages}.md");
            fs::write(out.join(&file), &text)?;
            urls.insert(file, json!(page_url(&last)));
            progress(pages as u64, bytes);
        }
        if let Some(w) = wait {
            level = (level / 2).max(1);
            thread::sleep(w);
        } else if let Some(d) = delay {
            thread::sleep(d);
        } else {
            level = (level + 1).min(MAX_CONCURRENCY);
        }
        if slowed > 50 && pages == 0 {
            bail!("the site keeps refusing requests (HTTP 429/503 or timeouts)");
        }
        if slowed > 500 {
            break; // keep what was fetched; the summary reports the slow-downs
        }
    }
    fs::write(out.join("urls.json"), serde_json::to_string_pretty(&urls)?)?;
    Ok(
        json!({"pages": pages, "bytes": bytes, "skipped": skipped, "queued_left": queue.len(),
        "discovered": {"llms_txt": via_llms, "sitemap": via_sitemap}, "out": out, "robots_txt": robots.is_some(),
        "concurrency": level, "slowed_down": slowed}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crawler_keeps_the_tls_stack_cloudflare_lets_through() {
        assert_eq!(TLS_PROVIDER, ureq::tls::TlsProvider::Rustls);
    }

    #[test]
    fn scope_keeps_the_start_folder_only() {
        let u = |s: &str| Url::parse(s).unwrap();
        let start = u("https://docs.python.org/3/");
        assert!(in_scope(
            &u("https://docs.python.org/3/library/os.html"),
            &start,
            &None
        ));
        assert!(!in_scope(
            &u("https://docs.python.org/3.11/"),
            &start,
            &None
        ));
        assert!(!in_scope(&u("https://example.com/3/x"), &start, &None));
        let page = u("https://vite.dev/guide/index.html");
        assert!(in_scope(
            &u("https://vite.dev/guide/features"),
            &page,
            &None
        ));
        let fr = Some(regex::Regex::new("/fr/").unwrap());
        assert!(!in_scope(
            &u("https://docs.python.org/3/fr/x.html"),
            &start,
            &fr
        ));
    }

    #[test]
    fn markdown_copies_cite_their_page() {
        let p = |s: &str| page_url(&Url::parse(s).unwrap());
        assert_eq!(
            p("https://vite.dev/guide/features.md"),
            "https://vite.dev/guide/features"
        );
        assert_eq!(
            p("https://pydantic.dev/docs/v/concepts/config/index.md"),
            "https://pydantic.dev/docs/v/concepts/config/"
        );
        assert_eq!(
            p("https://docs.python.org/3/library/os.html"),
            "https://docs.python.org/3/library/os.html"
        );
    }

    #[test]
    fn a_short_page_beats_a_long_footer() {
        let footer = "<p>Copyright 2001, Python Software Foundation, licensed under the PSF License, version 2, with examples, \
                      recipes, and other code under the Zero Clause BSD License, see History and License.</p>"
            .repeat(5);
        let html = format!(
            "<html><body><div class=\"document\"><div class=\"body\" role=\"main\"><h1>nis</h1><p>This module is no \
             longer part of the standard library. It was removed in Python 3.13.</p></div></div>\
             <div class=\"footer\">{footer}</div></body></html>"
        );
        let md = markdown(
            &html,
            &Url::parse("https://docs.python.org/3/library/nis.html").unwrap(),
        );
        assert!(
            md.contains("removed in Python 3.13") && !md.contains("Copyright"),
            "{md}"
        );
    }

    #[test]
    fn page_forms_share_one_key() {
        let k = |s: &str| page_key(&Url::parse(s).unwrap());
        let a = k("https://vite.dev/guide/features");
        for other in [
            "https://vite.dev/guide/features.md",
            "https://vite.dev/guide/features/",
            "https://vite.dev/guide/features.html",
        ] {
            assert_eq!(k(other), a);
        }
        assert_eq!(
            k("https://vite.dev/guide/index.html"),
            k("https://vite.dev/guide/")
        );
        assert_ne!(k("https://vite.dev/guide/cli"), a);
    }

    #[test]
    fn links_from_html_and_llms_txt() {
        let base = Url::parse("https://vite.dev/guide/").unwrap();
        let found = links(
            r#"<a href="features#x">a</a> [b](/config/) [c](https://vite.dev/guide/cli.md)"#,
            &base,
        );
        let found: Vec<_> = found.iter().map(Url::as_str).collect();
        assert_eq!(
            found,
            [
                "https://vite.dev/guide/features",
                "https://vite.dev/config/",
                "https://vite.dev/guide/cli.md"
            ]
        );
    }
}
