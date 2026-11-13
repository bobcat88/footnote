# Documentation map

The product README is [../README.md](../README.md). This folder holds the longer records. Some of them were written during the **pilot**, when the project was called
BorgKnowledge and ranked with FTS5, AND-matching and RRF over one MiniLM model. They are kept because their evidence is still real, but their conclusions about
ranking are superseded by the benchmark protocol in [`../bench/`](../bench/README.md).

## Current

| Document | What it is |
|---|---|
| [`../bench/README.md`](../bench/README.md) | The benchmark protocol: datasets, splits, receipts, determinism, decision rules |
| [`../bench/DECISION.md`](../bench/DECISION.md) | Decision rules written before the results, and every outcome: models, fusion, chunking, rerank, storage engine |
| [`../bench/experiments/RESULTS.md`](../bench/experiments/RESULTS.md) | The 15 optimization experiments, the heldout confirmation, E12 reranker, E14 latency at scale |
| [`OPERATIONS.md`](OPERATIONS.md) | Limits, privacy, recovery, the audit log (scan bounds and hybrid limits updated) |

## Pilot-era records (historical)

| Document | Status |
|---|---|
| [`PILOT.md`](PILOT.md) | The pilot README. Names and commands are the old ones |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Schema and boundaries still apply; the ranking sections describe FTS5 and RRF |
| [`HYBRID.md`](HYBRID.md), [`VERIFICATION.md`](VERIFICATION.md) | Pilot acceptance evidence for the first hybrid implementation (MiniLM, E5, RRF) |
| [`QDRANT-QUALITY.md`](QDRANT-QUALITY.md), [`SERVER-BENCHMARK.md`](SERVER-BENCHMARK.md) | The Qdrant comparison. Superseded for storage by the bake-off in `bench/DECISION.md`: SQLite, Tantivy and an exact in-memory scan |
| [`TECHNICAL-QUALITY.md`](TECHNICAL-QUALITY.md), [`KNOWLEDGE-RUNTIME.md`](KNOWLEDGE-RUNTIME.md) | Pilot harness and an opt-in personal runtime |
| [`benchmarks/`](benchmarks/), [`research/`](research/), [`roadmap/`](roadmap/), [`migration/`](migration/) | Pilot experiments, literature notes and plans |

Pilot documents contain local absolute paths from the machine they were produced on, and use the old executable and tool names (`borg-knowledge`, `borg_*`).
The `borg_*` tool names still work as aliases in the current server.
