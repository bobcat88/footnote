# Bench progress (archived 2026-11-20, stale; current state is in ../INDEX.md)

Updated 2026-11-13. The protocol and rules are in `../README.md`; dated sections below preserve the earlier work.

## Current queue, 2026-11-13

Work is in `D:\Projets\BorgKnowledge`, branch `bench/model-candidates`. The active task is the retrieval comparison and improvements, not the landing page.

1. `run_headtohead.sh` is running the 27 configured Footnote/mem0/Cognee cases. Last inspected at 10:18 local time: twelve complete receipts, Footnote running on SciFact. Eleven completed cases pass their recorded repeat check; Cognee lexical on NFCorpus fails it. Syntec and NFCorpus are finished. Logs: `work/headtohead.log`.
2. `run_models2.sh` waits for that suite, then runs the four model candidates and the frozen pair on four dev corpora. Logs: `work/models2.log`; append-only results: `sweeps/models2.jsonl`.
3. `model_latency.py` waits for step 2, then measures resident batch-1 encoding for six models on the same fixed dev queries. Logs: `work/model_latency.log` and `work/model_latency.err.log`.
4. `run_matched.ps1` waits for step 3. It confirms dev-qualified model configurations on heldout, exports fixed public inputs, runs the four matched backend controls on five corpora, writes reports, runs 100k/1M lexical scale controls, then runs the benchmark and Rust checks. Logs: `work/matched.log`, `work/matched.err.log`; PID: `work/matched.pid`; explicit backend/scale failures: `work/matched-failures.jsonl`.

These processes are already launched. Do not start another heavy benchmark or duplicate these runners. Check their command lines and logs before restarting anything. A `DONE` line is a runner completion marker; results and failure records still need review.

## Current evidence and implementation

- [Competitor report](../experiments/HEADTOHEAD.md), with receipt links, missing cases, scopes and storage parts. Regenerate with `rtk proxy python -B bench/experiments/headtohead_table.py --since 20261113T070000Z`.
- [Interim paired intervals](../experiments/HEADTOHEAD-INTERVALS.md). Ten receipt rankings were re-scored and checked against their recorded hashes. NFCorpus Footnote versus mem0 is not resolved by its paired interval.
- Quality blocker discovered at 10:05 local time: `20261113T083346Z-nfcorpus-cognee-lexical.json` reports different rankings across its two passes, checked on 200 queries. The receipt is retained as failure evidence. The global receipt validation gate fails on this case; its point score must not be treated as a reproducible result. Investigate adapter behavior and tie ordering before attributing the failure to the product.
- [Matched backend protocol](../experiments/LANCEDB-MATCHED.md), frozen before judged-corpus results. Seven backend/export tests pass. These use one model to isolate backend effects; they are not a production-pair comparison.
- [Environment snapshots](../environments/README.md). Competitor model-weight file hashes are still missing from their receipts; version snapshots do not close that reproducibility gap.
- All four candidate models pass the published-score reproduction gate in `lab/model_checks.jsonl`. The heldout confirmation runner applies the original +0.01 dev bar to both corpora of a language group and never changes product defaults.

At queue completion, the reports include `HEADTOHEAD.md`, `LANCEDB-RESULTS.md`, `MODEL-CANDIDATES.md`, `MODEL-LATENCY.md` and `MODEL-CONFIRMATION.md` under `experiments/`. Read source receipts and cost measurements before proposing a winner. Full production integration, memory budgets, crash safety and consequential licensing decisions remain separate gates. No push or production-default change is part of this queue.

## Future optimization backlog in Beads

Captured at Johan's request on 2026-11-13. Epic `bk-1tc`; all six children are open. These are planned investigations, not measured gains or changes to the active queue.

- `bk-1tc.1`: profile each stage, then reduce repeated builds/encoding with correct cache invalidation and separate cold/warm measurements.
- `bk-1tc.2`: controlled version comparison for NumPy, ONNX Runtime and tokenizers in isolated environments.
- `bk-1tc.3`: compare OpenVINO CPU with ONNX Runtime CPU on this Intel host.
- `bk-1tc.4`: evaluate a native Rust encoder with identical models and recipes, including end-to-end overhead.
- `bk-1tc.5`: evaluate Polars for corpus preparation and result tables if profiling justifies it. NumPy remains appropriate for dense numerical arrays.
- `bk-1tc.6`: diagnose the non-reproducible Cognee lexical receipt without deleting it or weakening validation.

Tasks `.2` through `.5` depend on `.1`. Each task records acceptance criteria. Inspect with `rtk proxy bd --sandbox show bk-1tc`; list with `rtk proxy bd --sandbox list --parent bk-1tc`. Wait for the existing measurement queue before starting heavy experiments. No product dependency or default was changed when recording this backlog.

## Done

