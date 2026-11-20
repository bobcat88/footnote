# Stack ingest timing: crawl, keyword index and embeddings for 8 libraries (issue #308)

Not a quality benchmark. Question: how long does it take to ingest the docs of a realistic web-app stack, and how much of that is embedding?
Harness: [stack_ingest_timing.py](stack_ingest_timing.py); raw timestamps in [STACK-INGEST-TIMING-2026-11-20.log.txt](../archive/STACK-INGEST-TIMING-2026-11-20.log.txt).

## Setup

- Machine: Intel Core Ultra 7 270K Plus, Windows 11. Live CPU load sampled before the run (5 samples over 10 s: 0.5 to 4.3%) and before every step (0 to 10%). No build, benchmark or other heavy job ran during the measurement (Process Lasso ignored). The release build was done before the first sample.
- Binary: `cargo build --release` of main at `afc06e9` (sha256 `67c8b11ddf6faa77...`).
- Commands per library, run once, sequentially, 08:10 to 08:19 on 2026-11-20: `footnote crawl <url> --out <dir>` (product defaults: page cap 1000, adaptive concurrency, the per-host level remembered from earlier crawls), `footnote index` (keyword/lexical), `footnote rebuild-embeddings` with the shipped default pair (bekko-a8m + gte-small, CPU, batch 128). Seconds are wall-clock of each command; the machine's network and the sites' speed are part of the crawl numbers.
- "Keyword-only" = crawl + `index`, no `rebuild-embeddings`. Index MB is the SQLite file after `index`, and after embeddings.

## Results (one run per library)

| Library | Docs engine | Pages | Chunks | Crawl s | Index s | Keyword-only s | Embed s | Total s | DB MB keyword | DB MB with embeddings |
|---|---|---|---|---|---|---|---|---|---|---|
| Flask | Sphinx | 77 | 707 | 3.7 | 0.6 | 4.3 | 28.0 | 32.3 | 3.4 | 4.4 |
| SQLAlchemy 2.0 | Sphinx | 222 | 9460 | 15.8 | 0.9 | 16.7 | 357.5 | 374.2 | 45.0 | 57.7 |
| Requests | Sphinx | 26 | 313 | 2.7 | 0.5 | 3.2 | 12.2 | 15.4 | 1.6 | 2.0 |
| Jinja | Sphinx | 16 | 301 | 4.2 | 0.5 | 4.7 | 12.4 | 17.1 | 1.7 | 1.9 |
| Pydantic (second pass, see below) | MkDocs | 94 | 2092 | 4.0 | 0.6 | 4.6 | 71.3 | 75.9 | 6.9 | 9.6 |
| Vite guide | VitePress | 24 | 276 | 3.4 | 0.6 | 4.0 | 11.2 | 15.2 | 1.2 | 1.5 |
| Serde | mdBook | 38 | 100 | 3.7 | 0.6 | 4.3 | 5.1 | 9.4 | 0.6 | 0.7 |
| Tokio tutorial | static site | 11 | 107 | 3.2 | 0.4 | 3.6 | 5.1 | 8.7 | 0.6 | 0.7 |
| **Stack total** | | **508** | **13356** | **40.7** | **4.7** | **45.4** | **502.8** | **548.2** | **61.0** | **78.5** |

- Embedding is **91.7%** of the stack total (502.8 of 548.2 s). The keyword-only stack costs **45.4 s** (crawl 40.7 + index 4.7), about 8% of the full ingest.
- Embedding cost follows chunks, not pages: 34 to 41 ms per chunk for the six libraries with 270 chunks or more (SQLAlchemy 37.8, Pydantic 34.1, Flask 39.6); the two smallest libraries (100 and 107 chunks, 5.1 s each) work out at 48 to 51 ms, so a fixed per-run part exists but was not isolated.
- SQLAlchemy alone is 357.5 s of embedding (71% of the stack's embedding): 14.3 MB of Markdown, 43 chunks per page against 9 for Flask. The other seven libraries together take 145.3 s of embedding and 174.0 s in total.
- Embeddings add about 29% to the database size (61.0 to 78.5 MB).
- Flask ran in 32.3 s here against 37 s recorded earlier; the Python docs figure in the epic (500 pages, 637 s, 19 s crawl) is consistent with the same shape: crawl is seconds, embedding is minutes.

## A number that looked off, and the second pass

Pass 1 crawled Pydantic from `https://docs.pydantic.dev/latest/`: **9 pages**, although 326 sitemap and 87 `llms.txt` URLs were discovered (`skipped: 9`). The site now redirects there to `pydantic.dev/docs/validation/latest/get-started/` and the crawl scope follows the redirect, so it stayed inside the `get-started/` folder. Pass 2 started from `https://pydantic.dev/docs/validation/latest/` and took 94 pages (`skipped: 92`); that is the row above. Pass 1's 9-page numbers (crawl 1.3 s, embed 18.4 s) are in the log only. This is a coverage defect in the product's redirect handling (a start URL whose redirect lands on a child page gets that child's folder as scope), not a timing finding; it is not fixed here. Vite (24 pages from `vitejs.dev/guide/`, redirected to `vite.dev/guide/`) is the guide section only by design; its other sections (config, plugins) are outside that start URL and not counted. Request, Jinja, Serde and Tokio page counts match the sizes of those sites' docs as far as I could tell, but I did not verify each against the site's own page list.

## Conclusion

1. **Background ingest is needed for embeddings, not for keyword search.** A keyword-only stack is ready in 45 s for 8 libraries (3 to 17 s each), which fits a foreground step with progress. The embedded stack takes 9 minutes, 6 of them for one library; any library of 2000 chunks or more (Pydantic 71 s, SQLAlchemy 6 minutes) is too long to block the user on. This supports story #307 (keyword first, embeddings in a background job). Without SQLAlchemy the stack would embed in 2.4 minutes, so the need is driven by the few large libraries, not by the stack size.
2. **What to prioritise, in order:**
   - **Keyword first plus background embedding, one library per job, largest last.** This is the measured win: search works after 45 s instead of 548 s; nothing else here comes close.
   - **Pruning boilerplate / large generated pages before embedding.** Cost scales with chunks, and SQLAlchemy produces 43 chunks per page. I did not measure which pages or how much of those chunks is boilerplate (changelogs, generated API listings, indexes), so I claim no saving; a count of chunks by page type for SQLAlchemy and Pydantic is the cheap next measurement.
   - **GPU:** not measured in this run. The earlier DirectML study ([GPU-ENCODING.md](GPU-ENCODING.md)) found batch encoding 3.2 times faster only for fp32 bekko-a8m, and that the int8 gte-small gives different vectors on the GPU, which ruled it out for the shipped pair. So GPU does not apply to the default setup as shipped.
   - **Batch size:** fixed at 128 in `crates/search/src/semantic.rs`; no other value was tried here, so no claim.
   - **Page cap:** not the lever on this stack: the largest crawl is 222 pages, far below the default cap of 1000, so no library was cut by it.
3. Before relying on these numbers for a plan: one run per library; crawl seconds depend on each site and on the remembered concurrency level, and the first pass of Pydantic was wrong for the reason above.
