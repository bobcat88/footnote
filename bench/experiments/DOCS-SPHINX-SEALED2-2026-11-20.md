# Sealed final 2: Footnote against Grounded Docs on Sphinx documentation (Scrapy, pytest)

Written 20 November 2026, before any system ran on these sites. Issue #301, follows
[DOCS-SPHINX-RESULTS-2026-11-19.md](DOCS-SPHINX-RESULTS-2026-11-19.md) (sealed Flask/Python: inconclusive on quality) and
[DOCS-SPHINX-CRAWL-RESULTS-2026-11-19.md](DOCS-SPHINX-CRAWL-RESULTS-2026-11-19.md) (link-following crawl order, PR #300, tuned on Django and
Jinja). This is the run that can turn the Sphinx result into a claim, so nothing below is changed after the run starts.

## Set

`bench/datasets/docs-sphinx-sealed2/dataset.yaml`, recipe `recipe.json` beside it. Two Sphinx-built sites not used in any earlier
set: **Scrapy** (docs.scrapy.org/en/latest, sphinx_rtd_theme, 150 sitemap pages) and **pytest** (docs.pytest.org/en/stable, furo theme,
its sitemap lists versions, not pages). 20 questions each, 40 in all, 45 distinct judged URLs, every one fetched with HTTP 200 and checked
to be the page that answers the question. The questions were written by Claude from the sites' page lists and page text; neither engine
had run on either site, and no result of either was seen. Never tuned on.

- dataset.yaml sha256 `b5a53b5c9df3c1d251fd839e461727d07775eff9b6875e350e307ca47f932483`
- recipe.json sha256 `25822c90354c47990b2dee89d0d017ccb2f24c52e22160b6426e09fc8c882551` (cap 200 pages per site, `llms-*.txt` excluded, as in the earlier recipes)
- Not used: Flask, Python, Django, Jinja (earlier sets); Sphinx's own site (blocks Grounded Docs from this IP).

## Systems and harness (unchanged from DOCS-SPHINX-2026-11-19.md)

- **Grounded Docs 3.4.1**, hybrid, EmbeddingGemma-300M Q8_0 through llama.cpp b11901 on the GPU, telemetry off, its own scrape.
- **Footnote** built from main at `77ea5c2` (includes PR #300), shipped model pair, hybrid, `--limit 5 --per-source 1`, its own crawl.
  Binary `bench/work/sphinx2/final.exe` sha256 `1e14802ff6bf3dbb3a096cab0ff6e7d5d8aabbe9549fa99c7871e8cd2468f4d2`, frozen for the run.
- Harness `bench/experiments/docs_rematch.py`, unchanged: own crawl indexed and searched twice (rankings must be identical), Footnote
  also on the pages Grounded Docs stored (same text), per-query latency one query at a time with the two CLIs alternating.

## Run rules (fixed now)

- Run once. Crawl order alternates: **Scrapy: Footnote first; pytest: Grounded Docs first**. Cool-down 600 s between systems on one host.
  Bootstrap seed **20261121**, 10,000 resamples. Live machine load is sampled before the run (Process Lasso ignored) and reported; no other
  benchmark or heavy job runs during it.
- Decision rule: primary = Footnote (own crawl) minus Grounded Docs, nDCG@5, paired bootstrap over the 40 questions. **Better** if the
  95% lower bound is above 0, **equivalent** if above −0.05, otherwise **worse** (at n=40 this means "not shown equivalent", not "shown
  worse"). Same labels for Footnote on Grounded Docs' pages. Reported, not judged: Recall@5, Hit@3, MRR, per site, median query latency,
  crawl and index seconds, judged-URL coverage.
- A host that blocks a crawl (HTTP 429, challenge) is reported as blocked and not retried into the score. If infrastructure interrupts the
  run, what failed and what caused it is recorded and the sealed set is **not** rerun silently: stop and report.
- No coloured winners unless the earlier rematches' rule allows it (colour once all engines of the #84 comparison are benched).
- No new third-party library or tool.
