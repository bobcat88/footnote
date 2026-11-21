//! Documentation crawler (backlog item 14; study: docs/research/DOCS-INGESTION-STUDY.md). Cheapest discovery first: the site's
//! `llms.txt`, its sitemap, then links within the start URL's path. Each page's main content (Readability) becomes Markdown in
//! `<out>/<n>.md`; `<out>/urls.json` maps every file to its URL. Polite by construction: robots.txt (rules and `Crawl-delay`),
//! an identifying user agent, and adaptive concurrency per host: a few requests in flight, more while the site answers, half as
//! many after a 429, a 503 or a timeout (`Retry-After` honoured). JavaScript is not run.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
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

/// What a page looked like when it was fetched: the cheap change check of `docs refresh` compares against it.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct Validators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    /// sha256 of the fetched body, for servers that send neither header.
    pub hash: String,
}

enum Fetch {
    /// Body, final URL after redirects, content type, validators.
    Page(String, Url, String, Validators),
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
        Some((body, last, kind, v)) => Fetch::Page(body, last, kind, v),
        None => Fetch::Failed,
    }
}

/// Body and final URL (after redirects) of a successful text response.
fn get(agent: &ureq::Agent, url: &Url) -> Option<(String, Url, String)> {
    match fetch(agent, url) {
        Fetch::Page(body, last, kind, _) => Some((body, last, kind)),
        _ => None,
    }
}

