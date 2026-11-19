# Documentation search rematch on Sphinx docs: results (#106)

Protocol: [DOCS-SPHINX-2026-11-19.md](DOCS-SPHINX-2026-11-19.md), written before the runs. Raw numbers: `DOCS-SPHINX-2026-11-19-final.json`.
Footnote binary `final.exe` sha256 `7a2879b40eb4c012067f2c80d8c8105c73a4dcde70550b461f54abcc82df5e0b`; Grounded Docs 3.4.1 hybrid with
EmbeddingGemma-300M Q8_0 (llama.cpp b11901, GPU). Sealed set sha256 `2e06dc864bfd4b345c78f25bb18ad89194400f1da1e28146e7dc50081a9d113a`,
seed 20261120, machine idle (about 4% load) when it started. Order alternated (Flask: Footnote first; Python: Grounded first), 600 s cool-down;
the run was not interrupted (my two resume messages only killed waiting commands, the benchmark process ran to its end).

## Sealed final: Flask and Python, 40 questions

Δ% = Footnote ÷ Grounded Docs − 1 (higher is better for quality, lower is better for time). Paired bootstrap, 10,000 resamples;
verdict labels are the protocol's: better if the 95% lower bound > 0, equivalent if > −0.05, otherwise "worse" (which with n=40 means
"not shown equivalent", not "shown worse").

| Measure | Grounded Docs | Footnote (own crawl) | Δ% | Footnote on Grounded's pages | Δ% |
|---|---|---|---|---|---|
| nDCG@5, all 40 | 0.774 | 0.770 | −0.4% (−0.003 [−0.085, +0.078], not shown equivalent) | 0.810 | +4.6% (+0.036 [−0.038, +0.110], equivalent) |
| nDCG@5, Flask 20 | 0.733 | 0.778 | +6.1% | 0.786 | +7.2% |
| nDCG@5, Python 20 | 0.814 | 0.763 | −6.3% | 0.833 | +2.3% |
| MRR | 0.757 | 0.742 | −2.0% | 0.776 | +2.5% |
| Recall@5 | 0.913 | 0.913 | 0.0% | 0.913 | 0.0% |
| Hit@3 | 0.850 | 0.875 | +2.9% | 0.850 | 0.0% |
| Query latency, median s | 2.62 | 1.24 | −52.8% | | |
| Crawl, Flask s | 53.3 | 3.5 | −93.4% | | |
| Crawl, Python s (500 pages) | 629.6 | 19.0 | −97.0% | | |
| Judged-URL coverage | 1.000 | 1.000 | | | |

Winners are not coloured: Grounded Docs is the only other engine benched, but the earlier rematches' rule (colour once all engines are
benched) applies to the whole #84 comparison, which is not finished.

Reading: on identical text Footnote is equal or better (+0.036); on its own crawl the points are equal overall (Flask ahead, Python behind),
and the interval is too wide at 40 questions to call them equivalent. Footnote answers in half the time and crawls 14 to 33 times faster.
Index builds: Footnote 37 s (Flask) and 637 s (Python) end to end against Grounded Docs' crawl-plus-embed time inside its scrape;
those are not comparable one for one and no delta is claimed.

## Development round (Django, Jinja; Sphinx), 28 questions

| Run | Grounded | Footnote | Δ% | note |
|---|---|---|---|---|
| dev3, own crawl | 0.831 | 0.647 | −22.1% | Django coverage 0.435 (200-page cap, sitemap order) |
| dev3, same text | 0.831 | 0.900 | +8.3% | ranking is not the problem |
| dev4, sitemap shallow-first (H) | 0.831 | 0.506 | −39.1% | coverage fell to 0.217: H rejected and removed |
| Sphinx site, 12 questions | not run | not run | | host blocks Grounded Docs from this IP; Footnote-only numbers are in the run folders, not reported as a rematch |

## What this changes in the product, and what it does not

- Fixed (kept): `footnote crawl` was blocked by Cloudflare on Read the Docs and Pallets hosts through its Windows TLS stack; it now uses
  rustls with the platform verifier. Without it Footnote could not have crawled Flask or Jinja at all.
- Open: on very large sites under a page cap Footnote follows the sitemap's order and can miss the pages that matter (Django 0.435);
  Grounded Docs follows links from the start page. Sitemap depth ordering did not help. Not tuned on the sealed set.
- Not shown: that Footnote's own-crawl search is equivalent to Grounded Docs' on Sphinx documentation at n=40; the sealed result is
  inconclusive on quality and clear on speed.
