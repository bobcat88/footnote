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

D and F made concrete (added 16 November, 09:10, before measuring either; Johan gave the go the same morning to run the final
alone once D and F are settled):
- **D, measured:** the B2 development databases searched hybrid, `--limit 5`, with `--per-source 1` against no per-file cap
  (today's product default; the metric already counts each page once, so a repeated page wastes a slot). **Keep D** by the
  search rule on the development questions (pooled up, neither set down by more than 0.01). If kept, the final passes
  `--per-source 1` and the MCP tool description recommends it for documentation; the product default stays unchanged until
  a guard run with the cap exists (the guard harness has no per-file option).
- **F, this round:** profile the index build per development library (index: extraction, chunking and keyword index; then
  vectors per model) on an idle machine (live CPU sampled first, Process Lasso ignored). A speed-up is kept only if the
  rebuilt databases return identical rankings on the 99 development questions and the build is faster in wall time. Indexing
  while crawling is reported as a design note if the profile shows it would pay; it is not built in this round.

**Result D (B2 databases, `bench/work/dev/D-nocap/`):** no per-file cap against `--per-source 1`: gd 0.6894 (-0.0239
[-0.0428, -0.0084]), confirm 0.6008 (-0.0369 [-0.0695, -0.0111]), pooled 0.6536 (-0.0292 [-0.0457, -0.0144]). **Kept:** the cap
of one passage per page is worth +0.029 nDCG@5 pooled; the final passes `--per-source 1`, the MCP tool description recommends
it for documentation.

**Result F, this round:** the 14 November build study (`BUILD-SPEED.md`) already profiled the build: encoding is 97-99.9 %
ONNX inference, both models already saturate the CPU (two models at once: 0.94x, rejected), and batch size or order changes
the int8 vectors. What changed since: B2 embeds 30 % fewer chunks on the development libraries (29,678 -> 20,860) and the
seven builds took 771 s against 916 s for A (84 %; same machine, not certified idle). Indexing while crawling would overlap at
most the crawl itself (about 38 s against about 754 s of indexing on five libraries), so it is not worth building now. The
lever left is document encoding on the GPU (test 3 of `BUILD-SPEED.md`, never run), kept only if vectors match the CPU.

**Final run, how (written 16 November before it starts; Johan's go to run it alone, same morning):** kept changes A, B2, D,
E, G (and the lean output, which does not change rankings). Sealed set `bench/datasets/docs-heldout3/dataset.yaml`, SHA-256
checked before the run (aa54e211…). One binary for the Footnote side (`bench/work/dev/bin/final.exe`, from `feat/lean-output`).
Sequential, on the idle machine, nothing else running:
1. Grounded Docs 3.4.1: `docs_vs_grounded.py` with `DOCS_RUN=bench/work/heldout3/run1`, `DOCS_DATASET` and `DOCS_RECIPE` of
   the sealed set: its own scrape (timed per library), hybrid search with EmbeddingGemma on llama-server.
2. Footnote: `docs_crawl.py coverage minimal final.exe` then `endtoend minimal final.exe --limit 5 --per-source 1 --mode hybrid
   <shipped pair>` with `DOCS_CRAWL_WORK=bench/work/heldout3/crawl` and `DOCS_SEED=20261117`: own crawl (timed), index,
   search, paired bootstrap against step 1's Grounded hybrid rankings.
End-to-end time reported, not judged: Footnote crawl plus index build against Grounded's scrape (which embeds as it goes).

**Final run, deviations (written 16 November, 08:45, after the run, before any valid end-to-end score):**
1. **Scoring bug (harness):** `DOCS_CRAWL_WORK` was given as a relative path, so the file-to-URL manifest had relative keys
   while Footnote returns absolute paths; no hit mapped to a URL and every end-to-end score came out 0.000. Fix: resolve
   `DOCS_CRAWL_WORK` to an absolute path. The databases, binary and rankings are unchanged (`fn-hybrid-pass1/2.json` kept);
   only the step that maps and scores is re-run. The invalid outputs are kept as `endtoend-invalid-relative-paths.json`.
2. **Flask crawl blocked (environment):** Grounded Docs scraped Flask first (76 pages in 51 s); minutes later Cloudflare
   served our IP a challenge (HTTP 429 on every page and on robots.txt, still so at 08:45 with curl). Footnote's crawl
   backed off as designed and stopped after 501 slow-downs: 38 Flask pages in 1,006 s, coverage of judged Flask URLs 0.64
   (Grounded 1.00). The 38 pages are real (no challenge page saved). The pre-registered primary stays this as-run crawl;
   a Flask re-crawl once the block lifts is reported as a diagnostic only, and the sequential order (Grounded first) is a
   flaw of this design, noted for future finals (alternate the order or wait between systems).

**Final result (sealed docs-heldout3, 40 questions, seed 20261117; `bench/work/heldout3/`):**

| Questions | Grounded Docs (own scrape, hybrid) | Footnote, own crawl (primary) | Footnote on Grounded's pages, `--per-source 1` (diagnostic) |
|---|---|---|---|
| Hono (20) | 0.879 | 0.893 (+0.014 [-0.044, +0.075]) | 0.915 (+0.036 [-0.037, +0.117]) |
| Flask (20) | 0.960 | 0.540 (-0.420 [-0.626, -0.220]) | 0.887 (-0.073 [-0.151, -0.005]) |
| All (40) | 0.920 | **0.716 (-0.203 [-0.334, -0.082])** | 0.901 (-0.019 [-0.074, +0.039]) |

**Verdict by the pre-registered rule: worse** (lower bound -0.334 < -0.05), so the crawler does not ship as default from this
run. Reading: on Hono, where both crawls were complete, Footnote is equivalent or better (and faster end to end: crawl 2 s +
index 17 s against Grounded's scrape 38 s). The loss is Flask: mostly the blocked crawl (38 of 76 pages, judged-URL coverage
0.64), and even on Grounded's own Flask pages Footnote trails by 0.073 (Sphinx-built docs, like Python; not investigated on
this sealed set, which is now spent). On Grounded's pages over both libraries: -0.019 [-0.074, +0.039], which the -0.05 rule
also calls worse by its lower bound. Lessons: a final must not let one system's crawl spoil the host for the other (alternate
order, cool-down, or crawl both from cached pages for the quality part), and Sphinx sites need a dev-set look before the next
final.

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
