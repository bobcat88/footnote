# Bench: protocol, contenders, rules

Status: **protocol v1, harness built and validated, optimization sweeps in progress**. The rules below were written before the first result so they cannot bend to fit the numbers.
Written 2026-11-11. Nothing here is a claim of superiority; it is how we find out.

> **Start at [INDEX.md](INDEX.md).** It lists every benchmark and report in the repository with its headline, status and a link to the exact line, and holds the figures the website may quote.
>
> **Convention for a new report:** add one row to `INDEX.md` in the same PR (stable id, date, question, headline copied from the source, status, line link). Name files `<TOPIC>-YYYY-MM-DD.md`. When a newer result replaces an older one, change the older row to `SUPERSEDED by <id>`; never delete rows or results. Move raw logs and scratch output to `archive/` with `git mv`, not into a new report. `python scripts/check_bench_index.py` (run by `scripts/check.sh`) fails if a `*RESULTS*`, `*REPORT*` or `*STATUS*` file under `experiments/` is not indexed or a link or line anchor no longer matches.

## What is in this folder

| Path | Role |
|---|---|
| `metrics.py`, `test_metrics.py` | nDCG/Recall/MRR/recall_any/all, hand-computed unit tests |
| `fetch.py`, `datasets.lock.json` | Public datasets (BEIR SciFact and NFCorpus, MTEB-French Syntec and Alloprof, LongMemEval-S, a French+English union `mix-fr-en`), SHA-256 locked, official dev sets or a fixed hash split |
| `anchors_fetch.py`, `anchors.json` | Published MTEB and BEIR reference scores with the results-repo commit |
| `engines.py`, `run.py`, `run_lme.py`, `run_suite.py` | Product engines (CLI as shipped) and the receipt-writing runners |
| `lab/` | Lab engine (resident ONNX encoders, own BM25, chunkers, fusion, cross-encoder rerank), model registry with hash lock, `check_model.py` validation gate |
| `sweep.py`, `sweeps/*.jsonl` | Tuning runs on dev splits (append-only rows) |
| `INDEX.md`, `archive/` | Index of all reports; moved raw logs and stale progress notes |
| `receipts/` | Immutable, reportable results (test split) ; `receipts/dev/` tuning receipts |
| `report.py`, `RESULTS.md`, `test_validation.py` | Report generator and the harness validation gate |

### Validation already done

- BM25 on SciFact: 0.663 (product FTS5 index, OR) and 0.661 (lab BM25) against a published 0.665.
- Lab encoder reproduces published MTEB scores on Syntec for MiniLM-L12 (0.628 vs 0.655), E5-small (0.752 vs 0.735), Bekko a8m/a25m (0.812/0.823 vs 0.803/0.816). `multilingual-e5-base` int8 fails the gate (0.779 vs 0.829) and is not used as configured.
- A stale-connection bug in the first LongMemEval smoke test produced an impossible 0.025 recall; the runners now assert that every returned id belongs to the queried corpus.

## Why this exists

Published memory-tool scores (LoCoMo, LongMemEval) measure LLM-judged answers on English chats. They cannot be compared with
passage retrieval on bilingual documents, and vendors dispute each other's figures. The only comparable evidence is the
**retrieval layer measured on public datasets with published reference numbers**, run by us, with every input pinned.

## Rules that make results comparable (never relax)

1. **Immutable receipts.** Every run writes `bench/receipts/<UTC>-<dataset>-<engine>-<config>.json` and never edits it. A re-run is a new file.
2. **Everything pinned.** Receipt records: dataset id + file SHA-256 (from `datasets.lock.json`), model files SHA-256, engine versions (crate/extension versions and lockfile hash), binary git commit + dirty flag + exe SHA-256, OS/CPU/RAM, protocol version.
3. **Determinism check.** Each configuration runs twice; the SHA-256 of the full ranked output must match. A config that is not deterministic is reported as such.
4. **Metrics fixed in advance.** nDCG@10 (linear gain, ideal DCG from the positive qrels), Recall@5/10/20, MRR@10, plus latency p50/p95 and index build time, disk size, peak RSS. Implemented once in `bench/metrics.py`, unit-tested against hand-computed values, and validated against a published reference number before any engine is trusted.
5. **Same inputs for every engine.** Same documents, same chunking, same embeddings (same ONNX model files) so a difference is the engine, not the model. Model changes are a separate axis, run on the winner only.
6. **Anchors are cited, not claimed.** Published numbers go in `anchors.json` with URL, retrieval date and commit/revision. Vendor self-reports are never used to decide.
7. **No tuning on the test split.** Parameters (BM25 k1/b, fusion constant, rerank pool) are fixed before the run or tuned on a dev split and then frozen; the receipt says which.
8. **Failures are results.** A crash, timeout, or unsupported feature is recorded, not skipped.
9. **Datasets live outside git** (`bench/.cache`, ignored); the lock file with hashes lives inside. Receipts, anchors and the lock are committed and tagged (`bench-v1`, ...). Copy the tagged receipts into the vault for a second durable location.

## Two baselines, kept apart

The shipped lexical path joins every query term with **AND** (`terms.join(" AND ")` in `src/app.rs`), no stemming, FTS5 `unicode61`.
That is a precision-first design for keyword queries from an agent, but on natural-language questions it requires every word to appear in
one chunk, so it is **not a BM25 baseline**. Measured lexical recall of 0.04 on the real public corpus partly reflects this, and part of
the reported hybrid gain is therefore not a vector-versus-BM25 effect. Report both:

- `shipped`: the CLI exactly as it is (AND, top 20 chunks, then RRF in hybrid mode).
- `bm25-or`: FTS5 with OR terms ranked by `bm25()`; the honest lexical baseline to compare against Tantivy, Lance FTS and Qdrant BM25.

