# Documentation map

The product README is [../README.md](../README.md) and the cross-platform Linux/Windows setup & LLM testing guide is [../HOW-TO.md](../HOW-TO.md). This folder holds architecture, operations, roadmap, and design documents for Footnote.

## Current

| Document | What it is |
|---|---|
| [`../bench/README.md`](../bench/README.md) | The benchmark protocol: datasets, splits, receipts, determinism, decision rules |
| [`../bench/DECISION.md`](../bench/DECISION.md) | Decision rules written before the results, and every outcome: models, fusion, chunking, rerank, storage engine |
| [`../bench/experiments/RESULTS.md`](../bench/experiments/RESULTS.md) | The 15 optimization experiments, heldout confirmation, E12 reranker, E14 latency at scale |
| [`../bench/INDEX.md`](../bench/INDEX.md) | Index of benchmark reports with statuses and line links |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Engine schema, layers, and safety boundaries |
| [`OPERATIONS.md`](OPERATIONS.md) | Limits, privacy, recovery, and the audit log |
| [`DEFINITION-OF-DONE.md`](DEFINITION-OF-DONE.md) | Completion checklist for PRs and stories |
| [`ISSUE-RULES.md`](ISSUE-RULES.md) | Issue structure, label taxonomy, Epic nesting, and AI ownership tracking |
| [`RUST-RULES.md`](RUST-RULES.md) | Rust API, error handling, store discipline, and performance rules |
| [`security/AUDIT-2026-11-18.md`](security/AUDIT-2026-11-18.md) | Internal security audit findings and fixes |
| [`roadmap/PRIORITY-QUEUE.md`](roadmap/PRIORITY-QUEUE.md) | Value-ordered implementation queue across open GitHub issues (`scripts/pmo_queue.py`) |
| [`roadmap/BACKLOG.md`](roadmap/BACKLOG.md) | Planned features and open research tracks |
| [`roadmap/EPICS.md`](roadmap/EPICS.md) | Epic structure matching GitHub Issues |
| [`roadmap/EDITIONS.md`](roadmap/EDITIONS.md) | Footnote One (Apache-2.0) vs Enterprise boundaries |

## Design & Research

| Document | Topic |
|---|---|
| [`AGENT-MEMORY-DESIGN.md`](AGENT-MEMORY-DESIGN.md) | Agent memory design and hook protocol |
| [`CODE-GRAPH-DESIGN.md`](CODE-GRAPH-DESIGN.md) | Code graph design, two-tier indexing, and architecture map |
| [`FEDERATION-DESIGN.md`](FEDERATION-DESIGN.md) | Federated multi-index enterprise design |
| [`research/`](research/) | Technical studies (code graph, docs crawler, embeddings, remote access, token economy) |
| [`adr/`](adr/) | Architecture Decision Records |
