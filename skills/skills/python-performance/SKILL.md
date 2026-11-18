---
name: python-performance
description: Profile and improve Python data processing or tooling with correctness-preserving
  time and memory measurements. Use for Python bottlenecks, not routine typing or
  packaging work.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: a162aa2c93e8b3107b292b59e3e2306d79ac5cfacf6032528220a2d37f775951
  source-card: AI-Agents/skills/python-performance/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Python performance



## Find the limiting resource

Capture the representative command and input, environment lock revision, Python version, wall time and process memory. Use the project's profiler, or version-appropriate standard-library profiling. Profile separately from final timing.

Use tracemalloc for Python-tracked allocation snapshots and differences; it does not establish total process memory or all native allocations. Measure whole-process peak memory for fit-to-machine claims. For a pipeline, distinguish parsing, transformation, external calls and serialization.

Optimize the dominant algorithm or data access pattern before adding parallelism. Use Polars lazy scans and inspect plans where the project already uses Polars; do not mandate a library migration. Preserve null, dtype, ordering and numeric semantics when replacing loops or pandas operations. Measure I/O-inclusive throughput on the same workload.

For asyncio, measure useful concurrent I/O and bound outstanding work. For CPU parallelism, include worker startup, serialization and native-library thread contention. Do not infer speed from CPU count or free-threading availability.

## Acceptance and evidence

**Ship:** the target workload produces equivalent results and meets its project-owned time/memory budget. Record baseline and repeated candidate samples under the same conditions; identify noisy or inconclusive differences.

**Best:** additionally retain stage profiles and receipts, validate representative large inputs and failure/restart behavior, and verify performance in the deployment environment.

Report rows/second only with schema, operation and input size. No universal throughput, free-RAM fraction, test-duration or import-time cutoff is justified for every Python project. If the budget is unspecified, propose it from the actual user flow and target machine.

Use existing checks, for example `rtk uv run pytest`. Do not install packages or run costly benchmarks solely because they are listed in a reference. This skill runs in the current agent without orchestration.

## Candidate helper for round 2

Compare correctness-tagged pipeline receipts with matched input/environment IDs, raw durations and whole-process memory. Keep profiler output separate; reject missing or non-finite measurements.

## Sources

Source review: 2026-11-17. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental. Earlier drafts are preserved as project evidence, not active instructions.

- https://docs.python.org/3/library/profile.html
- https://docs.python.org/3/library/tracemalloc.html
- https://docs.pola.rs/user-guide/lazy/using/
- https://docs.pola.rs/user-guide/lazy/query-plan/
- https://docs.pola.rs/user-guide/misc/multiprocessing/
- https://docs.python.org/3/howto/free-threading-python.html

## Shipped measurement helper

Run `rtk proxy python scripts/compare_receipts.py before.json after.json` from this skill directory. It uses Python 3.10+ standard library only. It reads explicit JSON receipts and writes JSON to stdout; no network or command execution. Without a caller-provided `--max-regression-pct`, exit 0 means valid measurement, not acceptance. Exit 1 means over-budget or inconclusive; exit 2 means invalid input. See [receipt contract](references/measurement-receipts.md) before first use.
