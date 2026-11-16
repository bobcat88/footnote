# Local hybrid retrieval: implemented, measured, not globally accepted

## Verified artifact

- [Source-bound acceptance receipt](../artifacts/hybrid-acceptance/803ba94aa98b4f628abb4feb4557a323/receipt.json): **14 Rust tests**, real release OCR **1 test**, metric definitions **2 tests**, format check, strict Clippy, release build, Python AST checks and dependency compatibility passed.
- [Benchmark report](../artifacts/hybrid/0e2683fcfeee46199ecea871ba029f82/report.json), [raw results](../artifacts/hybrid/0e2683fcfeee46199ecea871ba029f82/raw-results.json), [frozen judgments](../artifacts/hybrid/0e2683fcfeee46199ecea871ba029f82/judgments.json), [corpus text/hashes](../artifacts/hybrid/0e2683fcfeee46199ecea871ba029f82/corpus.json).
- Every CLI call has raw stdout/stderr, argument list, exit status, timing and sampled process-tree RSS in that run directory. Query vectors and Qdrant cosine candidates/RRF scores are retained separately. Receipt binds **31 execution inputs** and **817 raw-evidence files**; all 228 CLI calls exited zero. These generated artifacts are ignored by Git and must be retained separately if sharing the uncommitted source tree.
- Release SHA-256: `1d30707fa78b023d3fe35a5bd4913fb0138e26ecf3a5fc6fc95eef060a5bd736`; 3,036,672 bytes. Official project-local Rust/Cargo 1.99.1, Windows MSVC; Python 3.11 environment pinned in [requirements-hybrid.lock](../requirements-hybrid.lock).

The former blocker was real but is resolved: Program Files contained an incomplete Rust installation (initial DLL failure, then missing standard-library metadata and fmt/clippy). The global installation was not changed. [Provisioner](../tools/provision_rust.py) downloaded official components, verified their distribution-manifest SHA-256, and installed under `artifacts/toolchain/installed`; [component receipt](../artifacts/toolchain/receipt.json). A native `D:/...` string cannot be prepended literally to a Bash colon-separated PATH; use `/d/...` in Bash and native paths for `RUSTC`/arguments.

## Runtime contract

Since 16 November 2026 `search` is hybrid by default when the corpus has vectors from models installed in Footnote's models folder
(found by manifest identity), and lexical FTS5 otherwise; `--mode lexical` forces words only. The historical explicit form:

```bash
K='D:/Projets/BorgKnowledge/target/release/borg-knowledge.exe'
DB='D:/Projets/BorgKnowledge/artifacts/hybrid-acceptance/803ba94aa98b4f628abb4feb4557a323/pilot-copy.db'
MODEL='D:/Projets/BorgKnowledge/artifacts/models/minilm'
# Already executed on this copy; explicit rebuild is a write operation.
"$K" --db "$DB" rebuild-embeddings --model "$MODEL"
"$K" --db "$DB" search 'réduire le coût des reçus inutilisés' \
  --mode hybrid --model "$MODEL" --technology 'Python/SQLite' \
  --tested-version sqlite=3.53.1 --status proposal --limit 1
```

This actual filtered release query returned **record 2**, still a proposal with current pinned evidence. Both real imported records were read back. Schema 3→4 migration, embedding rebuild and backup/restore were performed on a **new pilot copy**; records/history/sources/chunks matched exactly before/after. The original `artifacts/demo/knowledge.db` and historical receipts remain unchanged. The current binary refuses read access to schema 3; use the accepted copy or explicitly migrate a coherent backup through a write command.

Schema 4 adds only derived `embedding_models` and `embeddings` tables. Vectors are JSON float64 arrays in SQLite, **not sqlite-vec**, ANN, or an external server. Both models produce 384-dimensional CPU ONNX dynamic-int8 embeddings with attention-mask mean pooling and L2 normalization. Manifest hash binds immutable public repository revision, model/tokenizer hashes, query/passage prefixes and truncation length. Helper hash and item revision signatures invalidate reuse. Inputs/files are local; helpers contain no download/network call. Global agents remain unconfigured.

Rebuild embeds at most **10,000 chunks plus records combined**, under a write transaction; it does not replace authoritative records/history. Empty corpora are supported. Search uses a bounded exact cosine scan, semantic top60 + lexical top20, RRF `1/(60+rank)` with rank starting at one. Item order is deterministic by kind/ID on ties. The combined result limit is 1–20; records/results remain separate arrays, with `rrf_score` available for cross-array ordering. Scores are ranking values, **never confidence**. `eligible_embeddings` counts considered top60 candidates, not total embedding coverage.

