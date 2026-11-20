# Sealed final 2 on Sphinx documentation (Scrapy, pytest): results (#301)

Protocol: [DOCS-SPHINX-SEALED2-2026-11-20.md](DOCS-SPHINX-SEALED2-2026-11-20.md), committed with the set's SHA-256 before the run. Raw numbers:
`DOCS-SPHINX-SEALED2-2026-11-20-final.json`. Set sha256 `b5a53b5c9df3c1d251fd839e461727d07775eff9b6875e350e307ca47f932483`, recipe
`38f9d095d483b1a30eca472e459b5fcad63b8b02c18163049c8283d641761440`. Footnote built from main `77ea5c2` (with the link-following crawl, PR #300),
binary sha256 `1e14802ff6bf3dbb3a096cab0ff6e7d5d8aabbe9549fa99c7871e8cd2468f4d2`. Grounded Docs 3.4.1 hybrid, EmbeddingGemma-300M Q8_0
(llama.cpp b11901, GPU). Seed 20261121, order Scrapy: Footnote first, pytest: Grounded Docs first, 600 s cool-down. Machine load about 1.5% before
the start (CPU samples 0.2 to 2.7%, no other benchmark running; Process Lasso ignored). The run was started at 07:19 and ended at 07:54 without interruption.

**Before the run:** two launch attempts died before touching any site or system (a missing output folder, then an invalid JSON escape in
`recipe.json`, fixed and re-hashed, recorded in the protocol; the questions were not changed). The third launch is the only run; nothing was rerun.

## Result: 40 questions, 2 sites

Δ% = Footnote ÷ Grounded Docs − 1 (higher is better for quality, lower is better for time). Paired bootstrap, 10,000 resamples.

| Measure | Grounded Docs | Footnote (own crawl) | Δ% | Footnote on Grounded's pages | Δ% |
|---|---|---|---|---|---|
| nDCG@5, all 40 | 0.721 | 0.768 | +6.5% (+0.047 [−0.054, +0.155], not shown equivalent) | 0.712 | −1.3% (−0.010 [−0.122, +0.097], not shown equivalent) |
| nDCG@5, Scrapy 20 | 0.745 | 0.745 | 0.0% | 0.719 | −3.4% |
| nDCG@5, pytest 20 | 0.698 | 0.791 | +13.4% | 0.704 | +0.9% |
| MRR | 0.707 | 0.715 | +1.2% | 0.690 | −2.5% |
| Recall@5 | 0.842 | 0.942 | +11.9% | 0.858 | +2.0% |
| Hit@3 | 0.825 | 0.925 | +12.1% | 0.875 | +6.1% |
| Query latency, median s | 1.83 | 1.15 | −37.1% | | |
| Crawl, Scrapy s | 160.6 | 10.1 | −93.7% | | |
| Crawl, pytest s | 121.8 | 7.6 | −93.8% | | |
| Judged-URL coverage | 1.000 | 1.000 | | | |

Pages stored: Scrapy 146 (Grounded Docs) against 153 (Footnote); pytest 95 against 200 (the cap). Footnote's index builds: 76 s (Scrapy)
and 87 s (pytest). Grounded Docs' embedding time is inside its scrape and is not comparable, so no index-time delta is claimed. Footnote's two
search passes gave identical rankings. No host blocked either crawl.

Winners are not coloured: the #84 comparison with other engines is not finished (same rule as the earlier rematches).

## Reading

- **The quality result is inconclusive.** Footnote's own crawl scores higher by 0.047 nDCG@5, but the interval at 40 questions spans −0.054 to
  +0.155: the lower bound is below −0.05, so by the protocol's rule it is not shown equivalent, and it is not shown better. Not a win claim.
- On identical text (Grounded Docs' stored pages) the two are level (−0.010, interval −0.122 to +0.097): the ranking is not better or worse
  than Grounded Docs' here. The own-crawl lead on pytest comes from the crawl, not the ranking: Footnote stored 200 pages where Grounded stored
  95, and Recall@5 is 0.942 against 0.842; on Scrapy, where the page sets match, the two are equal (0.745 each).
- Together with the Flask/Python final (0.770 against 0.774, inconclusive), the pooled picture on four Sphinx sites is "level with Grounded Docs
  within the noise of 40-question sets". That pooling is not a protocol result and is not claimed as one.
- **Clear on speed:** Footnote crawls about 16 times faster (−94%) and answers in about 63% of the time (−37%, median per query, both CLIs run
  one query at a time alternating).
- Judged-URL coverage was 1.000 for both on both sites, so the link-following crawl did not lose pages here; it was not stress-tested by a
  site where the cap cuts out judged pages (pytest reached the cap and still covered all 24 judged pages).

## What this changes

Nothing in the product. No document states the earlier Sphinx result as a loss that this run overturns, so no claim wording was changed; the
roadmap backlog gets one line pointing here.
