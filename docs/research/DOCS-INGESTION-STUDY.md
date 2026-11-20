# Technical documentation in Footnote Library: storage and crawler

15 November 2026. Backlog item 14. Desk study; the measurements are pre-registered in
[DOCS-CRAWL.md](../../bench/experiments/DOCS-CRAWL.md) and [DOCS-VS-GROUNDED.md](../../bench/experiments/DOCS-VS-GROUNDED.md).

## 1. Where documentation lives: one database per source and version

| Option | Gain | Cost |
|---|---|---|
| One global database (docs mixed with the user's own corpus) | One file, one search | A query about React's `useEffect` can return a Vite or Python page that shares words; updating one library means touching the shared index; nothing tells the agent which source a hit came from but the path |
| **One database per documentation source and version** (`react@19`, `python@3.13`) | Clean by construction: searching React searches only React; a version upgrade rebuilds one small file; removing a library deletes one file; shareable read-only across a team (documentation is not personal, unlike memory) | Searching everything at once needs the router |
| Both, through the router (recommended) | Per-library search when the agent knows the library (the usual case, as in Grounded Docs, which requires a library name); all sources at once when it does not | None measured |

The router already exists for team indexes ([FEDERATION-DESIGN.md](../FEDERATION-DESIGN.md)): when it shares word statistics
across the indexes it searches, nDCG@10 matched one big index exactly at 4, 16 and 48 indexes (lab test, mix-fr-en, 3,039
questions), at 0.74 to 1.34 times the query time of one index, gathering the statistics not included. Without shared statistics,
splitting by topic lost 0.021 to 0.028: exactly the documentation case, where each index holds one subject. So keeping documentation apart costs no search
quality, provided the router shares statistics.

Memory stays in its own database, as today. Documentation never enters memory: memory holds what the user and agents decided;
documentation is reference text with a source URL and a version.

## 2. The crawler, built in Rust

Pipeline, cheapest first (the order Grounded Docs also uses):
1. **`llms.txt` and `llms-full.txt`** at the site root: the site's own list of pages for language models, often with Markdown
   versions (`page.md`), no HTML cleaning needed.
2. **`sitemap.xml`**: the complete page list, with last-modified dates for refreshes.
3. **Link crawl** within the start URL's path, breadth first, with a page budget.
4. **JavaScript rendering**: not in the first version; the bench measures how many pages need it.

Politeness: robots.txt respected, a few requests at a time, a delay per host, an identifying user agent, conditional requests
(ETag, Last-Modified) on refresh, content hashes (as for files today).

Candidate crates (crates.io, checked 15 November 2026):

| Crate | Role | Licence | Latest | Notes |
|---|---|---|---|---|
| `ureq` | HTTP | MIT/Apache | 3.5 | Already in the tree; enough for a polite, sequential-per-host crawl |
| `spider` | Full crawler | MIT | 2.53.9 (Sep 2026) | Mature, many features (headless Chrome, concurrency); large; a bench candidate, not a default |
| `dom_smoothie` | Main-content extraction (Readability) | MIT | 0.19.0 (Oct 2026) | Removes navigation, sidebars, footers |
| `htmd` | HTML to Markdown | Apache-2.0 | 0.5.6 (Oct 2026) | 11 M downloads, active |
| `fast_html2md` | HTML to Markdown | MIT | 0.0.65 (Nov 2026) | From the spider authors; pre-1.0 |
| `html2md` | HTML to Markdown | **GPL-3.0+** | 0.2.17 | **Excluded**: licence |
| `texting_robots` | robots.txt | MIT/Apache | 0.2.2 (2023) | Small and stable; unmaintained since 2023 |
| `sitemap` | sitemap.xml | MIT | 0.4.1 (2020) | Old; parsing a sitemap is a few lines of XML, so likely not needed |

Smallest working design to test first: `ureq` + `llms.txt`/sitemap discovery + `dom_smoothie` + `htmd`, about two new
dependencies. `spider` is benched against it; it wins only if it finds clearly more of the relevant pages.

## 3. What the benches decide

- **A. Search on identical text** (DOCS-VS-GROUNDED.md, written earlier): is Footnote's search at least equivalent to Grounded
  Docs' on the same pages?
- **B. Crawl coverage and C. end to end** (DOCS-CRAWL.md): which crawler finds the pages that answer the questions, and does
  crawl + chunking + search together match or beat Grounded Docs?

Grounded Docs runs through `npx @arabold/docs-mcp-server@3.4.1` (security review done 15 November: its own install script only
prints a message; `better-sqlite3`, `sqlite-vec` and `tree-sitter` fetch or build native binaries; telemetry off with
`DOCS_MCP_TELEMETRY=false` and `--no-telemetry`). Running it requires explicit approval.
