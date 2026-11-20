# Rust coding & performance rules

Footnote prioritizes small, predictable binaries, memory safety, and evidence-backed changes. These rules complement `CONTRIBUTING.md` and `docs/DEFINITION-OF-DONE.md`.

## 1. Ownership & API design

- **Slice-first parameters:** Accept `&[T]`, `&str`, or `&Path` rather than `&Vec<T>`, `&String`, or `&PathBuf` unless the callee specifically mutates the container or transfers ownership.
- **Defer `.clone()`:** Keep references across read/filter paths and clone only at the boundary where an owned value or mutation is genuinely required.
- **Stream with iterators:** Prefer iterator chains and single-pass folds over allocating intermediate `Vec`s when transforming or filtering query hits.
- **`#[must_use]` on pure & result-producing helpers:** Mark constructors, builders, pure score/ranking functions, and validation results `#[must_use]` so silently dropped values fail CI clippy/compiler checks.
- **`#[non_exhaustive]` on evolving public types:** Add `#[non_exhaustive]` to public enums and error types that cross crate boundaries when new variants are expected.
- **Use `NonZeroU*` niches where zero is invalid:** Limits, shard counts, ports, and window capacities that cannot validly be zero should use `NonZeroUsize` / `NonZeroU32` / `NonZeroU64` so `Option<NonZeroU*>` occupies a single word and invalid zero states become unrepresentable.
- **Avoid multi-bool parameter lists:** Replace functions taking two or more adjacent `bool` arguments with two-variant enums or a typed options struct.
- **Zero glob imports in production code:** Spell out imported symbols explicitly outside `mod tests`.

## 2. Static initialization & panic hygiene

- **Hoist compiled regexes:** Compile static regular expressions once via `std::sync::OnceLock<Regex>` rather than calling `Regex::new` inside request or indexing loops.
- **No `.unwrap()` or `.expect()` in production paths:** Library and CLI code must propagate errors via `Result` with actionable context; reserve `.unwrap()` and `.expect()` strictly for tests.

## 3. Correctness & index/store discipline

- **Single source of truth for semantic rules:** Filter predicates, visibility scopes, cutoffs, and limits live in one shared helper rather than being duplicated across fast paths and slow paths.
- **Fast-path equivalence tests:** Any optimization that adds an early cutoff, bounded `SCAN_LIMIT`, or prefilter fast path must include a test asserting exact equivalence against the unoptimized full scan on edge cases.
- **Fail closed on index or store corruption:** If secondary index metadata or sidecar state is missing, truncated, or schema-mismatched, return a structured error (or force a verified rebuild path) instead of silently serving partial or empty results.

## 4. Benchmark-first performance gate

Never adopt a new crate or rewrite a hot path on intuition alone. Every performance change (`ignore` vs `walkdir`, `jiff` vs `time`, allocation pruning, concurrency changes) requires:

1. A benchmark hypothesis and decision rule written before measurement (`bench/experiments/PLAN.md`).
2. Dev and held-out latency/memory measurements (`bench/README.md`) with paired comparison intervals (`bench/compare.py`).
3. Proof that accuracy or precision does not regress alongside latency gains.

## 5. Windows MSVC & local build speed

These optional settings speed up local Windows developer iterations in `~/.cargo/config.toml` without changing `Cargo.lock` or release builds:

- Linker: `lld-link` (via LLVM/Visual Studio toolchain) to shrink incremental link times.
- Compiler cache: `RUSTC_WRAPPER = "sccache"` to share compiled dependencies across worktrees.
- Lighter debug symbols in dev profiles: `debug = "line-tables-only"` when full variable inspection is unnecessary.
- Test runner: `cargo nextest run` for parallel per-binary test scheduling.
