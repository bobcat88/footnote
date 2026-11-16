# Documentation retrieval: optimisation on seen sets, decision on a sealed set

Written 16 November 2026, after DOCS-CONFIRM (worse: −0.119 end to end) and before any change below is built.

## Sets

- **Development (seen):** Grounded Docs' 59 questions (react, python, fastapi, vite, tailwindcss) and docs-confirm's 40
  (pydantic, svelte). Every candidate is tuned and selected here only.
- **Sealed decision set:** `bench/datasets/docs-heldout3/dataset.yaml` (sha256
  `aa54e2119e5543d1fc6c0165d3207906f77baa450a8624b3a2f44e054d1c1b7c`): 40 questions, Hono 20 (llms.txt site) and Flask 20
  (Sphinx HTML, no llms.txt), 52 judged URLs, each checked to answer HTTP 200; recipe `docs-heldout3/recipe.json` (200 pages
  each, `llms-*.txt` copies excluded). Written by Claude before any system ran on these libraries. Run once, at the end.
- **General regression guard:** the shipped held-out benches (Alloprof, mix-fr-en, `shipped-ensemble-mcp`), because chunking
  changes touch every corpus.

## Candidates

Search side (product changes, each measured alone then combined):
- **A. Context header:** each chunk carries its page title and heading path (`Title > H2 > H3`) in the embedded text and the
  keyword heading field, instead of the last heading only.
- **B. Structure-aware sizes:** merge sections shorter than a floor into the next one within the same parent, target about
  1,500 characters, never split a fenced code block.
- **C. Post-crawl cleanup:** drop lines repeated on most pages of one site (navigation, footers, banners) and near-duplicate
  pages before indexing.
- **D. Page assembly:** one result per page (`--per-source 1`) returning the best chunk with its heading path.

Crawler side:
- **E. Adaptive concurrency per host:** start at 2 requests in flight, add one after each run of successes up to 8, halve on
  HTTP 429/503 or timeouts, honour `Retry-After` and robots.txt `Crawl-delay`, and remember the last good level per host.
  Compressed responses (gzip, brotli). No probing beyond what the crawl itself requests.

End to end (added 16 November, 06:05, before any measurement of it):
- **F. Index while crawling:** pages are chunked and embedded as they arrive instead of after the crawl, and the per-library
  index build time is profiled (chunking, keyword index, vectors) to find the next bottleneck. Target reported, not judged:
  Footnote's crawl plus index finishes before Grounded Docs' crawl alone on the same libraries.

Result B (floor 500, target 1,500, hard cap 3,000 inside code blocks; against A, `docs_dev.py` with `DOCS_REF`):

| Set | nDCG@5 | B - A [95 % CI] |
|---|---|---|
| gd (59) | 0.7030 | +0.0478 [+0.0089, +0.0908] |
| confirm (40) | 0.6182 | +0.0208 [-0.0520, +0.0888] |
| pooled (99) | 0.6687 | +0.0369 [-0.0009, +0.0744] |

General guard (held-out, `shipped-ensemble-mcp`): Alloprof nDCG@10 0.4090 (A 0.4757, -0.067), mix-fr-en 0.4362 (A 0.4892,
-0.053). **Not kept:** the guard drops far more than 0.005. Those corpora have no Markdown headings, so the only change they
see is the size (1,500 instead of 2,000 characters).

**B2 (added 16 November, 08:00, before measuring it):** B's merging of short sections and whole code blocks, with A's size
(a chunk ends past 2,000 characters outside code, 3,000 inside), so text without headings or fences is chunked exactly as
before. Same rule as B, against A.

Result B2 (against A):

| Set | nDCG@5 | B2 - A [95 % CI] |
|---|---|---|
| gd (59) | 0.7133 | +0.0580 [+0.0196, +0.1008] |
| confirm (40) | 0.6378 | +0.0404 [-0.0335, +0.1135] |
| pooled (99) | 0.6828 | +0.0509 [+0.0124, +0.0892] |

General guard: Alloprof nDCG@10 0.4757 (A 0.4757), mix-fr-en 0.4888 (A 0.4892, -0.0005). **Kept.** Over the shipped baseline
(before A) the two kept search changes add about +0.095 nDCG@5 on the pooled development questions (A +0.044, then B2 +0.051).

Extraction (added 16 November, 07:40, after finding it in the crawl output and before any measurement of the fix):
- **G. Main landmark over a stray Readability pick:** 65 of the 500 Python pages Footnote crawled hold only the site footer
  (index pages and removed-module pages: the real content is short and Readability prefers the longer footer). When the page
  has a main landmark (`main`, `role=main`, `article`) and Readability's text is not inside it, or Readability finds nothing,
  use the landmark. **Keep G** when a fresh crawl of the seven development libraries with G recovers the footer-only Python
  pages and changes, on the other six libraries, no more than 2 % of pages by more than 5 % of their extracted length
  (sites change between crawls; changed pages are listed and inspected). Its ranking effect is measured with the other
  kept changes in the final run only.

**Result G, first try (`main`, `role=main`, `article`): not kept.** Python footer-only pages 65 -> 1, but on the other six
libraries 14 of 883 pages changed (1.6 %), with FastAPI 4.6 % and Svelte 2.9 % over the 2 % line read per library. Inspected:
FastAPI and Svelte pages gain their title and API headings (better; FastAPI links become relative), React's home page grows,
and Tailwind `line-clamp` drops to a lorem-ipsum demo card: Tailwind has no `main`, so the first `<article>` (an example) won.
**Deviation, written before re-running:** G is narrowed to `main` and `role=main` (no `article`) and the G side is crawled
again; the same rule applies, per library.

