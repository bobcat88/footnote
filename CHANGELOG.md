# Changelog

Notable changes to Footnote, newest first. Versions follow semantic versioning. From 16 November 2026 the Unreleased section is
generated from Conventional Commits with git-cliff (`cliff.toml`); earlier entries were written by hand.

## Unreleased

### Added

- **library:** Documentation crawler, system TLS roots, overnight benchmarks
- **library:** Adaptive per-host crawl concurrency (3.6x faster, same pages); chunks carry their heading path, code comments are no longer headings
- **search:** Chunks follow page structure, code blocks stay whole

### Fixed

- **library:** Crawler saves each page once (.md, HTML and trailing-slash forms)
- **library:** Use the main landmark when Readability picks a footer

### Documentation

- **library:** Documentation storage and crawler study; pre-registered crawl bench
- **roadmap:** Leaner agent output first; output and storage formats

### Tests and benchmarks

- **docs:** Fresh 40-question confirmation set and protocol, written before running; crawler keeps same-host redirect scope only
- **docs:** Confirmation verdict (worse), sealed held-out set 3 and optimisation protocol; crawler cites the HTML page for Markdown copies
- **bench:** Heading path kept (+0.044 nDCG@5 pooled on the dev sets, general guard unchanged)

### Build and CI

- Check commit messages; CHANGELOG from commits (git-cliff)
- Fix the commit check step (a raw newline made the workflow invalid)

## 0.1.0-alpha.1 - 2026-11-15

Private pre-release (GitHub Releases, Windows and Linux x64, built with dist).

### Added
- `footnote status`: what is installed, on, registered and running, and what memory did this week (the launcher's overview).
- `footnote options` and `footnote config get/set`: the options catalogue, memory on/off, and a company policy file
  (`%ProgramData%\footnote\policy.json`, `/etc/footnote/policy.json`) that settings cannot override.
- `footnote uninstall --memory`.
- Model manager: `footnote models list/install/use/remove/recommend`, with hardware detection (GPU vendor, VRAM, CPU, RAM);
  downloaded models stay out of any git repository.
- Memory scope is the repository's remote name (folder name as fallback); `footnote memory-rescope`.
- Live progress of long steps (`footnote jobs`, `<state>/jobs/<id>.json`), `footnote memory-list`, `footnote activity --days`.
- Release workflow (dist): GitHub Releases for Windows and Linux x64.

### Changed
- The repository is a Cargo workspace (ADR 0001): `footnote-core`, `footnote-search`, `footnote-library` and `footnote-memory`
  under `crates/`, the `footnote` executable at the root. Library and Memory are features of the executable, on by default.
- The memory hook runs in Rust (`footnote hook`); the Python hook is gone. Per-prompt recall about 24 ms in-process.
- Embeddings are computed in-process with ONNX Runtime; no Python on the search or memory path.
- Product site and QLoop passes moved under `design/`.

### Added
- Opt-in resident daemon for recall by meaning on each prompt (`"resident": true`, off by default until its relevance cutoff passes).
- Memory visibility: agents say when a memory shapes an answer and give a one-line receipt after each save.
- Opt-in metrics build (`--features metrics`) and `footnote metrics`.
- CI on Windows and Linux: format, clippy for every product combination, tests.

### Fixed
- Audit chain fork when two MCP sessions wrote at once.
- Long accented prompts failed memory recall (512-byte query limit).
