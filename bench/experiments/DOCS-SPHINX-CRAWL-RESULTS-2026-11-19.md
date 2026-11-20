# Sphinx crawl coverage under a page cap: results of four ideas

Protocol: [DOCS-SPHINX-CRAWL-2026-11-19.md](DOCS-SPHINX-CRAWL-2026-11-19.md), written before the runs. Harness `docs_sphinx_crawl.py`.
Development set only (Django 16 + Jinja 12 = 28 questions); the sealed Flask/Python set was not read. Footnote only, hybrid search
`--limit 5 --per-source 1`, machine load 4 to 9% before the runs (Process Lasso ignored), one run per variant: seconds are indicative.
Reference (recorded, not rerun): Grounded Docs 0.831 nDCG@5 on this core set (dev3).
Sphinx's own site (12 questions) was not run: its host has challenged this IP since the rematch, and Footnote-only numbers there would
not be comparable with anything.

## Results (cap 200 unless stated; Django has 23 judged pages of grade >= 2, Jinja 8)

| Variant | Django coverage | nDCG@5 all (Django) | Recall@5 all | Django crawl / index s | Django DB MB |
|---|---|---|---|---|---|
| Baseline (sitemap order), same day | 0.435 | 0.649 (0.408) | 0.679 | 67 / 151 | 16.1 |
| 1 `objects.inv` seed, inventory order | 0.696 | 0.761 (0.605) | 0.821 | 77 / 248 | 27.0 |
| 1 `objects.inv` seed, most symbols first | 0.783 | 0.832 (0.729) | 0.875 | 52 / 238 | 27.4 |
| 2 links first, sitemap after (`links`) | 1.000 | 0.895 (0.839) | 0.946 | 3 / 202 | 23.0 |
| 2 alternate links and sitemap (`hybrid`) | 1.000 | 0.897 (0.843) | 0.946 | 3 / 214 | 24.2 |
| 1 + 2 (symbols first + hybrid) | 1.000 | 0.905 (0.857) | 0.946 | 138 / 218 | 24.7 |
| 3 fetch on miss, 5 pages per query, on baseline (always) | n/a | 0.803 | 0.839 | +5 s per query | |
| 3 fetch on miss, 5 pages per query, on 1 (always) | n/a | 0.852 | 0.893 | +7 s per query | |
| 4 cap 400 (sitemap order) | 1.000 | 0.881 (0.815) | 0.929 | 71 / 313 | 32.7 |
| 4 cap 800 (sitemap order; the site has 657 pages) | 1.000 | 0.878 (0.810) | 0.929 | 155 / 340 | 36.2 |
| 1 + 4 (most symbols first, cap 400) | 0.957 | 0.867 (0.791) | 0.929 | 138 / 278 | 31.0 |
| **Kept: hybrid only, final binary** | 1.000 | 0.894 (0.837) | 0.946 | 12 / 208 | 24.2 |

Jinja: coverage 1.000 and nDCG@5 0.969 in every variant (16 pages, cap never reached).
Final binary sha256 `6474746276506279f1319220281ac9eb9ac2e3eec27eaa83655887742b633614`; baseline `ba02572fad48...7a4e`. The kept build is 0.894
nDCG@5 against Grounded Docs' recorded 0.831 on the same questions: this is a development number, tuned on these questions, so it is **not**
a claim of a win (see "Not done").

## Idea by idea

1. **`objects.inv` seeding: helps alone, not kept.** Every Sphinx site publishes it (Django: 659 documented pages). Taking its pages first
   lifts Django coverage 0.435 to 0.696 (inventory order) and 0.783 when pages with the most documented symbols come first. On top of link
   following it adds 0.008 nDCG@5 (about one question of 28), inside the noise, and it needs a zlib reader (a direct `flate2`
   dependency, already in the tree through `ureq`) and only works on Sphinx. With a 400-page cap the symbol order was worse than plain
   sitemap order (0.957 against 1.000). Not worth a dependency. `searchindex.js` lists the same documents and was not tried separately.
2. **Link-following: kept (the fix).** Following links from the start page, taking turns with the sitemap's order, reaches every judged
   Django page in 200 pages (coverage 1.000, nDCG@5 0.649 to 0.894). Pure links first and the
   alternation are indistinguishable (0.895 against 0.897), so the one that still reaches pages nothing links to (the alternation) is kept.
   **Invalid first attempt:** my first `links` and `hybrid` runs showed no effect (coverage 0.435) because a link to a page the sitemap had already
   listed was never queued; fixed, rerun (`links200b`, `hybrid200b`) and only the rerun is reported above.
   The sitemap itself was empty in one run (`cap800`: 0 discovered, rate limit), where link-following alone crawled all 657 pages.
3. **Fetch on miss: helps, not shipped.** Prototype: per query, fetch the 5 sitemap/inventory pages whose URL words best match the query, index
   them, search again. Always fetching lifts the baseline from 0.649 to 0.803 and the objects.inv crawl from 0.832 to 0.852. A weak-result
   trigger on the top fusion score does not separate misses from hits (thresholds 2.5 to 5 give 0.663 to 0.767 on the baseline, always 0.803;
   on the symbols crawl a threshold of 3 is worse than never fetching, 0.813 against 0.832), and each fetch costs 5 to 7 s at query time and
   needs the source URL kept per index and a network call from `search`. Once the crawl reaches the pages (idea 2), it has nothing left to
   fix on Django. The prototype code was removed; the results stand as recorded here. Open as a follow-up for sites where no cap is large enough.
4. **Higher cap: the cheap control works but costs more.** Cap 400 reaches coverage 1.000 and 0.881 nDCG@5 on Django, at 2.1 times the
   index time (313 s against 151 s) and 2 times the database (32.7 MB); cap 800 adds nothing (the site has 657 pages) and costs 340 s. Link-following
   gets 0.894 at cap 200 for 208 s and 24 MB. It also works when the cap is the limit and the sitemap is flat or empty.

Combined: objects.inv with link-following 0.905 against link-following alone 0.894: not kept (see 1).

## General guard (not tuned on)

docs-confirm (Pydantic 20, Svelte 20), cap 200, baseline against the kept build: coverage 1.000 both; nDCG@5 0.569 against 0.567
(Pydantic 0.741 both, Svelte 0.397 to 0.392, within the -0.01 limit); these sites have fewer than 200 pages so the cap is not reached.
The search code is unchanged.

## Not done

- No new sealed set: a claim that Footnote beats or equals Grounded Docs on Sphinx documentation needs one (written and hashed before
  the run, alternating order, 600 s cool-down). Stopped at development results as instructed.
- Sphinx's own site, `searchindex.js`, Flask and Python (sealed set spent) were not run.
- One run per variant on a shared machine; Django's host slowed the crawls differently each time (crawl seconds range from 3 to 155 s for
  the same 200 pages).
