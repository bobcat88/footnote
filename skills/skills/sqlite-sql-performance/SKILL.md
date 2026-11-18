---
name: sqlite-sql-performance
description: Diagnose SQLite query latency, FTS5 ranking, transaction contention,
  WAL growth and index costs using representative data and query plans.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 44f2a861beb1eead3fc8efdd61f5204e6c9a059b64499e4fd83df2bbbaff4db6
  source-card: AI-Agents/skills/sqlite-sql-performance/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# SQLite SQL performance



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Inspect the query and workload

- Capture schema, SQLite version, row counts, bound parameter distributions and the exact read/write mix. Use a disposable copy for tuning experiments.
- Use `EXPLAIN QUERY PLAN` to investigate access paths. A scan is not automatically a bug; tiny tables and broad queries can legitimately scan.
- Measure candidate indexes against read latency, write latency and database size. Check result parity, ordering and tie-breaking.
- Do not parse EXPLAIN's human-readable detail as a stable API. Store it as diagnostic evidence.
- WAL permits concurrent reading and writing but still has a single writer. Check transaction duration, busy handling and checkpoint behavior. WAL is unsuitable for clients sharing a database over a network filesystem.
- For FTS5, freeze tokenizer, query syntax and relevance fixtures. Its bm25 convention ranks better matches with lower numeric values; verify sort direction in the real query.

## Measurements and checks

Run the representative query set with declared cold/warm conditions; report query p50/p95, rows returned, failures/busy retries, write throughput, database/WAL bytes and checkpoint duration. Latency budgets belong to the project. A changed result set or lost writes invalidates a speedup.

Start with read-only inspection. Do not issue journal-mode changes, VACUUM, REINDEX or migrations on a live database as a diagnostic shortcut. Preserve restore and concurrency tests for a shipping storage change.

## Candidate helper for round 2

Replay parameterized read-only query fixtures against a temporary database and produce parity, latency and size receipts. Require explicit opt-in for mutation experiments; never accept arbitrary SQL from retrieved content.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://www.sqlite.org/eqp.html
- https://www.sqlite.org/wal.html
- https://www.sqlite.org/fts5.html


## Shipped measurement helper

Run `rtk proxy python scripts/compare_receipts.py before.json after.json` from this skill directory. It uses Python 3.10+ standard library only. It reads explicit JSON receipts and writes JSON to stdout; no network or command execution. Without a caller-provided `--max-regression-pct`, exit 0 means valid measurement, not acceptance. Exit 1 means over-budget or inconclusive; exit 2 means invalid input. See [receipt contract](references/measurement-receipts.md) before first use.
