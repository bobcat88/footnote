# ADR 0001: one repository, a shared engine, one crate per product

Accepted 15 November 2026.

## Context

Footnote is becoming a family of products: Library (document corpora), Memory (agent memory), later Code (code graph) and Alias
(pseudonymisation), behind a launcher that switches them and their options on or off. Until now everything lived in one Rust
crate (6,300 lines), with the command line, the storage engine and both products mixed in `src/app.rs`, and the top of the
repository mixed product code, design passes and research scripts.

## Decisions and consequences

| Decision | Consequence |
|---|---|
| **One repository** for every product (monorepo, Cargo workspace) | A change to the shared engine reaches every product in one commit, one CI, one version. Products are not released separately; a product moves to its own repository only when it gets its own team. |
| **One executable**, `footnote`, each product a compile-time feature (default: all) | A company can receive a build without a product it does not use (less code to audit). At run time the launcher switches products and options; a product that is off loads nothing. |
| **Layers:** `footnote-core` (state, configuration, audit log, metrics, safe file access) → `footnote-search` (storage schema, keyword and vector search, fusion, records) → products (`footnote-library`, `footnote-memory`) → the binary (command line, MCP server, daemon, hook, install) | Dependencies point one way only. Products share the engine (one engine, tools switched on or off) and never depend on each other. |
| **The binary stays the root package** (`src/`), libraries live in `crates/` | Build commands, bench scripts and test paths stay unchanged (`cargo build --release --features metrics`). |
| **Launcher: Tauri 2**, a native window with a web interface and Rust logic, calling `footnote` for everything | Windows first, Linux and macOS from the start. All logic stays in the engine, so every launcher action also works from the command line and in tests. |
| **One schema history** in `footnote-search`, including the memory table | Corpus and memory databases keep one file format and one migration order; a product cannot fork the schema. |
| **Crates are created when their product starts** (`code`, `alias`) | No empty scaffolding. |

## Layout

```
Cargo.toml, src/        the footnote binary: command line, MCP server, daemon, hook, install
crates/core/            footnote-core
crates/search/          footnote-search (the schema lives in store.rs)
crates/library/         footnote-library: extraction, corpus indexing, later documentation crawling
crates/memory/          footnote-memory: remember, recall, pack, forget
apps/launcher/          Tauri launcher
bench/                  benchmarks and experiments (Python, drive the real binary)
docs/                   architecture, ADRs, roadmap, operations
scripts/                checks, release, install
```

Kept at the root on purpose: `DESIGN.md` and `PRODUCT.md`, and `tools/`: runtime helpers `tools/extract.py` and `tools/provision_embeddings.py` live there.

## Practices

Decisions as ADRs in `docs/adr/`; CI on every push (tests, clippy, format through `scripts/check.sh`); semantic versions and a
`CHANGELOG.md`; one branch per piece of work, merged by pull request; GitHub Issues and `docs/roadmap/BACKLOG.md` for priorities.

## Notes

Renamed from the pre-open-source prototype name to `footnote` across binary, crates, state directories, and documentation.
