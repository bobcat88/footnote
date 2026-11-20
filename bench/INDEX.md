# Benchmark and report index

One row per benchmark, result or research report in the repository, with the headline copied from the source and a link to the exact line. This file is the single entry point; the reports themselves stay where they are (history, raw data and line links are preserved). Convention for new reports: [README.md](README.md).

- **Status** is one of CURRENT, SUPERSEDED by `<id>`, WORK IN PROGRESS, DEAD END. A protocol or plan row is CURRENT and points at its results row in the question. Rows are never deleted; a newer result marks the older row SUPERSEDED.
- **Headline** is the verbatim text of the linked line. It is the first result line of the report, not always the strongest number: n, model, machine and date are in the source and in the claims table below.
- Dates are the date in the file name, else the date the file was first committed.
- Last checked against `main` on 2026-11-20. `scripts/check_bench_index.py` fails when a report under `bench/experiments` is missing here or a link or line anchor does not resolve.
- Not indexed on purpose: roadmap notes (`docs/roadmap/`) and design documents.

## Claims the website may quote

Grades from the internal evidence ledger: **S** safe to state, **C** state with the caveat next to the number, **N** do not state. Machine unless noted: Intel Core Ultra 7 270K Plus, 24 logical CPUs, 47.6 GiB, Windows 11. Each row points to the index row holding the source line. If a source changes, change this table in the same PR.