**Result G, second try (`main`, `role=main`): not kept.** Tailwind 0 changed; Python fixed (65 -> 1); FastAPI 7 of 152 (4.6 %)
and Svelte 5 of 173 (2.9 %) still over the line. Inspected: Readability's pick was inside `main` on those pages; the check
compared texts with whitespace collapsed, and the page's text nodes run together ("agentsUse") where Readability spaces them,
so it fired by mistake. On Svelte the landmark also brings hover-tooltip text into code blocks (worse).
**Deviation 2, written before re-running:** the containment check ignores all whitespace (first 60 characters); the G side is
crawled again; same rule.

**Result G, third try: not kept.** Svelte 4 of 173 (2.3 %), FastAPI still 7 of 152 (4.6 %). Probe of FastAPI `/about/`:
Readability drops the heading permalink (`About¶`) that the landmark text keeps, so a prefix match still fails.
**Deviation 3, written before re-running:** "inside" becomes word overlap: at least half of the distinct words (3+ letters,
lower case) of Readability's text occur in the landmark. The G side is crawled again; same rule. This is the last try: if it
fails, G is dropped.

**Result G, fourth try: kept.** Fresh back-to-back crawls (`bench/work/dev/extract-check/`, `changes.json`): Python
footer-only pages 65 -> 1 (the one left is `copyright.html`, which is a copyright page); the 57 Python pages changed by more
than 5 % were all footer-only before (for example `nis.html` now opens with "Deprecated since version 3.11, removed in version
3.13"); React, FastAPI, Vite, Tailwind CSS, Pydantic and Svelte: 0 pages changed. The three earlier tries are kept as
`G-try1..3` and `changes-try1..3.json`.

## Rules

- **Keep a search change** when mean nDCG@5 rises on the pooled development questions (99) and neither development set drops
  by more than 0.01, and the general guard drops by no more than 0.005 nDCG@10 on either corpus.
- **Keep E** when the crawl of the seven development libraries is faster in wall time with no more failed or refused
  requests than sequential, and coverage of judged URLs is unchanged.
- **Final decision, once, on the sealed set:** Footnote (own crawl, all kept changes) minus Grounded Docs 3.4.1 (own crawl,
  hybrid, EmbeddingGemma), nDCG@5, paired bootstrap over 40 questions (10,000 resamples, seed 20261117). Equivalent: lower bound
  above −0.05; better: above 0. If at least equivalent, the crawler ships with the kept changes as defaults.
- Reported, not judged: end-to-end time (crawl plus index) of both systems on the sealed libraries, index size.
- **Competitor runs (Johan, 16 November):** candidates are A/B-tested against Footnote's own previous version only; Grounded
  Docs is run once, on the sealed set, after gains have accumulated, and not at all if no candidate is kept.

## Result E, adaptive concurrency (16 November, 06:03, `bench/work/dev/crawl-speed/`)

Both crawlers back to back on each library, the order alternating; a dev-set index build was running on the CPU meanwhile,
for both sides alike.

| Library | Sequential | Adaptive | Speed-up | Pages, skipped (both) | Judged URLs covered (both) |
|---|---|---|---|---|---|
| react | 46.5 s | 8.4 s | 5.5 | 218, 91 | 100 % |
| python | 63.9 s | 15.4 s | 4.1 | 500, 7 | 100 % |
| fastapi | 8.2 s | 4.3 s | 1.9 | 152, 14 | 100 % |
| vite | 6.7 s | 4.1 s | 1.7 | 24, 0 | 83 % |
| tailwindcss | 20.8 s | 6.1 s | 3.4 | 223, 0 | 85 % |
| pydantic | 16.0 s | 5.0 s | 3.2 | 93, 92 | 100 % |
| svelte | 5.8 s | 3.1 s | 1.9 | 173, 1 | 100 % |
| **Total** | **167.8 s** | **46.2 s** | **3.6** | identical | identical |

No site answered 429, 503 or timed out; every crawl ended at the maximum of 8 in flight. **Kept** (faster, no more failures,
same coverage).

## Result A, heading path (16 November, 06:30, `bench/work/dev/`)

Same exported pages, shipped pair, hybrid, `--per-source 1`; baseline = the binary before A.

| Development set | Baseline nDCG@5 | A | Difference, paired 95 % interval |
|---|---|---|---|
| Grounded Docs' 59 | 0.6081 | 0.6553 | +0.0472 [+0.0154, +0.0822] |
| docs-confirm's 40 | 0.5574 | 0.5973 | +0.0400 [−0.0134, +0.0928] |
| **Pooled 99** | 0.5876 | **0.6318** | **+0.0443 [+0.0160, +0.0738]** |

Per library: python 0.667 → 0.794, pydantic 0.444 → 0.514, tailwindcss 0.368 → 0.416, fastapi 0.784 → 0.830,
vite 0.569 → 0.600, svelte 0.670 → 0.680, react 0.633 → 0.616. Part of the Python gain is the fix that stops `#` comments in
code blocks from splitting chunks.

General guard (held-out, `shipped-ensemble-mcp`, A's binary): Alloprof nDCG@10 0.4757 (before 0.4757), mix-fr-en 0.4892
(before 0.4892): unchanged, these corpora have no Markdown headings. **Kept.**
