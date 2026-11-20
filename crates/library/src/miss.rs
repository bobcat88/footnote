//! Fetch on miss (#301): when a search over a cached documentation site comes back weak, the pages most likely to hold the
//! answer are fetched live. This file is the pure part: the weak-result rule, the Sphinx `objects.inv` parser and the ranking of
//! candidate pages. Fetching is `crawl::Client::fetch_on_miss`.
use std::collections::HashSet;
use std::io::Read;
use url::Url;

/// Largest inventory after decompression (a zip bomb guard; numpy's is about 2 MB).
const MAX_INV: u64 = 64 * 1024 * 1024;
/// Words that carry no topic; a query made only of them has no terms and is never "weak".
const STOP: &[&str] = &[
    "the", "and", "for", "how", "what", "does", "with", "that", "this", "from", "can", "are",
    "use", "using", "when", "which", "why", "its", "into", "not", "you", "your", "get", "set",
];

/// Topic words of a query: lowercase alphanumeric runs of 3 or more characters, minus `STOP`. `requests.Session.mount` gives
/// `requests`, `session`, `mount`.
pub fn terms(query: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() >= 3 && !STOP.contains(&w.as_str()) && seen.insert(w.clone()))
        .collect()
}

/// Weak-result rule, deterministic and independent of the ranker's score scale (lexical, hybrid and reranked scores differ, and
/// the lexical ones are not returned). Of the first `TOP` hits take the one holding the most query terms in its title, path
/// and text (case-insensitive substring); the result is weak when there is no hit, or when that hit holds fewer than
/// `COVERAGE` of the terms. A strong result therefore has one passage that talks about (nearly) everything asked.
pub const TOP: usize = 3;
pub const COVERAGE: f64 = 0.6;

pub fn is_weak<'a>(query: &str, hits: impl IntoIterator<Item = &'a str>) -> bool {
    let want = terms(query);
    if want.is_empty() {
        return false;
    }
    let best = hits
        .into_iter()
        .take(TOP)
        .map(|h| {
            let h = h.to_lowercase();
            want.iter().filter(|t| h.contains(t.as_str())).count()
        })
        .max()
        .unwrap_or(0);
    (best as f64) < COVERAGE * want.len() as f64
}

/// One documented object of a Sphinx inventory.
#[derive(Debug, PartialEq)]
pub struct Object {
    pub name: String,
    /// Priority: 0 important, 1 normal, 2 unimportant (hidden objects, -1, are dropped).
    pub priority: i64,
    /// Relative to the inventory's folder; may carry `#anchor`.
    pub uri: String,
}

/// Parse an `objects.inv` (version 2): four text header lines, then zlib-compressed lines `name domain:role priority uri
/// dispname`; a `uri` ending in `$` stands for the name appended. `None` for anything else (version 1 is not supported).
pub fn parse(bytes: &[u8]) -> Option<Vec<Object>> {
    let mut rest = bytes;
    let mut first = &bytes[..0];
    for i in 0..4 {
        let at = rest.iter().position(|&b| b == b'\n')?;
        if i == 0 {
            first = &rest[..at];
        }
        rest = &rest[at + 1..];
    }
    if !first.starts_with(b"# Sphinx inventory version 2") {
        return None;
    }
    let mut text = String::new();
    flate2::read::ZlibDecoder::new(rest)
        .take(MAX_INV)
        .read_to_string(&mut text)
        .ok()?;
    static LINE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = LINE.get_or_init(|| {
        regex::Regex::new(r"^(.+?)\s+(\S+)\s+(-?\d+)\s+(\S*)\s+(.*)$").expect("valid pattern")
    });
    Some(
        text.lines()
            .filter_map(|l| {
                let c = re.captures(l)?;
                let priority: i64 = c[3].parse().ok()?;
                let uri = c[4]
                    .strip_suffix('$')
                    .map_or(c[4].to_string(), |u| format!("{u}{}", &c[1]));
                (priority >= 0 && !uri.is_empty()).then(|| Object {
                    name: c[1].to_string(),
                    priority,
                    uri,
                })
            })
            .collect(),
    )
}

/// Pages (fragment dropped, deduplicated, inventory order) of the given URIs resolved against `base`.
pub fn pages<'a>(uris: impl IntoIterator<Item = &'a str>, base: &Url) -> Vec<Url> {
    let mut seen = HashSet::new();
    uris.into_iter()
        .filter_map(|u| base.join(u).ok())
        .map(|mut u| {
            u.set_fragment(None);
            u
        })
        .filter(|u| seen.insert(u.to_string()))
        .collect()
}

