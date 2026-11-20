# Sphinx crawl coverage under a page cap: four ideas (follow-up to #106)

Written 19 November 2026, before any variant ran. Context: [DOCS-SPHINX-RESULTS-2026-11-19.md](DOCS-SPHINX-RESULTS-2026-11-19.md): same text,
Footnote ranks better than Grounded Docs; on its own crawl it loses judged-URL coverage under a 200-page cap (Django 0.435; shallow-first
sitemap order was worse, 0.217, and was removed).

## Rules

- Development questions only: `bench/datasets/docs-sphinx-dev` (Django 16, Jinja 12 = `dataset-core.yaml`, 28 questions; Sphinx's own 12 added
  only if its host answers). The sealed Flask/Python set (sha256 2e06dc864bfd...) is spent: not read, not tuned on.
- Harness `docs_sphinx_crawl.py`: crawl, judged-URL coverage (grade >= 2), index, hybrid search `--limit 5 --per-source 1`, nDCG@5 and
  Recall@5 with Grounded Docs' metrics. Footnote only; Grounded Docs is not rerun per candidate. The reference points are the recorded
  dev3 numbers: Grounded 0.831 nDCG@5 on the core set; Footnote baseline (cap 200, sitemap order) coverage 0.435, nDCG@5 0.647.
  The baseline is also rerun here with the same harness, so every variant is compared with a same-day, same-harness number.
- Each idea alone, then the kept ones combined. Kept when Django coverage and nDCG@5 rise and nothing falls (Jinja has 15 pages and
  must stay at 1.0). Ideas that do not help are reverted and recorded.
- Any claim of a final win needs a NEW sealed set, written and hashed before its run, alternated order, 600 s cool-down. Not in this round
  unless time allows; otherwise this stops at dev results.

## Ideas

1. `inv`: seed from Sphinx's `objects.inv` (every documented page; `invsym` = pages with most documented symbols first). `searchindex.js`
   lists the same pages (docnames), so it is not tried separately.
2. `links` (link-following first, sitemap as fallback) and `hybrid` (alternate one link-discovered page with one sitemap page).
3. Fetch on miss: at query time, when results are weak, fetch the pages most likely missing instead of a bigger cap.
4. Higher cap (control): 200 / 400 / 800, with index size and crawl time.
