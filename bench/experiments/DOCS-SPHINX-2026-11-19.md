# Documentation search rematch on Sphinx docs (Flask, Python)

Written 19 November 2026, before any system ran on the development libraries. Issue #106, epic #84. Follows
[DOCS-OPTIMIZE-2026-11-16.md](DOCS-OPTIMIZE-2026-11-16.md), whose sealed final lost on Flask (Cloudflare blocked Footnote's crawl after
Grounded Docs had scraped the site first, and on Grounded Docs' own Flask pages Footnote still trailed by 0.073) and said Sphinx sites
need a dev-set look before the next final.

## Sets

- **Development (new):** `bench/datasets/docs-sphinx-dev/dataset.yaml`: 40 questions on three Sphinx sites with three themes: Django 5.2
  (custom theme, 16), Jinja (Pallets theme like Flask, 12), Sphinx's own docs (12); 48 judged URLs, each checked to answer HTTP 200;
  recipe `recipe.json` (200 pages each, `llms-*.txt` excluded). Written by Claude from the sites' page lists before any system ran on them.
  Tuning is allowed here. Flask and Python are not in it: their questions are sealed.
- **Seen Sphinx evidence (not rerun):** Flask's 20 questions of docs-heldout3 (spent), Python's questions in Grounded Docs' set. Reported
  from the earlier notes only.
- **Sealed final (written after the development work is closed, committed with its SHA-256 before any system runs on it):** fresh
  questions on Flask (Pallets theme, Cloudflare) and Python (docs.python.org, python-docs-theme, 500 pages).

## Systems, as in DOCS-CONFIRM

- **Grounded Docs 3.4.1**, hybrid, EmbeddingGemma 300M Q8_0 through llama.cpp, telemetry off, its own scrape.
- **Footnote**, shipped model pair, hybrid, `--limit 5 --per-source 1`, its own crawl (`footnote crawl`), one frozen binary per phase
  (SHA-256 recorded in the results).

## Harness

`bench/experiments/docs_rematch.py` (new; reuses `docs_vs_grounded.py` and `docs_crawl.py` unchanged): per library one system crawls
first, then a cool-down (default 600 s), then the other; the first system **alternates between libraries** (the lesson of the Flask
block). Then Footnote's own crawl is indexed and searched twice (rankings must be identical), Footnote is also run on the pages
Grounded Docs stored (same text, to separate crawl from ranking), and per-query latency is timed one query at a time with the two
CLIs alternating, the way the harness invokes them (Grounded Docs through `npx`).

## Rule (same as DOCS-CONFIRM / DOCS-OPTIMIZE)

- **Primary:** Footnote (own crawl) minus Grounded Docs (own scrape), nDCG@5 with Grounded Docs' metric code, paired bootstrap over the
  questions (10,000 resamples, seed 20261119 on development, 20261120 on the sealed set). **Equivalent:** 95 % lower bound above −0.05.
  **Better:** above 0. Otherwise **worse**.
- Reported, not judged: Recall@5, Hit@3, MRR, per library, same-text run, crawl and index seconds, per-query latency, crawl coverage
  of the judged URLs.
- **Development phase:** if Footnote is not at least equivalent, read the misses, then test candidate changes against Footnote's own
  previous binary only (never rerun Grounded Docs per candidate). A candidate is kept when mean nDCG@5 rises on the development
  questions and no library drops by more than 0.01, and the general guard (the held-out Alloprof and mix-fr-en benches) drops by no more
  than 0.005 nDCG@10. If no candidate is kept, the sealed final runs on the frozen binary.
- **Sealed final, once:** both systems on the sealed Flask and Python questions, alternating crawl order (Flask: Footnote first;
  Python: Grounded Docs first), cool-down 600 s between systems on one host. A crawl blocked by a host (HTTP 429/Cloudflare challenge)
  is reported as blocked, not retried into the score; coverage of judged URLs is reported.
- Machine load is sampled before every run and reported; other agent sessions share the machine, so seconds are indicative only.
- No new third-party library or tool: Grounded Docs 3.4.1 and llama.cpp b11901 were reviewed in earlier rounds.

## Development, round 1 (19 November, written before the next measurement)

Run `bench/work/sphinx/dev1..dev3` (local, not committed). Deviations, in order:

1. **Footnote's crawler could not fetch Read the Docs / Pallets sites at all** (Jinja, Requests; Flask's Nov 16 "block" is probably the
   same): Cloudflare answered every request from `footnote crawl` with a challenge (HTTP 429, `cf-mitigated: challenge`) while curl
   and Python with the same headers got 200. Isolated with a debug build: the Windows TLS stack (`native-tls`) is the trigger; the
   same binary with rustls (`ureq` features `rustls` + `platform-verifier`, system roots kept) gets 200 on Jinja, FastAPI, Python,
   Django and Flask. Fix kept (a defect, not a tuning): the crawler uses rustls. New crates reviewed, see the PR.
