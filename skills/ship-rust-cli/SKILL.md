---
name: ship-rust-cli
description: Use when building, changing or releasing a Rust command-line tool, daemon or library: CLI contract and exit codes, error and shutdown handling, Rust review pitfalls, default crates, release profile, Windows CRT and Linux glibc builds, and the measured gate before any speed claim.
license: MIT
---

# Ship a Rust CLI

Done = `cargo fmt --check` / `cargo clippy --locked -- -D warnings` / `cargo test --locked` pass, contract holds, release binary run on every shipped target.

## Contract (write it before the code)
- stdout = data / stderr = talk / `--json` = one stable document
- Exit: 0 ok / 1 failure / 2 usage (clap default) / others named in `--help`; partial batch failure → non-zero + which items failed
- Batch exits when done / daemon on Ctrl+C or SIGTERM: stop intake → finish or checkpoint within a deadline → flush logs → release locks → exit 0
- State in platform folders (`%LOCALAPPDATA%`, XDG) / env var override so tests never touch real state
- Never print secrets; redact tokens in errors and logs

## Defaults (unless the repo already chose)
- `clap` derive / `anyhow` apps / `thiserror` libs / `tracing` / `serde`
- `tokio` only for real concurrent I/O / `rayon` for CPU / `rusqlite` sync, `sqlx` async / `jiff` dates / `ignore` repo walks / `criterion` micro-benches
- `simd-json` / `mimalloc` / `parking_lot` only after `bench-gate` (make-code-faster) wins on your workload

## Review pitfalls
- `.clone()` to quiet the borrow checker / cloning before filtering → filter, then clone or borrow
- Params: `&str` / `&[T]` / `impl AsRef<Path>`, not owned types, unless kept
- Fast path + general path share one rule function + a parity test
- No `unwrap`/`expect` on input, I/O, network; `with_context` on errors; a panic is a bug
- `unsafe` needs `// SAFETY:` + a test (Miri where it runs)
- Blocking file / CPU / SQLite work in async → `spawn_blocking`

## Release
- Profile: `lto = "thin"` / `codegen-units = 1` / `strip = "symbols"`; dev+test `debug = "line-tables-only"` if linking is slow
- `--locked` in CI and release / commit `Cargo.lock` for binaries
- Windows: CRT must match every prebuilt native lib (most C++ prebuilts = dynamic `/MD`; `crt-static` then fails to link)
- Linux: binary needs the build runner's glibc or newer → build on the oldest runner your native deps allow / musl only if all deps support it
- Run the built artifact (`--version` + one real command) per target / checksums / signing per `fix-ci-and-releases`

## Speed claims
`hyperfine --runs 10 --export-json` baseline and candidate → `bench-gate` (make-code-faster). No gate, no claim.
