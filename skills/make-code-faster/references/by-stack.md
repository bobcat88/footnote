# By stack

## Rust
- `--release` with the shipped profile + features; debug timings mean nothing
- Profilers: `cargo flamegraph` (Linux perf, macOS DTrace) / `samply` (all three) / WPR or Superluminal (Windows). Keep symbols while profiling (`debug = "line-tables-only"`, `strip = false`)
- Order: algorithm or data structure → fewer allocations (reuse buffers, `with_capacity`, borrow, filter before clone) → buffered + batched I/O → `rayon` → crate swaps
- Crate swaps after bench-gate on real payloads: `simd-json` / `mimalloc` (jemalloc has no MSVC) / `parking_lot` for contended locks / PGO via `cargo-pgo`
- `criterion` for micro / hyperfine end-to-end for what users feel / allocations (dhat, heaptrack) separate from peak RSS

## Python
- Time: `python -m cProfile -o out.prof` / `py-spy record` on a live process. Memory: `tracemalloc` for Python objects, whole-process peak for "does it fit"
- Order: algorithm + data access → vectorise (NumPy, pandas, Polars) → Polars lazy + `explain()` for pushdown → `iter_rows` over materialised dicts → Numba or a PyO3 extension
- Parity: nulls / dtypes / order / float semantics identical; test on real data
- asyncio only for concurrent I/O, bounded by a semaphore / process pools pay startup + pickling, include them / free-threaded builds aren't a win until measured
- `orjson` for hot JSON / Pydantic v2 for validation; measure before widening

## Web, React
- Production build + Performance panel; React Profiler is a profiling build with overhead
- Pages: LCP ≤ 2.5 s / INP ≤ 200 ms / CLS ≤ 0.1 at p75 of real visits; lab diagnoses, only field proves INP
- LCP → server time, discover hero early, never lazy-load it / INP → split long tasks, less JS, off-main-thread / CLS → reserve media + font space
- React: state down / narrow context + store subscriptions / `memo` only for measured expensive renders / virtualise long lists / split routes / keep focus, scroll, loading + error states
- Bundles: say compressed or not, initial vs deferred

## Desktop (Tauri, Electron)
- Startup: launch → first paint → first usable action, on the installed release build
- Memory: whole process tree (WebView children) after a fixed settle
- IPC: measure real payload sizes before batching or changing transport