2. **Sphinx's own site (sphinx-doc.org, Cloudflare) answered Grounded Docs' scrape with 429s** after Footnote's 200-page crawl (11 s,
   no 429 for Footnote) and has kept challenging our IP since. Its 12 questions are dropped from the rematch (`dataset-core.yaml`,
   `recipe-core.json`: Django 16, Jinja 12); the full 40-question file stays. Lesson for the final: the second system on a
   Cloudflare host can be locked out by the first; order alternates, the cool-down is 600 s, and a blocked crawl is reported as such.
3. **Result on the core set with the fixed binary** (`dev3`): own crawl, Footnote − Grounded nDCG@5 −0.184 [−0.355, −0.014]
   (Django −0.406, Jinja +0.112); on Grounded Docs' stored pages (same text) Footnote − Grounded +0.069 (0.900 against 0.831).
   Ranking on Sphinx text is not the problem; the crawl is: judged-URL coverage on Django 0.435 against 1.000. The sitemap lists
   32,861 pages alphabetically and the crawler took the first 200 in that order (faq, howto, internals, ref), never reaching
   `topics/`.

**Candidate H (crawl order under a page cap), written before measuring it:** sitemap pages are queued shallowest path first
(stable, so equal depth keeps sitemap order). **Keep H** when, on the development libraries, judged-URL coverage rises on Django and
falls on none, and end-to-end nDCG@5 rises (paired, against the same binary without H); the Django, Jinja, FastAPI, Python, React,
Vite, Tailwind, Pydantic and Svelte start pages are crawled with and without H and coverage compared where the earlier question
sets' recipes allow (Grounded Docs' stored sets are gone with their work folders, so only the Sphinx development libraries are measured;
the general guard is the unchanged search code).

**Result of H (`dev4`, same core set, 60 s cool-down):** judged-URL coverage on Django **fell** from 0.435 to 0.217 (Jinja 1.000
unchanged) and end-to-end nDCG@5 against Grounded Docs fell from −0.184 to −0.309 [−0.493, −0.120]. Keep rule failed: **H rejected and
removed**. Reading: Django's sitemap is flat, so depth does not separate the pages that judges point to; no sitemap-order heuristic is
adopted. The crawl gap on very large sites under a 200-page cap is reported as a finding (Footnote follows the sitemap's order; Grounded
follows links from the start page, breadth first), not tuned further on the development set.

## Sealed final (written before any system ran on Flask or Python)

Dev closed after round 1 (H rejected). Sealed set `bench/datasets/docs-sphinx-sealed/dataset.yaml` (sha256
`2e06dc864bfd4b345c78f25bb18ad89194400f1da1e28146e7dc50081a9d113a`), recipe `recipe.json` beside it. Flask 20 and Python 20
questions, seed 20261120, hybrid `--limit 5 --per-source 1`, Footnote with the TLS fix (rustls) and no ordering change. Crawl order
alternates (Flask: Footnote first, Python: Grounded first), 600 s cool-down. Decision rule as in the protocol above: better if the 95%
paired lower bound > 0, equivalent if > −0.05, otherwise worse; a crawl that a host blocks is reported as blocked, not retried into the score.
Sphinx's own site (12 dev questions) stays out of the rematch because its host blocks Grounded Docs' scraper from this IP.
- final.exe sha256 7a2879b40eb4c012067f2c80d8c8105c73a4dcde70550b461f54abcc82df5e0b (frozen for the sealed run)
