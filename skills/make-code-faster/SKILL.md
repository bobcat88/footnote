---
name: make-code-faster
description: Use when code, a query, a build, app startup or a page is too slow or uses too much memory, or before claiming any speed-up: name the workload, profile, change one thing, decide from repeated measurements. Rust, Python (pandas, Polars, asyncio), React and web, desktop apps. Runs bench-gate to turn baseline and candidate samples into promote, inconclusive or reject.
license: MIT
---

# Make code faster

No speed claim without a baseline and a candidate measured the same way. Faster + wrong = failure.

## Method
1. Workload: exact command or user action / input + size / build mode / machine / cold or warm / budget from the product
2. Baseline + the output that proves correctness
3. Profile before changing (separately from final timing: profilers add overhead)
4. Biggest cost first: algorithm → data access (I/O, queries, allocations) → parallelism → micro-tuning / one change at a time
5. Candidate measured identically / output matches / gate

## Run
```
hyperfine --warmup 1 --runs 10 --export-json base.json '<old>'
hyperfine --warmup 1 --runs 10 --export-json cand.json '<new>'
node <skills>/make-code-faster/scripts/bench-gate/bench-gate.mjs base.json cand.json [--min 0.02] [--unit s]
```
- Also takes a JSON array or one number per line → memory / bytes / latency (lower = better)
- Two-sided Mann-Whitney → **promote** exit 0 / **inconclusive** exit 3 (noise or < `--min`: keep the simpler code) / **reject** exit 1
- ≥ 5 runs per side (3 can never reach p < 0.05) / cold and warm separately
- Tests: `node --test scripts/bench-gate/tests`

## By stack → [references/by-stack.md](references/by-stack.md)
- Rust: release profile / flamegraph or samply / clones + allocations in hot loops / `criterion` / crate swaps only after the gate
- Python: cProfile or py-spy / `tracemalloc` ≠ process memory / vectorise or Polars lazy / bound async / count worker startup + pickling
- Web, React: production build / real interaction trace / CWV for pages / memo, virtualise, split only what the trace shows
- Databases: `EXPLAIN` first → `manage-databases` / desktop: startup + process-tree memory → `ship-desktop-app`

## Report
Workload / machine / build / bench-gate table + decision / output parity / not measured (platforms, cold start, scale). Rows per second only with schema + operation + input size.