## Datasets and anchors

| Track | Dataset | Why | Anchor to cite |
|---|---|---|---|
| EN retrieval | BEIR SciFact (5.2k docs), NFCorpus (3.6k) | Public, small enough for the current 10,000-chunk hybrid cap; BM25 and dense baselines published | BEIR paper BM25; `embeddings-benchmark/results` for the same models |
| FR / multilingual | MTEB-French retrieval tasks (Syntec, Alloprof; BSARD and others once the cap is lifted) | Same model family as ours has published scores | `embeddings-benchmark/results`: `sentence-transformers__paraphrase-multilingual-MiniLM-L12-v2`, `intfloat__multilingual-e5-small` |
| Memory-tool | LongMemEval-S, **retrieval only** (session-level Recall@k, no LLM) | The only memory-specific retrieval metric; AutoMem reports Recall@5 97.0% | LongMemEval paper (arXiv 2410.10813); AutoMem benchmark page |
| Real corpus | `docs/TECHNICAL-QUALITY.md` public technical corpus | Already built here; FR to EN questions; labels are author-made, **not independent** | Own history only |
| Scale | FiQA (57k) and a synthetic 100k/1M chunk set | Needs a vector index beyond the 10,000-chunk cap | Engine vendors excluded; our own runs only |

## Tracks

- **A. Lexical quality.** FTS5 (`unicode61`, `porter`, `trigram`), Tantivy (standard, French/English stemmers), LanceDB FTS, Qdrant Edge BM25, DuckDB `fts`. Differences come from tokenization, stemming, accent folding and BM25 parameters.
- **B. Vector index.** Exact vs ANN: recall@10 against exact, p50/p95 latency, build time, memory, disk, at 10k / 100k / 1M chunks. sqlite-vec (exact), USearch HNSW, LanceDB IVF-PQ and HNSW, Qdrant Edge HNSW, `hnsw_rs`. At exact search all engines return the same neighbors; only ANN settings change precision.
- **C. Integrated hybrid.** End-to-end on the same corpora: ingest time, **incremental update cost** (hash-driven re-index and tombstones), cold start, concurrent readers, crash safety, backup/restore, binary size and build time, Windows support, filter support (`technology`, `project`, `tested_version`, `status`), and whether provenance (hash-bound citations, read-only readers) survives the move.

## Contenders (facts checked 2026-11-11; versions from crates.io)

| Contender | Role | Verified | Watch-outs |
|---|---|---|---|
| **SQLite FTS5 + brute-force cosine** (today) | Baseline | Stable, one file, BM25, backup API | `unicode61` has no French stemming; vectors as JSON, capped at 10,000 chunks |
| **SQLite + sqlite-vec** 0.1.9 (MIT/Apache) | Evolutionary path | Same file as FTS5, exact KNN stable | Pre-v1; ANN indexes are alpha; fusion written in SQL by us |
| **Tantivy** 0.27 (MIT) + **USearch** 2.26.6 (Apache-2.0) | Best-of-breed libraries | Lucene-style BM25, language stemmers, very widely used; USearch actively released | We own storage, fusion, filters, transactions and provenance; two files to keep consistent |
| **LanceDB** 0.41 (Apache-2.0) | All-in-one | Rust crate; FTS with stemming incl. French, French stop words, ASCII folding, n-gram; IVF-PQ/HNSW; built-in RRF hybrid; dataset versioning | Heavy dependency tree (Arrow/DataFusion); in embedded mode **new rows stay outside the FTS index until `optimize()`**, scanned at query time; optimistic-concurrency write conflicts |
| **Qdrant Edge** (Apache-2.0, GA June 2026, `cargo add qdrant-edge`) | In-process, "SQLite for vector search" | Dense + sparse BM25 with full-text index, RRF fusion, payload filters; we already measured server Qdrant at equal top-5 to SQLite on 32/32 queries | Newer than server Qdrant; Edge concurrency under load not documented |
| **Turso DB** 0.8 (MIT) | Watch | SQLite-compatible Rust rewrite, active (updated 2026-11-10) | Pre-1.0; vector and FTS features to be verified before any run |
| **DuckDB** `fts` + `vss` | Reference only | Analytics engine; BM25 via `fts` | `vss` HNSW experimental, `fts` index rebuilt by hand after changes |
| **Infino** | Watch | Embedded BM25 + IVF/RaBitQ + RRF, Parquet storage (vendor description) | Newest entrant; licence and Windows support unverified |
| pgvector (+ParadeDB), Vespa, Milvus, Weaviate | Out of scope | Server-class | Violates local-first; kept only as a ceiling reference if needed. `milvus-sdk-rust` has a non-standard licence |
| Chroma | Excluded | Python-centric embedded store | Not usable from the Rust core without a sidecar |

## Order of work

0. Harness: `metrics.py` + tests, dataset fetch and lock, generic BEIR-format runner, receipts, anchors. Validate on SciFact with `bm25-or` against the published BM25 figure before trusting anything else.
1. Track A (cheapest, biggest expected effect for French), then B, then C.
2. Pick by the criteria table in `DECISION.md` (to be written before the results, with weights), not after.
3. Optimization on the winner: resident embedding model in the daemon (measured 3 ms encode vs ~1 s per process today), reranker budget (190 / 426 / 817 ms for 5 / 10 / 20 candidates on CPU, from `docs/benchmarks/RETRIEVAL-BUDGETS.md`), chunking (the encoder sees the first 128 tokens of a ~2,000-character chunk), abstention.

## Known limits of this bench

Author-made labels exist only for the in-repo corpus. Public datasets have their own label noise. LongMemEval is English. Results on one CPU
and one OS are not general. The bench ranks retrieval layers; it does not measure answer quality.