/// Pages of the objects the query names, best first, at most `cap`. A query word equal to an object's full name
/// (`requests.Session.mount`) ranks first, one that is the tail of a name (`Session.mount`) next, then objects whose name parts
/// hold at least half of the query's terms, by how many.
pub fn resolve(objects: &[Object], query: &str, base: &Url, cap: usize) -> Vec<Url> {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let want = terms(query);
    let mut scored: Vec<(usize, i64, usize)> = Vec::new(); // (score, priority, index)
    for (i, o) in objects.iter().enumerate() {
        let name = o.name.to_lowercase();
        let score = if words.contains(&name) {
            10_000
        } else if words
            .iter()
            .any(|w| w.contains('.') && name.ends_with(&format!(".{w}")))
        {
            5_000
        } else {
            let parts: HashSet<&str> = name.split(|c: char| !c.is_alphanumeric()).collect();
            let hit = want.iter().filter(|t| parts.contains(t.as_str())).count();
            if hit == 0 || hit * 2 < want.len() {
                0
            } else {
                hit
            }
        };
        if score > 0 {
            scored.push((score, o.priority, i));
        }
    }
    scored.sort_by_key(|&(s, p, i)| (std::cmp::Reverse(s), p, i));
    pages(
        scored.iter().map(|&(_, _, i)| objects[i].uri.as_str()),
        base,
    )
    .into_iter()
    .take(cap)
    .collect()
}

/// Sitemap pages whose path holds query terms, best first, at most `cap`: the fallback when the site has no inventory.
pub fn rank_urls(urls: &[Url], query: &str, cap: usize) -> Vec<Url> {
    let want = terms(query);
    let mut scored: Vec<(usize, usize)> = urls
        .iter()
        .enumerate()
        .map(|(i, u)| {
            let p = u.path().to_lowercase();
            (want.iter().filter(|t| p.contains(t.as_str())).count(), i)
        })
        .filter(|&(n, _)| n > 0 && n * 2 >= want.len())
        .collect();
    scored.sort_by_key(|&(n, i)| (std::cmp::Reverse(n), i));
    scored
        .into_iter()
        .take(cap)
        .map(|(_, i)| urls[i].clone())
        .collect()
}

/// An `objects.inv` as a test fixture.
pub fn inventory_fixture(lines: &str) -> Vec<u8> {
    use std::io::Write;
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(lines.as_bytes()).expect("in memory");
    let mut out = b"# Sphinx inventory version 2\n# Project: demo\n# Version: 1.0\n# The remainder of this file is compressed using zlib.\n".to_vec();
    out.extend(z.finish().expect("in memory"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const INV: &str = "requests.Session py:class 1 api/#$ -\nrequests.Session.mount py:method 1 api/#$ -\n\
                       requests.get py:function 1 api/#$ -\nquickstart std:doc -1 user/quickstart/ Quickstart\n\
                       Making a Request std:label 1 user/quickstart/#make-a-request Make a request\n\
                       adapters std:label 2 user/advanced/#transport-adapters Transport Adapters\n";

    #[test]
    fn weak_rule_needs_one_top_passage_holding_most_terms() {
        assert!(is_weak("mount adapter prefix", []));
        assert!(
            is_weak(
                "mount adapter prefix",
                [
                    "session pooling",
                    "cookies",
                    "retries",
                    "mount adapter prefix"
                ]
            ),
            "4th hit is not looked at"
        );
        assert!(!is_weak(
            "mount adapter prefix",
            ["unrelated", "Session.mount(prefix, adapter)"]
        ));
        assert!(
            !is_weak("how to the", []),
            "no topic terms, nothing to judge"
        );
    }

    #[test]
    fn inventory_is_parsed_and_a_symbol_resolves_to_its_page() {
        let objs = parse(&inventory_fixture(INV)).unwrap();
        assert_eq!(objs.len(), 5, "the -1 object is hidden");
        assert_eq!(
            objs[1],
            Object {
                name: "requests.Session.mount".into(),
                priority: 1,
                uri: "api/#requests.Session.mount".into()
            }
        );
        let base = Url::parse("https://docs.example.dev/en/latest/").unwrap();
        let got = |q: &str| {
            resolve(&objs, q, &base, 3)
                .iter()
                .map(Url::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            got("requests.Session.mount"),
            ["https://docs.example.dev/en/latest/api/"]
        );
        assert_eq!(
            got("how does Session.mount work?"),
            ["https://docs.example.dev/en/latest/api/"]
        );
        assert_eq!(
            got("transport adapters"),
            ["https://docs.example.dev/en/latest/user/advanced/"]
        );
        assert!(got("pagination cursor").is_empty());
        assert_eq!(pages(objs.iter().map(|o| o.uri.as_str()), &base).len(), 3);
        assert!(parse(b"not an inventory\n\n\n\nxx").is_none());
    }

    #[test]
    fn sitemap_fallback_ranks_by_path_terms() {
        let u: Vec<Url> = ["a/index", "a/timeouts", "a/retry-timeouts"]
            .map(|p| Url::parse(&format!("https://x.dev/{p}")).unwrap())
            .into();
        let r = rank_urls(&u, "retry timeouts", 5);
        assert_eq!(r[0], u[2]);
        assert_eq!(r.len(), 2);
    }
}
