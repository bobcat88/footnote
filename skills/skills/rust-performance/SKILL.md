---
name: rust-performance
description: Profile and improve Rust CLI or service latency, allocations, memory,
  startup or binary size. Use for measured Rust bottlenecks; do not redesign unrelated
  code.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 34d94eb59854478f00df1f7decb5aec55f7bbf2ca7f443ef212f1ab0c92b0800
  source-card: AI-Agents/skills/rust-performance/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Rust performance



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Diagnose before changing

- Benchmark the release configuration and real input sizes; debug-build timing does not establish shipped performance.
- Separate startup, steady-state work and shutdown. For services, measure queueing and concurrency as well as individual request execution.
- Select a platform profiler from the Rust Performance Book. Preserve symbols when needed; keep profiler overhead out of final wall-time measurements.
- Optimize the dominant algorithm, allocation or I/O cost first. Verify output parity before claiming a win; do not replace readable code for an improvement within measurement noise.
- Evaluate clone removal, buffer reuse or layout changes only where profiles justify them. Count allocations separately from peak process memory.

## Measurements and checks

Record p50/p95 request latency at a declared load, throughput, peak process memory and errors. For a CLI, record cold/warm startup and elapsed time on small and representative inputs. Report binary bytes with target, features and strip settings. Compare one change at a time.

Use existing repository checks, typically `rtk cargo test --locked` and `rtk cargo bench --locked`. Do not add nightly-only benchmarking to a stable project without a concrete need. A benchmark with fewer operations or different output fails parity.

## Candidate helper for round 2

A receipt comparator can verify matching input/build IDs and correctness before computing relative timing and memory differences. It must not declare a statistical win from a single sample.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://nnethercote.github.io/perf-book/profiling.html
- https://nnethercote.github.io/perf-book/benchmarking.html
- https://doc.rust-lang.org/cargo/commands/cargo-bench.html


## Shipped measurement helper

Run `rtk proxy python scripts/compare_receipts.py before.json after.json` from this skill directory. It uses Python 3.10+ standard library only. It reads explicit JSON receipts and writes JSON to stdout; no network or command execution. Without a caller-provided `--max-regression-pct`, exit 0 means valid measurement, not acceptance. Exit 1 means over-budget or inconclusive; exit 2 means invalid input. See [receipt contract](references/measurement-receipts.md) before first use.