Structured technology/project/status/exact-version filters are applied before semantic ranking and exclude prose chunks rather than pretending to infer document compatibility. Regression tests cover mismatched version, project, technology and status. Source/evidence freshness remains point-in-time hash verification. Changed/deleted/reindexed item signatures do not silently reuse old vectors; new/revised items require an explicit embedding rebuild for semantic coverage. Lexical retrieval can still return new items before that rebuild. Identical content is deduplicated.

Failure tests cover missing model index, zero-norm derived-vector corruption (observed RED, repaired), corrupt model artifact hashes with transaction rollback/database-byte preservation, changed helper recipe, 10,001-item bound and model identity changes between request/response. No reranker, source-authority reasoning, automatic supersession transition or calibrated abstention was added.

## Evaluation protocol and limits

Exactly **1,000 owned synthetic Markdown documents**: 20 authored technical evidence documents and 980 gardening distractors. **32 fixed queries**, split **16 development / 16 holdout**; each has 14 answerable and 2 no-answer queries. Query definitions and relevance judgments were written before the first run. There was no model/RRF tuning. Splits have different queries and mostly different technical topics, but version/contradiction topics overlap: this is **not topic-disjoint independent human validation**. Judgments are small manually declared evidence sets, not exhaustive expert annotation of every possibly related document.

Categories include exact identifiers/errors, English paraphrase, French→English, English→French, incompatible versions, withdrawn advice, contradictory sources, prompt-injection context and missing answers. Contradictory sources are both relevant **evidence of the conflict**, not equally valid recommendations. Document retrieval cannot enforce a version/status policy from prose; the separate structured-record regression tests establish that exact filters work for records.

FTS5 and SQLite hybrid are the actual optimized executable, limit5, two measured invocations/query. The timing sample is a mixed-query, sequential, OS-cache-warm workload with **fresh model/session/process each call**, not hardware-cache-cold storage and not a persistent warm inference service. Index and 32 additional lexical top20 calls supply identical RRF candidates to the isolated Qdrant adapter. Total: **228 CLI calls**; quality evaluated once/query/engine/model, not inflated by repeats.

R@5 is macro recall over answerable query judgments; P@5 divides relevant returned evidence by five, counting absent slots as irrelevant. Most questions have only one relevant judgment, so their ideal P@5 is 0.2, not 1.0. MRR evaluates the first relevant result; nDCG@5 uses binary judgments. No-answer false positive means **any retrieved candidate**, not a hallucinated generated answer. No answer generator is evaluated.

| Engine/model | Split | R@5 | P@5 | MRR | nDCG@5 | Process p50 ms | p95 ms |
|---|---|---:|---:|---:|---:|---:|---:|
| FTS5 | development | 0.464 | 0.100 | 0.500 | 0.472 | 6.47 | 8.98 |
| FTS5 | holdout | 0.286 | 0.057 | 0.286 | 0.286 | 6.23 | 9.40 |
| SQLite hybrid / E5-small | development | 0.929 | 0.200 | 0.762 | 0.804 | 1005.17 | 1125.39 |
| SQLite hybrid / E5-small | holdout | 0.929 | 0.200 | 0.610 | 0.689 | 988.64 | 1089.91 |
| SQLite hybrid / MiniLM | development | 0.929 | 0.200 | 0.833 | 0.857 | 954.99 | 1013.76 |
| SQLite hybrid / MiniLM | holdout | 0.929 | 0.200 | 0.810 | 0.840 | 956.25 | 1032.80 |

Both hybrid models still miss `réduire le coût des reçus inutilisés` on the **unfiltered synthetic document** query. E5 also misses the held-out unchanged-filename question; MiniLM misses the held-out local-documents/cloud question. The successful filtered real-record pilot query is a different corpus/structured-filter condition and does not erase that failure.

**All four no-answer questions return five candidates under either hybrid model; FTS5 returns empty for all four.** `answerability: not_estimated` is deliberate. Each hybrid model returns four marked version-incompatible/withdrawn evidence occurrences across both splits; FTS5 returns three. These counts mark retrieved risks, not actual recommendations made. No automatic advice promotion is safe. Citation metadata/hash/text checks passed for **333 selected text results** across lexical and SQLite-hybrid evaluated outputs; this proves fixture provenance, not recommendation truth or citation entailment.