- Harness validated (BM25 SciFact 0.663 vs published 0.665; lab encoders reproduce published MTEB scores for MiniLM, E5-small, Bekko).
- 10 product-engine receipts (lexical: `shipped-lexical`, `bm25-or-lexical` on syntec, scifact, nfcorpus, alloprof, longmemeval_s) and 2 hybrid receipts (syntec, `shipped-hybrid` 0.669 and `shipped-hybrid-e5` 0.780). They carry the old harness hash if `lab/` changed since: re-run `run_suite.py` before tagging `bench-v1`.
- Lab engine, sweeps `lexical` (done), `models` (running), splits (dev = official train/dev for SciFact/NFCorpus, hash 30 percent for the rest; report on `heldout`/`test`).

## Findings so far (evidence in `receipts/` and `sweeps/*.jsonl`)

1. The shipped lexical path ANDs every query word: SciFact nDCG@10 0.017 (295/300 queries empty) vs 0.663 with OR.
2. Hybrid in the product is therefore close to dense-only on natural-language questions.
3. Published dense scores (anchors.json): the shipped MiniLM-L12 is far below E5-small and Bekko on French (Alloprof 0.266 vs 0.274 vs 0.450) and below BM25 on SciFact (0.485 vs 0.665).
4. Stemming: +0.02..0.03 on English; on the mixed index one stemmer beats the union of both (0.469 vs 0.452).

## Running / next (in order)

```
# 1. dense model comparison on dev splits (background job; ~1 h)
bench/.labenv/Scripts/python.exe -I bench/sweep.py models --datasets scifact,nfcorpus,alloprof --split dev
# 2. pick the best EN and ML model, then fusion on top (pass the winner as --base-json)
bench/.labenv/Scripts/python.exe -I bench/sweep.py fusion --datasets scifact,nfcorpus --split dev --tag en --base-json '{"dense":{"model":"<en winner>"},"lexical":{"tokenizer":{"stem":"en"}},"chunker":{"kind":"whole"}}'
bench/.labenv/Scripts/python.exe -I bench/sweep.py fusion --datasets alloprof,mix-fr-en --split dev --tag ml --base-json '{"dense":{"model":"<ml winner>"},"lexical":{"tokenizer":{"stem":"fr"}}}'
# 3. chunking (alloprof, mix-fr-en, longmemeval via run_lme --split dev) then rerank (--max-queries 200)
# 4. freeze the two winning profiles, run them through run.py on heldout/test for receipts, write DECISION.md, run_suite.py, tag bench-v1
```

## Status 2026-11-12

Stages 1-6 done on dev, candidates validated on heldout with paired bootstrap (see `DECISION.md`). Provisional profiles: default `bekko-a8m` + BM25 (English stemmer, k1 1.5) + convex combination (z-score, alpha 0.55), whole documents; optional English variant `gte-small`; optional rerank mode `bge-reranker-v2-m3`.
Done since: test-split and LongMemEval heldout receipts, full `run_suite.py` (40 receipts), storage engine bake-off (`dbbake/`, see `DECISION.md`: Tantivy for lexical, exact in-memory vectors, SQLite stays the system of record).
Next: tag `bench-v1` (needs a commit, not authorized yet), copy tagged receipts to the vault, port the winners to the Rust core (Tantivy lexical layer, resident encoder, fusion), name and README.

## Open questions

- Dev results for chunking, rerank and language centering are in `DECISION.md` (stage 4-6). Remaining: confirm the winners on heldout, paired bootstrap, freeze two profiles (`en`, `mixed`).
- Cache-key bug fixed 2026-11-12 (chunked and whole texts shared an embedding cache key); `run_lme.py` no longer fails on lab engines.
- Port the winners into the Rust core: BM25 OR + stemming, resident encoder in the daemon, model choice, fusion weights.
- Database bake-off (sqlite-vec, Tantivy+USearch, LanceDB, Qdrant Edge) after the pipeline is fixed: the DB changes speed and operations, not exact-search precision.

## Rust port (started 2026-11-12)

1. DONE: lexical queries use OR (`app::match_expr`), tests updated (`tests/context.rs`). Product lexical: SciFact 0.017 -> 0.663, NFCorpus 0.189 -> 0.297, Syntec 0.000 -> 0.666, Alloprof -> 0.328, LongMemEval-S 0.015 -> 0.879. Receipts committed; Rust source is NOT committed (it shares files with the earlier hardening edits).
2. TODO: fusion = convex combination (z-score, alpha 0.55) instead of RRF, exact vectors in memory (`semantic.rs`).
3. TODO: Tantivy lexical layer (en_stem, derived index rebuilt from SQLite).
4. TODO: resident Rust encoder (`ort` + `tokenizers`, bekko-a8m, hash-locked) replacing `tools/embed.py` per-query helper (~1 s).
Gate for every step: product receipts through `bench/run.py` against the lab numbers in `DECISION.md`.
