---
skill_hash: 83858dd588594f32
reviewed: 2026-11-21
---
# make-code-faster: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| Profile release builds, keep symbols | [Rust Performance Book: profiling](https://nnethercote.github.io/perf-book/profiling.html) |
| Benchmark method, noise | [Rust Performance Book: benchmarking](https://nnethercote.github.io/perf-book/benchmarking.html) |
| hyperfine JSON export | [hyperfine](https://github.com/sharkdp/hyperfine) |
| Mann-Whitney U, exact small-sample p | [NIST Dataplot: Mann-Whitney](https://www.itl.nist.gov/div898/software/dataplot/refman1/auxillar/mannwhit.htm) |
| PGO gains (~1% instructions on rustc) | [Kobzol: cargo-pgo](https://kobzol.github.io/rust/cargo/2023/07/28/rust-cargo-pgo.html) |
| flamegraph, samply, criterion | [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) · [samply](https://github.com/mstange/samply) · [criterion](https://bheisler.github.io/criterion.rs/book/) |
| cProfile, tracemalloc scope | [Python profilers](https://docs.python.org/3/library/profile.html) · [tracemalloc](https://docs.python.org/3/library/tracemalloc.html) |
| py-spy | [py-spy](https://github.com/benfred/py-spy) |
| Polars lazy, query plans | [Lazy API](https://docs.pola.rs/user-guide/lazy/using/) · [Query plan](https://docs.pola.rs/user-guide/lazy/query-plan/) |
| Free-threading caveats | [Python free threading](https://docs.python.org/3/howto/free-threading-python.html) |
| React Profiler overhead | [React Profiler](https://react.dev/reference/react/Profiler) |
| CWV thresholds, fixes | [Web Vitals](https://web.dev/articles/vitals) · [LCP](https://web.dev/articles/optimize-lcp) · [INP](https://web.dev/articles/optimize-inp) |

## Field evidence (private projects, measured locally)
- Polars over pandas 5–13× on large frame ops; `iter_rows` over `to_dicts()` ~80% less memory, ~50% faster in a replay loop; DirectML 19.5× over a CPU loop for 5000+ assets. Kept here, not in the skill: they are one machine's numbers, not rules.