MiniLM ranks the small judged set better in this run, but neither model is selected as a universal default. Lexical remains the default. No seven-query hit rate is being called precision or used as a storage decision.

## Isolated Qdrant comparison

Qdrant-client **1.16.2 local persistent mode**, exact cosine, new per-run/per-model paths only. It was seeded with the **same 1,000 stored vectors** and queried with the same recipe and independently executed query embeddings. Top60 semantic candidates + the same lexical top20 feed the same RRF. Ordered top5 matched SQLite for **all 32 queries per model**; quality metrics therefore match. This establishes retrieval parity for this adapter/corpus, not superiority of either engine.

This is **not a Qdrant server/HNSW/network/concurrency benchmark**. The existing `D:/Second Brain/.qdrant_storage` was never opened or modified. A read-only loopback connection probe did not establish a successful port6333 connection (`connect_ex=10035`); no server acceptance is claimed. The old 2,816-dimensional collection has unknown model identity and cannot be reused with these 384-dimensional vectors.

Qdrant vector-search-plus-RRF stage medians were about **2 ms**; separate query-model/session load + encoding medians ranged **699–799 ms**. These in-process stages omit CLI startup, output/citation hashing/serialization and some SQLite work: **do not compare 2 ms against the full 956 ms CLI and claim an engine speedup**. Raw Qdrant whole-benchmark-process RSS is recorded but not an isolated service-memory measurement. Qdrant returns adapter IDs resolved against SQLite; the 333 citation checks certify the CLI outputs, not a separate Qdrant citation API.

## Indexing, disk and memory (measured, not a storage verdict)

| Item | Observed |
|---|---:|
| Lexical full index | 187.33 ms |
| Unchanged rescan | 116.34 ms |
| E5 full embedding rebuild, including model/helper startup | 4015.64 ms |
| MiniLM full embedding rebuild | 3864.63 ms |
| Lexical SQLite file | 1,024,000 bytes |
| SQLite after E5 only | 9,994,240 bytes |
| SQLite after both models | 19,185,664 bytes |
| E5 vector JSON alone | 7,999,636 bytes |
| MiniLM vector JSON alone | 8,015,394 bytes |
| Each closed local-Qdrant directory | 4,145,675 bytes |
| E5 model directory | 135,391,794 bytes |
| MiniLM model directory | 135,391,950 bytes |
| Largest sampled hybrid CLI process-tree RSS | 551,215,104 bytes |
| Largest evaluated hybrid JSON response | 4,045 bytes |

Qdrant seed time was 2061.84 ms for E5 and 1986.70 ms for MiniLM, **excluding embedding computation** because it reused SQLite vectors. SQLite includes authoritative text/FTS and JSON float64 vectors; Qdrant has only the derived vector store. On-disk formats and coverage differ. This is evidence that the current JSON representation has overhead, not a fair engine storage verdict. RSS sampled every10ms can miss peaks and count shared pages twice. Both rebuilds reported zero passages reaching the token limit; longer real technical notes may truncate, especially MiniLM's 128-token setting versus E5's 512.

## Repeat / remaining acceptance

```bash
# Only needed when system Rust is incomplete. Public official download, local install:
rtk python tools/provision_rust.py
export PATH='/d/Projets/BorgKnowledge/artifacts/toolchain/installed/bin':"$PATH"
export RUSTC='D:/Projets/BorgKnowledge/artifacts/toolchain/installed/bin/rustc.exe'
uv pip sync --python .venv/Scripts/python.exe requirements-hybrid.lock
.venv/Scripts/python.exe tools/provision_embeddings.py
.venv/Scripts/python.exe tools/verify_hybrid.py
```

The Python acceptance runner discovers the provisioned local toolchain automatically, logs the complete gate, benchmarks both models and copies/migrates the historical pilot without changing it. It requires the existing pilot as an input. Historical lexical `tools/verify.py` remains separate and is not the hybrid acceptance runner.

Before global activation: approve a bounded **real technical** corpus and independent judgments; add/review answerability and authoritative/superseded advice handling; profile the large fresh-process inference cost; decide whether a persistent helper, compact vectors or optional reranking earns its complexity. Only then consider a separately authorized server comparison. Lifecycle `bk-doj`, real-vault quality acceptance, MCP transport and global enablement remain open. No commit, staging, push, protected policy edit, vault ingestion or BorgInvestor write occurred.
