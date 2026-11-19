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