fn page(
    r: &mut ureq::http::Response<ureq::Body>,
    url: &Url,
) -> Option<(String, Url, String, Validators)> {
    if !r.status().is_success() {
        return None;
    }
    let header = |r: &ureq::http::Response<ureq::Body>, k: &str| {
        r.headers()
            .get(k)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    let (etag, last_modified) = (header(r, "etag"), header(r, "last-modified"));
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
    let hash = footnote_core::hash(body.as_bytes());
    Some((
        body,
        last,
        kind,
        Validators {
            etag,
            last_modified,
            hash,
        },
    ))
}

/// The start URL's folder: "/3/" and "/docs" keep theirs; "/guide/index.html" keeps "/guide".
fn scope_dir(start: &Url) -> &str {
    let p = start.path();
    let last = p.rsplit('/').next().unwrap_or("");
    if p.ends_with('/') || !last.contains('.') {
        p.trim_end_matches('/')
    } else {
        p.rsplit_once('/').map_or("", |(d, _)| d)
    }
}

/// Scope of a crawl whose start URL answered from `last`. Within the site the original URL stays the scope (tailwindcss.com/docs
/// redirects to one page, /docs/installation/using-vite, and the scope must stay /docs). Another host is a move of the site
/// (vitejs.dev/guide/ to vite.dev/guide/; docs.pydantic.dev/latest/ to pydantic.dev/docs/validation/latest/get-started/): the
/// scope is the original folder as it sits in the target's path (/docs/validation/latest), or the target's own folder when the
/// original folder does not appear there.
fn redirect_scope(start: &Url, last: &Url) -> Url {
    if last.host_str() == start.host_str() {
        return start.clone();
    }
    let dir = scope_dir(start);
    let mut scope = last.clone();
    if let Some(at) = last.path().find(&format!("{dir}/")) {
        scope.set_path(&format!("{}/", &last.path()[..at + dir.len()]).replace("//", "/"));
    }
    scope
}

/// Same host, under the start path, http(s), no fragment, not excluded.
fn in_scope(u: &Url, start: &Url, exclude: &Option<regex::Regex>) -> bool {
    let dir = scope_dir(start);
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

/// Next page to fetch: the sitemap's order and the links found on fetched pages take turns, so a page cap is spent on both. The
/// sitemap alone can fill it with an alphabetical corner of a big site (Django: 0.435 of the judged pages in 200); links alone
/// miss pages nothing links to (DOCS-SPHINX-CRAWL-2026-11-19.md).
fn next_url(
    listed: &mut VecDeque<Url>,
    linked: &mut VecDeque<Url>,
    turn: &mut usize,
) -> Option<Url> {
    *turn += 1;
    if (*turn).is_multiple_of(2) {
        linked.pop_front().or_else(|| listed.pop_front())
    } else {
        listed.pop_front().or_else(|| linked.pop_front())
    }
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

/// A Sphinx site's `objects.inv` beside its start page, with the folder its URIs are relative to. Fetched like the sitemap.
fn inventory(agent: &ureq::Agent, scope: &Url) -> Option<(Vec<crate::miss::Object>, Url)> {
    let mut base = normalize(scope.clone());
    base.set_path(&format!("{}/", scope_dir(scope)));
    let inv = base.join("objects.inv").ok()?;
    let mut r = agent.get(inv.as_str()).call().ok()?;
    if !r.status().is_success() {
        return None;
    }
    let bytes = r
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .ok()?;
    crate::miss::parse(&bytes).map(|o| (o, base))
}

fn robots(agent: &ureq::Agent, site: &Url) -> Option<texting_robots::Robot> {
    site.join("/robots.txt")
        .ok()
        .and_then(|u| get(agent, &u))
        .filter(|(_, _, kind)| !kind.contains("html")) // some sites answer a missing robots.txt with an HTML page
        .and_then(|(txt, _, _)| texting_robots::Robot::new("FootnoteDocs", txt.as_bytes()).ok())
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
    let mut landing = None; // the page the start URL redirected to: it may sit outside a folder that has no page of its own
    let start = match get(&agent, &start) {
        Some((_, last, _)) => {
            let last = normalize(last);
            landing = Some(last.clone());
            redirect_scope(&start, &last)
        }
        None => start,
    };
    let robots = robots(&agent, &start);
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
    seeds.extend(landing);
    seeds.push(start.clone());
    seeds.extend(from_sitemap);
    // Sphinx: every documented object's page, so API pages nothing links to are still crawled.
    if let Some((objects, base)) = inventory(&agent, &start) {
        seeds.extend(crate::miss::pages(
            objects.iter().map(|o| o.uri.as_str()),
            &base,
        ));
    }

    let mut queue: VecDeque<Url> = VecDeque::new();
    let mut linked: VecDeque<Url> = VecDeque::new(); // links found on fetched pages
    let mut turn = 0usize;
    let mut linked_seen: HashSet<String> = HashSet::new();
    let mut taken: HashSet<String> = HashSet::new(); // fetched or being fetched (a page can sit in both queues)
    let mut seen: HashSet<String> = HashSet::new();
    let mut written: HashSet<String> = HashSet::new(); // page keys already saved (llms.txt `.md` first, then HTML)
    for u in seeds {
        if in_scope(&u, &start, &o.exclude) && seen.insert(u.to_string()) {
            queue.push_back(u);
        }
    }
    let (mut pages, mut bytes, mut skipped, mut slowed) = (0usize, 0u64, 0usize, 0usize);
    let mut urls = serde_json::Map::new();
    let mut meta = serde_json::Map::new(); // file -> fetched URL and validators, for `docs refresh`
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
    while !(queue.is_empty() && linked.is_empty()) && pages < o.max_pages {
        let mut batch = Vec::new();
        while batch.len() < level
            && let Some(u) = next_url(&mut queue, &mut linked, &mut turn)
        {
            if !taken.insert(u.to_string()) {
                continue;
            }
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
            let (body, last, kind, validators) = match f {
                Fetch::Page(body, last, kind, v) => (body, last, kind, v),
                Fetch::SlowDown(w) => {
                    slowed += 1;
                    wait = Some(wait.map_or(w, |x: Duration| x.max(w)));
                    taken.remove(u.as_str());
                    queue.push_front(u); // asked again later, at a lower level
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
                    // a link to a page the sitemap lists is queued again, so it is fetched early
                    if in_scope(&l, &start, &o.exclude)
                        && !taken.contains(l.as_str())
                        && linked_seen.insert(l.to_string())
                    {
                        seen.insert(l.to_string());
                        linked.push_back(l);
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
            meta.insert(
                file.clone(),
                json!({"url": last.as_str(), "validators": validators}),
            );
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
    fs::write(
        out.join("refresh.json"),
        serde_json::to_string_pretty(
            &json!({"scope": start.as_str(), "max_pages": o.max_pages, "pages": meta}),
        )?,
    )?;
    Ok(
        json!({"pages": pages, "bytes": bytes, "skipped": skipped, "queued_left": queue.len() + linked.len(),
        "discovered": {"llms_txt": via_llms, "sitemap": via_sitemap}, "out": out, "robots_txt": robots.is_some(),
        "concurrency": level, "slowed_down": slowed}),
    )
}

/// A page seen again: what `docs refresh` learns per known page.
#[non_exhaustive]
pub enum Recheck {
    /// Not modified (HTTP 304) or the same body; the validators may have been renewed.
    Same(Validators),
    /// A new body: its Markdown, validators, and the links on it.
    Changed(String, Validators, Vec<Url>),
    /// The site says the page no longer exists (404, 410).
    Gone,
    /// No answer worth acting on (network error, 429/503, not a page): keep what we have.
    Unreachable,
}

/// A page that was not in the index before.
pub struct Found {
    pub url: Url,
    pub text: String,
    pub validators: Validators,
}

/// The crawler's HTTP client for refreshes: conditional requests per page, then link and sitemap discovery of new pages.
pub struct Client(ureq::Agent);

impl Default for Client {
    fn default() -> Self {
        Client(agent())
    }
}

/// A fetched body as indexable text, with the links it holds; `None` for anything that is not a page.
fn text_of(body: String, last: &Url, kind: &str) -> Option<(String, Vec<Url>)> {
    let (text, found) = if kind.contains("html") {
        (markdown(&body, last), links(&body, last))
    } else if kind.contains("markdown") || kind.contains("text/plain") {
        (body, Vec::new())
    } else {
        return None;
    };
    (text.trim().len() >= 50).then_some((text, found))
}

impl Client {
    /// One conditional GET (`If-None-Match`, `If-Modified-Since`); a 200 whose body hashes the same is also `Same`.
    #[must_use]
    pub fn recheck(&self, url: &str, v: &Validators) -> Recheck {
        let Ok(u) = Url::parse(url) else {
            return Recheck::Unreachable;
        };
        let mut req = self.0.get(u.as_str());
        if let Some(e) = &v.etag {
            req = req.header("If-None-Match", e);
        }
        if let Some(m) = &v.last_modified {
            req = req.header("If-Modified-Since", m);
        }
        let Ok(mut r) = req.call() else {
            return Recheck::Unreachable;
        };
        match r.status().as_u16() {
            304 => Recheck::Same(v.clone()),
            404 | 410 => Recheck::Gone,
            _ => match page(&mut r, &u) {
                Some((_, _, _, nv)) if nv.hash == v.hash => Recheck::Same(nv),
                Some((body, last, kind, nv)) => match text_of(body, &normalize(last), &kind) {
                    Some((text, found)) => Recheck::Changed(text, nv, found),
                    None => Recheck::Unreachable,
                },
                None => Recheck::Unreachable,
            },
        }
    }

    /// Pages in scope that `known` (page keys, see `key_of`) lacks: `seeds` (links from changed pages) and the sitemap, then
    /// links of the new pages themselves, at most `room` of them, robots.txt honoured. One request at a time.
    /// shortcut: a listed page that turns out empty is fetched again on every refresh; remember rejects if that costs too much.
    pub fn find_new(
        &self,
        scope: &str,
        known: &HashSet<String>,
        seeds: Vec<Url>,
        room: usize,
    ) -> Vec<Found> {
        let Ok(scope) = Url::parse(scope) else {
            return Vec::new();
        };
        let queue = seeds.into_iter().chain(sitemap(&self.0, &scope)).collect();
        self.fetch_new(&scope, known, queue, room, true)
    }

    /// Fetch on miss: the (at most `cap`) pages most likely to answer `query` that `known` lacks, fetched live. Candidates are
    /// the pages of the objects the query names in the site's `objects.inv`, then sitemap pages whose path holds query terms.
    /// Nothing outside `scope` is requested, robots.txt is honoured, links on the fetched pages are not followed.
    pub fn fetch_on_miss(
        &self,
        scope: &str,
        query: &str,
        known: &HashSet<String>,
        cap: usize,
    ) -> Vec<Found> {
        let Ok(scope) = Url::parse(scope) else {
            return Vec::new();
        };
        let mut queue: VecDeque<Url> = VecDeque::new();
        if let Some((objects, base)) = inventory(&self.0, &scope) {
            queue.extend(crate::miss::resolve(&objects, query, &base, cap * 4));
        }
        queue.extend(crate::miss::rank_urls(
            &sitemap(&self.0, &scope),
            query,
            cap * 4,
        ));
        self.fetch_new(&scope, known, queue, cap, false)
    }

    fn fetch_new(
        &self,
        scope: &Url,
        known: &HashSet<String>,
        mut queue: VecDeque<Url>,
        room: usize,
        expand: bool,
    ) -> Vec<Found> {
        let robots = robots(&self.0, scope);
        let mut seen: HashSet<String> = known.clone();
        let mut out = Vec::new();
        while out.len() < room
            && let Some(u) = queue.pop_front()
        {
            let u = normalize(u);
            if !in_scope(&u, scope, &None)
                || !seen.insert(page_key(&u))
                || robots.as_ref().is_some_and(|r| !r.allowed(u.as_str()))
            {
                continue;
            }
            let Some((body, last, kind)) = get(&self.0, &u) else {
                continue;
            };
            let last = normalize(last);
            if !in_scope(&last, scope, &None) || (last != u && !seen.insert(page_key(&last))) {
                continue;
            }
            let hash = footnote_core::hash(body.as_bytes());
            if let Some((text, found)) = text_of(body, &last, &kind) {
                if expand {
                    queue.extend(found);
                }
                out.push(Found {
                    url: last,
                    text,
                    validators: Validators {
                        hash,
                        ..Validators::default()
                    },
                });
            }
        }
        out
    }
}

/// The key a page URL is compared by during discovery.
pub fn key_of(url: &str) -> Option<String> {
    Url::parse(url).ok().map(|u| page_key(&u))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crawler_keeps_the_tls_stack_cloudflare_lets_through() {
        assert_eq!(TLS_PROVIDER, ureq::tls::TlsProvider::Rustls);
    }

    #[test]
    fn links_and_sitemap_take_turns() {
        let u = |s: &str| Url::parse(&format!("https://x.dev/{s}")).unwrap();
        let mut listed: VecDeque<Url> = ["a", "b", "c"].map(u).into();
        let mut linked: VecDeque<Url> = ["l1", "l2"].map(u).into();
        let mut turn = 0;
        let order: Vec<_> = std::iter::from_fn(|| next_url(&mut listed, &mut linked, &mut turn))
            .map(|x| x.path().to_string())
            .collect();
        assert_eq!(order, ["/a", "/l1", "/b", "/l2", "/c"]);
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
    fn redirect_scope_keeps_the_original_folder() {
        let u = |s: &str| Url::parse(s).unwrap();
        let r = |a: &str, b: &str| redirect_scope(&u(a), &u(b)).to_string();
        assert_eq!(
            r(
                "https://docs.pydantic.dev/latest/",
                "https://pydantic.dev/docs/validation/latest/get-started/"
            ),
            "https://pydantic.dev/docs/validation/latest/"
        );
        assert_eq!(
            r("https://vitejs.dev/guide/", "https://vite.dev/guide/"),
            "https://vite.dev/guide/"
        );
        assert_eq!(
            r("https://x.dev/", "https://y.dev/en/start/"),
            "https://y.dev/"
        );
        assert_eq!(
            r("https://x.dev/a/", "https://y.dev/b/c/"),
            "https://y.dev/b/c/"
        );
        // same site: the original scope stays
        assert_eq!(
            r(
                "https://tailwindcss.com/docs",
                "https://tailwindcss.com/docs/installation/using-vite"
            ),
            "https://tailwindcss.com/docs"
        );
    }

    /// The start page answers from another host (localhost to 127.0.0.1) with a redirect to a child page; its siblings are crawled.
    #[test]
    fn crawl_follows_a_redirect_to_a_child_page() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            for mut c in l.incoming().flatten() {
                let mut buf = [0u8; 2048];
                let n = c.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let text = "A documentation page with enough words to count as content. ".repeat(5);
                let head = |status: &str, extra: &str, len: usize| {
                    format!(
                        "HTTP/1.1 {status}\r\n{extra}Content-Length: {len}\r\nConnection: close\r\n\r\n"
                    )
                };
                let reply = match path.as_str() {
                    "/latest/" => head(
                        "301 Moved",
                        &format!("Location: http://127.0.0.1:{port}/docs/latest/start/\r\n"),
                        0,
                    ),
                    "/docs/latest/start/" | "/docs/latest/concepts/" | "/docs/latest/api/" => {
                        let body = format!(
                            "<html><body><main><p>{text}</p><a href=\"/docs/latest/concepts/\">c</a> <a href=\"/docs/latest/api/\">a</a> <a href=\"/docs/other/\">o</a></main></body></html>"
                        );
                        head("200 OK", "Content-Type: text/html\r\n", body.len()) + &body
                    }
                    _ => head("404 Not Found", "", 0),
                };
                let _ = c.write_all(reply.as_bytes());
            }
        });
        let out = std::env::temp_dir().join(format!("footnote-crawl-redirect-{port}"));
        let start = format!("http://localhost:{port}/latest/");
        let o = Options {
            start: &start,
            max_pages: 10,
            exclude: None,
            concurrency: 1,
        };
        let r = crawl(&out, &o, &mut |_, _| {}).unwrap();
        let _ = fs::remove_dir_all(&out);
        assert_eq!(r["pages"], 3, "{r}"); // start, concepts, api; /docs/other/ is outside the scope
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