| Ledger | Figure | Index row | Grade and caveat |
|---|---|---|---|
| L1 | nDCG@10: SciFact 0.747 (published BM25 0.665), NFCorpus 0.368 (0.325), Alloprof FR 0.478, Syntec FR 0.795, FR+EN mix 0.489. Official test splits, 14 Nov, one machine. | [LIB-05](#library), [LIB-01](#library) | S. Public sets may overlap with model training data. |
| L2 | Best quality on 5 of 5 corpora; significantly better in 24 of 26 comparisons; the other 2 are ties with mem0 on English sets. One run per config, paired bootstrap 10,000 resamples, 95%, no multiplicity correction; mem0 without LLM extraction, Cognee on chunk index, ICM keyword-only. | [LIB-01](#library), [LIB-03](#library), [LIB-04](#library) | C. Footnote loses on index build time (L5). Cognee crashed on the mixed set. The README contradiction noted in the ledger (2.7) was fixed in #334. |
| L3 | Query speed: p50 20 ms (5k chunks), 45 ms (40k), 52 ms (100k, p95 79 ms), end to end over MCP, 300 real queries. | data: `sweeps/scale_product.jsonl` | C. 40k and 100k rows use random vectors: speed, not precision. |
| L4 | Lexical index 3.0 ms vs SQLite FTS5 423 ms at 1M documents. | data: `dbbake/results.jsonl`, [INF-02](#infrastructure), [LIB-01](#library) | C. Synthetic text; speed only; never "100x faster than other tools". |
| L5 | Index build: Footnote 280 s vs mem0 162 s (SciFact); 712 s vs 336 s (mix); ICM 6 to 7x faster. | [LIB-01](#library) | S, as a stated weakness. |
| L6 | Docs crawl 10.1 s vs 160.6 s (Scrapy), 7.6 s vs 121.8 s (pytest), about 16x; median query 1.15 s vs 1.83 s (-37%). Sealed protocol, 2 sites, 40 questions. | [DOC-12](#library-technical-documentation-search-and-crawl) | C. Footnote stored 200 pytest pages vs 95. Two sites only. |
| L7 | Docs search quality vs Grounded Docs is **inconclusive**: nDCG@5 0.768 vs 0.721 (+6.5%), interval [-0.054, +0.155]; earlier 59-query set 0.564 vs 0.635. | [DOC-12](#library-technical-documentation-search-and-crawl), [DOC-02](#library-technical-documentation-search-and-crawl) | N for "better"; C only as "level within noise on four Sphinx sites". Sealed sets are spent (below). |
| L8 | First docs hit at 5.9 s; all 8 libraries keyword-searchable at 44.1 s; fully embedded at 567.5 s. One run, CPU. | [DOC-14](#library-technical-documentation-search-and-crawl) | C. Total time did not improve (567 s vs 548 s). Say "usable in seconds", never "faster ingest". |
| M1 | LongMemEval-S any@5 0.978, all@5 0.908, nDCG@10 0.939 (ICM keyword-only 0.952 / 0.820 / 0.875). 500 questions, shipped binary, 14 Nov. | [MEM-01](#memory) | C. The 0.966 and 0.970 figures are the 12 Nov run on 470 questions and are **SUPERSEDED** by this one ([MEM-02](#memory)); never quote them. |
| C1 | Rust: 28 of 7,707 held-out links conflict with the compiler (0.4%). 15 projects, rust-analyzer SCIP as truth. | [COD-02](#code) | C. Recall 0.04 to 0.20 per project; independent no-edge review still open. |
| C2 | TypeScript and Python: 6 of 13,449 held-out links conflict (0.04%). | [COD-03](#code), [COD-04](#code) | C. "Conflict rate" and "analyzed precision" are different numbers: name the one quoted. |
| C3 | Java precision 1.00, recall 0.70 to 0.85 (gson, jsoup, commons-lang), javac as truth. | [COD-04](#code) | C. Three projects. |
| C4 | One Rust repository: Footnote 0.98 precision / 0.50 recall vs CodeGraph 1.6.3 0.88 / 0.55 vs codebase-memory-mcp 0.12.1 0.61 / 0.41 (without tests). | [COD-07](#code) | C, heavy caveat: n = 1, the repository the rules were tuned on. Never "most accurate". |
| T1 | Lean B: output tokens 20 to 34% lower, runs 15 to 22% faster, total tokens 0 to 7% lower; `claude-opus-5-5`, low reasoning, ladder L1 and L2, 6 runs per arm, list price $3.70. | [LEAN-02](#lean-and-tokens) | C, small box. Opus, not Haiku. The ledger cited an unmerged branch; the report is now on `main`. |

Do not quote: L9, L10, L11, M3, M5, C5, T3, T6, T7 (ledger grade N) and anything from a row marked WORK IN PROGRESS or SUPERSEDED.

## Reports

### Memory

| Id | Date | Question | Headline (verbatim from the source line) | Status | Source |
|---|---|---|---|---|---|
| MEM-01 | 2026-11-14 | How well does the shipped binary recall the right session on LongMemEval-S (500 questions), against ICM? | **Footnote**, shipped binary (`target/release/footnote.exe`), local run · **0.978** · **0.908** · **0.992** · **0.970** · **0.939** | CURRENT | [LONGMEM-ICM.md:7](experiments/LONGMEM-ICM.md#L7) |
| MEM-02 | 2026-11-12 | Earlier LongMemEval-S run (12 Nov, 470 questions, AND matching, one model per row) | > Earlier run, 12 November 2026: 470 questions (abstention-tagged ones excluded), a fresh process per query and AND lexical matching, with a single embedding model per row (MiniLM-L12 or e5-small). The current figure for the shipped binary is any@5 0.978, all@5 0.908 on all 500 questions, run on 14 November: see [`experiments/LONGMEM-ICM.md`]\(experiments/LONGMEM-ICM.md). The … | SUPERSEDED by MEM-01 | [RESULTS.md:96](RESULTS.md#L96) |
| MEM-03 | 2026-11-15 | What does keyword-extracted memory recall on LongMemEval-S before any change (plan and run 1)? | Raw: `bench/work/memory-baseline-none.json`. Median 30 memories per question; every question got at least one. | CURRENT | [MEMORY-BASELINE.md:28](experiments/MEMORY-BASELINE.md#L28) |
| MEM-04 | 2026-11-15 | Does adding the source context around a memory raise development recall? (protocol) | Frozen on 15 November 2026 before collecting contextualized recall results. | CURRENT | [MEMORY-CONTEXT.md:3](experiments/MEMORY-CONTEXT.md#L3) |
| MEM-05 | 2026-11-15 | Results of the source-context experiment | Keep bounded source context as a development candidate. This experiment supports retaining enough | CURRENT | [MEMORY-CONTEXT-RESULTS.md:64](experiments/MEMORY-CONTEXT-RESULTS.md#L64) |
| MEM-06 | 2026-11-15 | Does a compact source qualification cut noise without losing recall? (protocol) | Reuse exactly the 244 development databases from MEMORY-CONTEXT.md and their deterministic | CURRENT | [MEMORY-QUALIFICATION.md:11](experiments/MEMORY-QUALIFICATION.md#L11) |
| MEM-07 | 2026-11-15 | Results of the compact source qualification | Do not replace the current representation with this compact prefix recipe. It does not meet its | CURRENT | [MEMORY-QUALIFICATION-RESULTS.md:69](experiments/MEMORY-QUALIFICATION-RESULTS.md#L69) |
| MEM-08 | 2026-11-15 | Can a relevance cutoff on the fusion score switch recall by meaning on each prompt? | ## Result: no activation (15 November 2026) | SUPERSEDED by MEM-09 | [MEMORY-CUTOFF.md:28](experiments/MEMORY-CUTOFF.md#L28) |
| MEM-09 | 2026-11-16 | Recall by meaning on each prompt: does it pass the gate on a real memory database? | ## Result (16 November 2026): passes, hybrid is the default | CURRENT | [MEMORY-RECALL-SWITCH.md:45](experiments/MEMORY-RECALL-SWITCH.md#L45) |
| MEM-10 | 2026-11-17 | Can a reranker decide when no memory answers, so the hook injects nothing? | 1. **Dev only:** choose `t` as the value that gives the lowest no-answer injection rate while answerable any@3 stays within | CURRENT | [MEMORY-ABSTENTION.md:33](experiments/MEMORY-ABSTENTION.md#L33) |
| MEM-11 | 2026-11-16 | Do tags work as a pre-filter before ranking? | Built 16 November 2026 on request ("add the extra filtering"), following notes from 15 November: tags narrow the | CURRENT | [MEMORY-TAGS.md:3](experiments/MEMORY-TAGS.md#L3) |
| MEM-12 | 2026-11-15 | Agent-written memories: bilingual qualification pilot (protocol) | Frozen 15 November 2026 before question creation or inference. Synthetic source histories and | CURRENT | [MEMORY-WRITTEN.md:3](experiments/MEMORY-WRITTEN.md#L3) |
| MEM-13 | 2026-11-15 | Do self-contained agent-written memories hold at 600-document bilingual scale? | Measured 15 November 2026. Protocol: [MEMORY-WRITTEN.md]\(MEMORY-WRITTEN.md). | CURRENT | [MEMORY-WRITTEN-RESULTS.md:3](experiments/MEMORY-WRITTEN-RESULTS.md#L3) |
| MEM-14 | 2026-11-13 | How does ICM work and what should be tested against it? (research note, plan) | Research note, 2026-11-12. This is an evaluation plan and a comparison of documented behavior, not a claim that Footnote is ahead. ICM's claims are vendor-reported, not independently reproduced here. | CURRENT | [ICM-COMPARISON.md:3](experiments/ICM-COMPARISON.md#L3) |
| MEM-15 | 2026-11-13 | ICM and Footnote comparison suite (experimental, prepares controlled tests) | Status: experimental. This suite prepares controlled tests; it does not by itself fix retrieval or establish that either product wins. Existing production behavior is unchanged. | CURRENT | [ICM-SUITE.md:3](experiments/ICM-SUITE.md#L3) |

### Library

| Id | Date | Question | Headline (verbatim from the source line) | Status | Source |
|---|---|---|---|---|---|
| LIB-01 | 2026-11-14 | Is Footnote ahead of mem0, Cognee and ICM on the same documents and queries, and where does it lose? | Bold Δ: the paired interval excludes zero. **Footnote has the best quality on all five corpora; it is significantly better in 24 of 26 comparisons; the two others are ties with mem0 on the English sets.** On speed, Footnote is 4 to 10 times faster than mem0 (except tiny Syntec, where mem0's in-process call wins 6.9 against 12.8 ms) and 6 to 30 times faster than Cognee; ICM … | CURRENT | [FINAL-RANKING.md:17](experiments/FINAL-RANKING.md#L17) |
| LIB-02 | 2026-11-14 | Head-to-head receipts, 26 of 27 complete | Complete receipts: 26/27. Missing: 1. Receipt window starts at 20261113T070000Z. | CURRENT | [HEADTOHEAD.md:3](experiments/HEADTOHEAD.md#L3) |
| LIB-03 | 2026-11-14 | Paired bootstrap intervals for each head-to-head comparison | Paired bootstrap, 10,000 query resamples and fixed seed. Per-comparison intervals, without multiplicity correction. Competitor configurations differ in models/chunking; matched controls use identical frozen vectors and exclude encoding. | CURRENT | [HEADTOHEAD-INTERVALS.md:3](experiments/HEADTOHEAD-INTERVALS.md#L3) |
| LIB-04 | 2026-11-14 | Footnote against ICM 0.11.5 keyword-only, paired intervals | Paired bootstrap, 10,000 query resamples, fixed seed, per comparison without multiplicity correction. A = shipped Footnote, B = ICM 0.11.5 keyword-only (no embeddings). Test split. | CURRENT | [ICM-INTERVALS.md:3](experiments/ICM-INTERVALS.md#L3) |
| LIB-05 | 2026-11-12 | Retrieval quality of the shipped binary and the lab engines on the public test splits (generated from receipts) | Generated by `bench/report.py` from `bench/receipts/`. Partial receipts are excluded. Receipts made before the `bench-v1` tag are provisional. | CURRENT | [RESULTS.md:3](RESULTS.md#L3) |
| LIB-06 | 2026-11-12 | Which layers improve the pipeline beyond the frozen default? (pre-registered 12 Nov) | Goal: find layers that improve the Footnote retrieval pipeline beyond the frozen default (bekko-a8m + BM25 en_stem + convex combination, | CURRENT | [PLAN.md:3](experiments/PLAN.md#L3) |
| LIB-07 | 2026-11-12 | Results of the pre-registered optimization experiments | Pre-registered in [PLAN.md]\(PLAN.md) on 2026-11-12, before any result. Base: the frozen default (bekko-a8m, BM25 with English stemmer, | CURRENT | [RESULTS.md:3](experiments/RESULTS.md#L3) |
| LIB-08 | 2026-11-12 | Per-experiment tables E01 to E11, generated by report_exp.py | ### E01 to E11: one change at a time | CURRENT | [TABLES.md:1](experiments/TABLES.md#L1) |
| LIB-09 | 2026-11-14 | Which relevant documents does the shipped-like stack miss on dev? | Shipped-like lab stack (bekko-a8m + gte-small, z-score convex fusion, alpha 0.55, whole documents). Counts are relevant documents. Dev data only: this chooses experiments, it proves nothing. | CURRENT | [MISS-ANALYSIS.md:3](experiments/MISS-ANALYSIS.md#L3) |
| LIB-10 | 2026-11-14 | Which stronger embedding models gain on dev? | Tuning results only. These are not end-to-end latency measurements or heldout confirmation. | CURRENT | [MODEL-CANDIDATES.md:3](experiments/MODEL-CANDIDATES.md#L3) |
| LIB-11 | 2026-11-14 | Do the dev-qualified models also win on held-out data? | Candidates must gain at least 0.01 nDCG@10 on both dev corpora of a language group. Configurations are frozen before heldout scoring. Positive bootstrap intervals alone do not authorize a production change; latency, memory and integration checks still apply. Failed or incomplete dev configurations are ineligible. Intervals are per comparison, without multiplicity correction. | CURRENT | [MODEL-CONFIRMATION.md:3](experiments/MODEL-CONFIRMATION.md#L3) |
| LIB-12 | 2026-11-14 | What does resident query encoding cost per model? | Encoding only, no retrieval or query cache. Same fixed dev queries per corpus for every model. | CURRENT | [MODEL-LATENCY.md:3](experiments/MODEL-LATENCY.md#L3) |
| LIB-13 | 2026-11-15 | Can stronger embedding models through llama.cpp meet the cost limits? (test 5) | **Outcome of test 5: no candidate is adopted.** All three are clearly better on held-out data, but none meets the query cost limit | DEAD END | [LLAMACPP-MODELS.md:214](experiments/LLAMACPP-MODELS.md#L214) |
| LIB-14 | 2026-11-15 | Held-out confirmation of the llama.cpp candidates (test 5) | Condition 2 of `LLAMACPP-MODELS.md`: paired bootstrap 95 % interval of the nDCG@10 difference above zero on each corpus of the group. Configurations frozen from the dev sweep. | CURRENT | [LLAMACPP-CONFIRMATION.md:3](experiments/LLAMACPP-CONFIRMATION.md#L3) |
| LIB-15 | 2026-11-15 | Does llama.cpp in-process, without the HTTP server, meet query cost? (test 6) | Runner `t6_inproc.py`, rows in `sweeps/t6_inproc.jsonl`. Milliseconds per query, 100 dev queries per corpus, quiet machine. | DEAD END | [LLAMACPP-INPROCESS.md:37](experiments/LLAMACPP-INPROCESS.md#L37) |
| LIB-16 | 2026-11-16 | Which llama.cpp runtimes run best on the Radeon RX 6800? | 15 November 2026. Backlog P3 (llama.cpp speed grid, runtime versions). Reported, not judged. Script: | CURRENT | [LLAMACPP-RUNTIMES.md:3](experiments/LLAMACPP-RUNTIMES.md#L3) |
| LIB-17 | 2026-11-15 | Does a Vulkan speed grid justify a new indexing configuration? | results do not justify enabling a new indexing configuration. CPU samples | DEAD END | [LLAMACPP-SPEED-GRID-2026-11-15.md:47](experiments/LLAMACPP-SPEED-GRID-2026-11-15.md#L47) |
| LIB-18 | 2026-11-15 | A3 embedding candidates: screening protocol | Frozen before screening results, 15 November 2026. This is an extension of | CURRENT | [A3-SCREEN-2026-11-15.md:3](experiments/A3-SCREEN-2026-11-15.md#L3) |
| LIB-19 | 2026-11-15 | A3 screening: intermediate results | The six stage-A runs are complete. Protocol: [A3-SCREEN-2026-11-15.md]\(A3-SCREEN-2026-11-15.md). | CURRENT | [A3-SCREEN-RESULTS-2026-11-15.md:3](experiments/A3-SCREEN-RESULTS-2026-11-15.md#L3) |
| LIB-20 | 2026-11-16 | Qwen3-Embedding-4B single-terminal-token recheck (protocol) | model name and cache, the same official GGUF hash, b11901 Vulkan, GPU layers 99, | CURRENT | [QWEN4-TERMINAL-TOKEN-2026-11-16.md:6](experiments/QWEN4-TERMINAL-TOKEN-2026-11-16.md#L6) |
| LIB-21 | 2026-11-16 | Qwen3-Embedding-4B single-terminal-token recheck (result) | Protocol: [QWEN4-TERMINAL-TOKEN-2026-11-16.md]\(QWEN4-TERMINAL-TOKEN-2026-11-16.md). | CURRENT | [QWEN4-TERMINAL-TOKEN-RESULTS-2026-11-16.md:3](experiments/QWEN4-TERMINAL-TOKEN-RESULTS-2026-11-16.md#L3) |
| LIB-22 | 2026-11-14 | Does GPU encoding with DirectML or OpenVINO help? | bekko-a8m · fp32 · 87.8 → 283.8 (**3.2×**) · yes: cosine 1.000000, nDCG identical · 1.0 ms → 11.3 ms | CURRENT | [GPU-ENCODING.md:9](experiments/GPU-ENCODING.md#L9) |
| LIB-23 | 2026-11-14 | How fast can the index build, with pre-registered rules? | **Result (`work/build-speed-t2-parallel.jsonl`):** one after the other 70.2 s, at the same time 74.8 s (0.94×), vectors identical. **Rejected:** with 12 threads each, the two models already saturate the CPU. | CURRENT | [BUILD-SPEED.md:33](experiments/BUILD-SPEED.md#L33) |
| LIB-24 | 2026-11-14 | LanceDB against SQLite plus Tantivy on identical inputs (protocol) | Pre-registered 2026-11-13, before running these configurations. This is a component experiment, not a production migration. | CURRENT | [LANCEDB-MATCHED.md:3](experiments/LANCEDB-MATCHED.md#L3) |
| LIB-25 | 2026-11-14 | Matched backend results, 20 of 20 receipts | Complete receipts: 20/20. Missing: 0. Receipt window starts at 20261113T070000Z. | CURRENT | [LANCEDB-RESULTS.md:3](experiments/LANCEDB-RESULTS.md#L3) |
| LIB-26 | 2026-11-15 | Qdrant server, matched storage comparison | Result: on 100 frozen queries for each of Syntec, NFCorpus, SciFact, Alloprof | CURRENT | [QDRANT-STORAGE-2026-11-15.md:51](experiments/QDRANT-STORAGE-2026-11-15.md#L51) |
| LIB-27 | 2026-11-16 | PostgreSQL plus pgvector, isolated comparison | The isolated conda-forge environment contains PostgreSQL 16.14 and pgvector | CURRENT | [POSTGRES-PGVECTOR-2026-11-16.md:23](experiments/POSTGRES-PGVECTOR-2026-11-16.md#L23) |
| LIB-28 | 2026-11-16 | Graph stores on Windows: do they reproduce the exact top ten? | Result: on all five corpora, the sets of the first ten chunks matched the NumPy | CURRENT | [GRAPH-STORES-2026-11-16.md:42](experiments/GRAPH-STORES-2026-11-16.md#L42) |
| LIB-29 | 2026-11-16 | Exact vector candidate speed, bounded comparison | ## Result (16 November, 05:11, `bench/work/storage/exact-speed-mix-20261116`) | CURRENT | [STORAGE-EXACT-SPEED-2026-11-16.md:28](experiments/STORAGE-EXACT-SPEED-2026-11-16.md#L28) |
| LIB-30 | 2026-11-13 | Research queue and the 27 competitor runs | RRF loses to convex combination; per-query adaptive weights and pseudo-relevance feedback do not help; BM25 parameter tuning and BM25 variants do not matter | CURRENT | [README.md:16](research/README.md#L16) |
| LIB-31 | 2026-11-13 | Which competitors besides ICM belong in the head-to-head? | Caveat on slot 2: the graph is the architecture that "teaches something", but under a no-LLM setup the graph is built by a small NER model (GLiNER) and the `CHUNKS` search never touches it. A Cognee run measures "Cognee's chunk index", not "Cognee's graph retrieval". Say so in the report. | CURRENT | [competitors.md:13](research/competitors.md#L13) |
| LIB-32 | 2026-11-13 | Which embedding candidates could replace or add to bekko-a8m and gte-small? | Research date: 2026-11-13. All facts below were fetched online on that date unless a revision/date is given. Nothing was downloaded (only model-card text, HF API metadata, and JSON score files). | CURRENT | [embedding-boost.md:3](research/embedding-boost.md#L3) |
| LIB-33 | 2026-11-13 | How could the fusion stage improve? | Research date: 2026-11-13. All web pages were read on that date. "Fetched" means I read the page text myself; "via search summary" means I only saw a search-tool summary and did not read the page. | CURRENT | [fusion-boost.md:3](research/fusion-boost.md#L3) |
| LIB-34 | 2026-11-13 | What could beat plain Tantivy BM25? | Research date: 2026-11-13. Every number below carries its source in the source list (section 4). Anything I could not confirm in this session is marked UNVERIFIED. No code was run; no number comes from Footnote's own harness except those quoted in the brief. | CURRENT | [lexical-boost.md:3](research/lexical-boost.md#L3) |
| LIB-35 | 2026-11-15 | Embedding model leaderboards (reference) | Snapshot taken 14 November 2026 from the MTEB leaderboard (https://huggingface.co/spaces/mteb/leaderboard, model pages at | CURRENT | [EMBEDDING-LEADERBOARDS.md:3](../docs/research/EMBEDDING-LEADERBOARDS.md#L3) |

### Library: technical documentation search and crawl

| Id | Date | Question | Headline (verbatim from the source line) | Status | Source |
|---|---|---|---|---|---|
| DOC-01 | 2026-11-15 | Crawl coverage and end-to-end search against Grounded Docs | Written 15 November 2026, before any measurement. Backlog item 14; study: [DOCS-INGESTION-STUDY.md]\(../../docs/research/DOCS-INGESTION-STUDY.md). | CURRENT | [DOCS-CRAWL.md:3](experiments/DOCS-CRAWL.md#L3) |
| DOC-02 | 2026-11-15 | Is Footnote at least equivalent to Grounded Docs 3.4.1 at finding the right documentation page? | Crawl (Grounded Docs 3.4.1, its CI recipe): react 219 pages, python 500, fastapi 152, vite 28, tailwindcss 223. Both engines search | CURRENT | [DOCS-VS-GROUNDED.md:44](experiments/DOCS-VS-GROUNDED.md#L44) |
| DOC-03 | 2026-11-15 | Documentation retrieval follow-ups (protocol) | Prepared 15 November 2026, after the `DOCS-VS-GROUNDED.md` and grouped-result | CURRENT | [DOCS-FOLLOWUPS-2026-11-15.md:3](experiments/DOCS-FOLLOWUPS-2026-11-15.md#L3) |
| DOC-04 | 2026-11-15 | Documentation retrieval follow-up results | 15 November 2026. Protocol: [DOCS-FOLLOWUPS-2026-11-15.md]\(DOCS-FOLLOWUPS-2026-11-15.md). | CURRENT | [DOCS-FOLLOWUPS-RESULTS-2026-11-15.md:3](experiments/DOCS-FOLLOWUPS-RESULTS-2026-11-15.md#L3) |
| DOC-05 | 2026-11-16 | Documentation search on fresh questions: confirmation | Written 16 November 2026, before any system ran on these libraries. Confirms (or not) the diagnostics of | SUPERSEDED by DOC-06 | [DOCS-CONFIRM-2026-11-16.md:3](experiments/DOCS-CONFIRM-2026-11-16.md#L3) |
| DOC-06 | 2026-11-16 | Optimisation on seen sets and decision on a sealed set (Hono, Flask; set spent) | Both crawlers back to back on each library, the order alternating; a dev-set index build was running on the CPU meanwhile, | CURRENT | [DOCS-OPTIMIZE-2026-11-16.md:179](experiments/DOCS-OPTIMIZE-2026-11-16.md#L179) |
| DOC-07 | 2026-11-19 | Rematch on Sphinx documentation, Flask and Python (protocol; sealed set spent) | Written 19 November 2026, before any system ran on the development libraries. Issue #106, epic #84. Follows | CURRENT | [DOCS-SPHINX-2026-11-19.md:3](experiments/DOCS-SPHINX-2026-11-19.md#L3) |
| DOC-08 | 2026-11-19 | Sphinx rematch results (#106), Flask and Python sealed set (spent) | Not shown: that Footnote's own-crawl search is equivalent to Grounded Docs' on Sphinx documentation at n=40; the sealed result is | CURRENT | [DOCS-SPHINX-RESULTS-2026-11-19.md:51](experiments/DOCS-SPHINX-RESULTS-2026-11-19.md#L51) |
| DOC-09 | 2026-11-19 | Sphinx crawl coverage under a page cap: four ideas (protocol) | Written 19 November 2026, before any variant ran. Context: [DOCS-SPHINX-RESULTS-2026-11-19.md]\(DOCS-SPHINX-RESULTS-2026-11-19.md): same text, | CURRENT | [DOCS-SPHINX-CRAWL-2026-11-19.md:3](experiments/DOCS-SPHINX-CRAWL-2026-11-19.md#L3) |
| DOC-10 | 2026-11-19 | Sphinx crawl coverage: results of four ideas | Protocol: [DOCS-SPHINX-CRAWL-2026-11-19.md]\(DOCS-SPHINX-CRAWL-2026-11-19.md), written before the runs. Harness `docs_sphinx_crawl.py`. | CURRENT | [DOCS-SPHINX-CRAWL-RESULTS-2026-11-19.md:3](experiments/DOCS-SPHINX-CRAWL-RESULTS-2026-11-19.md#L3) |
| DOC-11 | 2026-11-20 | Sealed final 2, Scrapy and pytest (protocol; set sha256 committed before the run) | Written 20 November 2026, before any system ran on these sites. Issue #301, follows | CURRENT | [DOCS-SPHINX-SEALED2-2026-11-20.md:3](experiments/DOCS-SPHINX-SEALED2-2026-11-20.md#L3) |
| DOC-12 | 2026-11-20 | Footnote against Grounded Docs on Scrapy and pytest: quality and speed (set spent) | **The quality result is inconclusive.** Footnote's own crawl scores higher by 0.047 nDCG@5, but the interval at 40 questions spans −0.054 to | CURRENT | [DOCS-SPHINX-SEALED2-RESULTS-2026-11-20.md:38](experiments/DOCS-SPHINX-SEALED2-RESULTS-2026-11-20.md#L38) |
| DOC-13 | 2026-11-20 | Where does ingest time go for 8 libraries? (crawl, keyword index, embeddings) | Embedding is **91.7%** of the stack total (502.8 of 548.2 s). The keyword-only stack costs **45.4 s** (crawl 40.7 + index 4.7), about 8% of the full ingest. | CURRENT | [STACK-INGEST-TIMING-2026-11-20.md:27](experiments/STACK-INGEST-TIMING-2026-11-20.md#L27) |
| DOC-14 | 2026-11-20 | Is a project's docs keyword-searchable within seconds while embeddings finish? | First search usable: the first `footnote find` returned a docs hit at **5.9 s** (Serde alone; keyword-only, as the corpus had no vectors yet). The `docs ingest` command returned, with all 8 keyword indexes built, at **44.1 s**; the user is blocked for that long, not for the embedding. | CURRENT | [STACK-INGEST-TIMING-2026-11-20-B.md:28](experiments/STACK-INGEST-TIMING-2026-11-20-B.md#L28) |
| DOC-15 | 2026-11-15 | Technical documentation in the Library: storage and crawler | 15 November 2026. Backlog item 14. Desk study; the measurements are pre-registered in | CURRENT | [DOCS-INGESTION-STUDY.md:3](../docs/research/DOCS-INGESTION-STUDY.md#L3) |
| DOC-16 | 2026-11-15 | Mistral Libraries website indexing | Checked against Mistral's public documentation on 15 November 2026. This note | CURRENT | [MISTRAL-LIBRARIES-CRAWLER-2026-11-15.md:3](../docs/research/MISTRAL-LIBRARIES-CRAWLER-2026-11-15.md#L3) |

### Code

| Id | Date | Question | Headline (verbatim from the source line) | Status | Source |
|---|---|---|---|---|---|
| COD-01 | 2026-11-17 | Rust receiver typing, development check | 17 November 2026. This is a development check, not a held-out accuracy result or an agent benchmark. | CURRENT | [CODE-RECEIVERS-2026-11-17.md:3](experiments/CODE-RECEIVERS-2026-11-17.md#L3) |
| COD-02 | 2026-11-18 | How often does the Rust call graph contradict the compiler on 15 projects? (#214) | typed receiver) and refuse otherwise. Across 7,707 held-out links, 28 conflict with the compiler (0.4%), and the receiver | CURRENT | [CODE-RUST-PRECISION-2026-11-18.md:77](experiments/CODE-RUST-PRECISION-2026-11-18.md#L77) |
| COD-03 | 2026-11-18 | Same for TypeScript and Python (#216) | On held-out code, 6 of 13,449 comparable links conflict with the compiler (0.04%). The "analyzed, no edge" links are compiler | CURRENT | [CODE-TS-PY-PRECISION-2026-11-18.md:40](experiments/CODE-TS-PY-PRECISION-2026-11-18.md#L40) |
| COD-04 | 2026-11-19 | Can confidence tiers raise recall without lowering default precision? | Goal: raise recall without lowering the default tier's precision. The default tier (receiver_type, same_file, import, type_path, | CURRENT | [CODE-TIERS-2026-11-19.md:3](experiments/CODE-TIERS-2026-11-19.md#L3) |
| COD-05 | 2026-11-20 | Local type inference for Python and TypeScript receivers (#299) | Goal (#299): resolve `x.method()` when `x` is a local of a known type, without lowering the default tier's precision. | CURRENT | [CODE-INFERENCE-2026-11-20.md:3](experiments/CODE-INFERENCE-2026-11-20.md#L3) |
| COD-06 | 2026-11-20 | Overloaded definitions as one symbol (#299) | Goal (#299): raise recall without lowering the default tier's precision. | CURRENT | [CODE-OVERLOADS-2026-11-20.md:3](experiments/CODE-OVERLOADS-2026-11-20.md#L3) |
| COD-07 | 2026-11-14 | Three tools, clean truth: Footnote against CodeGraph and codebase-memory-mcp on one Rust repository (section, 17 Nov) | **Three tools, clean truth (17 November 2026).** Once a repository is graphed with rust-analyzer installed, its graph also holds | CURRENT | [CODE-GRAPH-DESIGN.md:305](../docs/CODE-GRAPH-DESIGN.md#L305) |
| COD-08 | 2026-11-15 | Code graph study: plan and decision rule |  | CURRENT | [CODE-GRAPH-STUDY.md:63](../docs/research/CODE-GRAPH-STUDY.md#L63) |
| COD-09 | 2026-11-20 | `footnote code-index --deep`: default tier vs deep tier on six projects (#299) | **Finding that changes the premise:** the compiler graph was already what `code-index` builds whenever the indexer is on PATH | CURRENT | [CODE-DEEP-2026-11-20.md:7](experiments/CODE-DEEP-2026-11-20.md#L7) |

### Lean and tokens

| Id | Date | Question | Headline (verbatim from the source line) | Status | Source |
|---|---|---|---|---|---|
| LEAN-01 | 2026-11-19 | Does LEAN-B lower cost per accepted task against Ponytail and Caveman? (plan) | Question: does `rules/lean/LEAN-B.md` lower **cost per accepted task** against Ponytail and Caveman? First pilot run: see LEAN-B-RESULTS-2026-11-20.md. No | CURRENT | [LEAN-B-BENCH.md:3](experiments/LEAN-B-BENCH.md#L3) |
| LEAN-02 | 2026-11-20 | Lean B pilot on ladder L1 and L2 with Opus 5.5 | Output tokens drop 20 to 34% in every treatment arm, and runs are 15 to 22% faster. Output is a small part of the total, so total tokens move only 0 to 7%. | CURRENT | [LEAN-B-RESULTS-2026-11-20.md:28](experiments/LEAN-B-RESULTS-2026-11-20.md#L28) |
| LEAN-03 | 2026-11-18 | Token economy layer: research and plan | 18 November 2026. Plan for a layer that saves input and output tokens, especially on code work, in the spirit of RTK | CURRENT | [TOKEN-ECONOMY-PLAN.md:3](../docs/research/TOKEN-ECONOMY-PLAN.md#L3) |

### Infrastructure

| Id | Date | Question | Headline (verbatim from the source line) | Status | Source |
|---|---|---|---|---|---|
| INF-01 | 2026-11-12 | Bench protocol, contenders and rules | Status: **protocol v1, harness built and validated, optimization sweeps in progress**. The rules below were written before the first result so they cannot bend to fit the numbers. | CURRENT | [README.md:3](README.md#L3) |
| INF-02 | 2026-11-12 | Decision rules, written before the optimization results | Written 2026-11-11, while the model sweep was still running. The weights below are fixed; changing them after seeing results requires a | CURRENT | [DECISION.md:5](DECISION.md#L5) |
| INF-03 |  | Bench progress log of 12 to 13 Nov (stale, archived) | Updated 2026-11-13. The protocol and rules are in `../README.md`; dated sections below preserve the earlier work. | SUPERSEDED by INF-01 | [PROGRESS.md:3](archive/PROGRESS.md#L3) |
| INF-04 | 2026-11-17 | Shared daemon throughput before and after one lock per model | Measured 17 November 2026 on a quiet test PC (confirmed idle: CPU 6 % average over 10 s before the run; no other cargo work | CURRENT | [DAEMON-THROUGHPUT-2026-11-17.md:3](experiments/DAEMON-THROUGHPUT-2026-11-17.md#L3) |
| INF-05 | 2026-11-16 | MCP concurrency diagnostic (protocol) | Frozen 15 November 2026, before the measured run. Implements the two scenarios | CURRENT | [DOCS-MCP-LOAD.md:3](experiments/DOCS-MCP-LOAD.md#L3) |
| INF-06 | 2026-11-15 | MCP concurrency: first diagnostic | 15 November 2026. Protocol: [DOCS-MCP-LOAD.md]\(DOCS-MCP-LOAD.md). A private | CURRENT | [DOCS-MCP-LOAD-RESULTS-2026-11-15.md:3](experiments/DOCS-MCP-LOAD-RESULTS-2026-11-15.md#L3) |
| INF-07 | 2026-11-16 | Personal memory encryption: build check | Rule written before the run (docs/roadmap/EDITIONS.md, 16 Nov): personal memory is encrypted regardless; project memory and | CURRENT | [SQLCIPHER-FEASIBILITY.md:3](experiments/SQLCIPHER-FEASIBILITY.md#L3) |
| INF-08 | 2026-11-19 | Third-party due diligence, 19 Nov | Follow-up skills & tools triage (19 Nov). Sourced upstream repositories were downloaded into a separate temporary review folder outside this repository and checked for structure, execution behavior, network calls, and licences. Candidates were evaluated for Footnote's agent workflow needs and ecosystem alignment. | CURRENT | [THIRD-PARTY-DUE-DILIGENCE.md:3](../docs/research/THIRD-PARTY-DUE-DILIGENCE.md#L3) |
| INF-09 | 2026-11-16 | Remote access from phones and tablets | 16 November 2026. Read from T3 Code's own documentation ([pingdotgg/t3code]\(https://github.com/pingdotgg/t3code), MIT): | CURRENT | [REMOTE-ACCESS-STUDY.md:3](../docs/research/REMOTE-ACCESS-STUDY.md#L3) |
| INF-10 | 2026-11-15 | Launcher design: initial analysis | 15 November 2026. Initial design analysis inputs before locking `DESIGN.md`; no visual decisions are taken here. Plan: | CURRENT | [LAUNCHER-DESIGN-ANALYSIS.md:3](../docs/research/LAUNCHER-DESIGN-ANALYSIS.md#L3) |
| INF-11 | 2026-11-18 | Security audit, 18 Nov | **Verdict:** no critical, high or medium issue open. Two medium issues found in the first pass are fixed. | CURRENT | [AUDIT-2026-11-18.md:6](../docs/security/AUDIT-2026-11-18.md#L6) |
| INF-15 | 2026-11-20 | Raw timestamps for DOC-13 | # pass 1, 8 libraries (run dir stackrun-20261120-0810) | CURRENT | [STACK-INGEST-TIMING-2026-11-20.log.txt:1](archive/STACK-INGEST-TIMING-2026-11-20.log.txt#L1) |
| INF-16 | 2026-11-20 | Raw timestamps for DOC-14 | 10:33:01 flow: cpu% before 2,2490529190745 1,48461811544321 5,39074080792665 | CURRENT | [STACK-INGEST-TIMING-2026-11-20-B.log.txt:1](archive/STACK-INGEST-TIMING-2026-11-20-B.log.txt#L1) |

## Sealed docs sets: single-use

A sealed set is read once, for one final decision. After that it is spent and must not be reused to tune or to claim.

| Set | Used by | State |
|---|---|---|
| [`datasets/docs-heldout3`](datasets/docs-heldout3/dataset.yaml) (Hono, Flask) | [DOC-06](#library-technical-documentation-search-and-crawl) | SPENT |
| [`datasets/docs-sphinx-sealed`](datasets/docs-sphinx-sealed/dataset.yaml) (Flask, Python) | [DOC-08](#library-technical-documentation-search-and-crawl) | SPENT |
| [`datasets/docs-sphinx-sealed2`](datasets/docs-sphinx-sealed2/dataset.yaml) (Scrapy, pytest) | [DOC-12](#library-technical-documentation-search-and-crawl) | SPENT |
| [`datasets/docs-confirm`](datasets/docs-confirm/dataset.yaml), [`datasets/docs-sphinx-dev`](datasets/docs-sphinx-dev/dataset.yaml) | [DOC-05](#library-technical-documentation-search-and-crawl), [DOC-10](#library-technical-documentation-search-and-crawl) | development sets, seen |

## Raw data, harness and archive

Not reports, but where the numbers come from. Rows above link to the reports; these hold the evidence behind them.

| Path | Holds |
|---|---|
| [`receipts/`](receipts/) | Immutable result receipts (`dev/`, `heldout/`, `superseded/` with its own [README](receipts/superseded/README.md)). Evidence for LIB-05, MEM-01 and MEM-02. |
| [`sweeps/`](sweeps/) | Append-only tuning rows (`*.jsonl`). |
| [`dbbake/`](dbbake/results.jsonl), [`lab/`](lab/) | Lexical scale results and the lab engine with its latency and model-check rows. |
| [`datasets/`](datasets/), [`environments/`](environments/README.md) | Frozen question sets and competitor environment snapshots. |
| [`experiments/code-graph/`](experiments/code-graph/receiver-review-2026-11-17.tsv) | Raw receiver review behind COD-01 to COD-03. |
| [`archive/`](archive/) | Moved here with `git mv`: raw logs ([INF-15](#infrastructure), [INF-16](#infrastructure)) and the stale progress log ([INF-03](#infrastructure)). Nothing the code reads. |
