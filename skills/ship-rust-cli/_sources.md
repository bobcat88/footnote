---
skill_hash: ff3e34e6cb342d73
reviewed: 2026-11-21
---
# ship-rust-cli: sources

Maintainer file, not synced to agents. `skill-check --stamp` records the skill hash after a review.

| Claim | Source |
|---|---|
| CLI output, exit codes, signals | [Command Line Interface Guidelines](https://clig.dev/) |
| API conventions (borrowed params, errors) | [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) |
| Release profile settings | [Rust Performance Book: build configuration](https://nnethercote.github.io/perf-book/build-configuration.html) · [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) |
| Static vs dynamic CRT | [Rust reference: C runtimes](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes) · [Microsoft CRT library features](https://learn.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features) |
| Blocking in async | [tokio::task::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) |
| Benchmark tool | [hyperfine](https://github.com/sharkdp/hyperfine) |

## Field evidence
- A release failed to link on both targets after an ONNX Runtime upgrade: the library needed glibc 2.38 (ubuntu-22.04 has 2.35), and its Windows prebuilt used the dynamic CRT while the packager defaulted to `crt-static`. Fixed by building on ubuntu-24.04 and switching to the dynamic CRT.
- Crate defaults come from a private library matrix (adopted in several projects, mostly without local benchmarks), hence the bench-gate rule for any "faster" crate.
