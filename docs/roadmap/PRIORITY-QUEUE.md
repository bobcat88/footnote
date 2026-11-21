# Local PMO Priority Queue

Value-ordered implementation queue for open Footnote GitHub issues.

- **Authority for issue state:** GitHub Issues (`bobcat88/footnote`).
- **Authority for execution order:** this queue, ranked by **Value / Effort** inside each tier.
- **Local CLI helper:** [`scripts/pmo_queue.py`](../../scripts/pmo_queue.py) syncs this queue against live GitHub issue state:
  ```bash
  python scripts/pmo_queue.py --next 10          # top 10 open issues to execute next
  python scripts/pmo_queue.py --tier P2          # pure-Rust & dependency-reduction track
  python scripts/pmo_queue.py --area engineering # filter by area
  python scripts/pmo_queue.py --check            # flag closed items still listed or unranked open issues
  python scripts/pmo_queue.py --offline --next 5 # offline read from this file only
  ```

## Scoring key

- **Impact:** `5` (product-wide / launch-blocking / major recurring savings) down to `1` (nice-to-have).
- **Effort:** `S` (< 1 day), `M` (1–3 days), `L` (3+ days or benchmark-gated spike).
- **Rule:** Never start a lower-tier item while an unblocked `P0` item remains open. Respect explicit `Depends` links (for example, resolve `#371` before `#129` locks the encrypted DB format).

---

## P0 — Now (Immediate Value / Effort & Public-Launch Gates)

| Rank | Issue | Epic | Area | Impact | Effort | Depends | Summary & Value Rationale |
|---:|---|---|---|---:|---|---|---|
| 1 | #379 | #91 | usage | 5 | S | — | Lean: idle-cache-expiry (>55m) handoff trigger & 140k default threshold (directly cuts cold 1h cache writes & long-context cache reads) |
| 2 | #380 | #91 | usage | 5 | S | — | Lean: `footnote audit-prefix` to rank Turn-1 fixed overhead across MCP servers, rules and skills |
| 3 | #335 | #93 | engineering | 5 | S | — | Pre-public checklist: final sign-off gate before repo and site go public (`#347` merged) |

---

## P1 — High Value (Core Speed, Retrieval Quality & Daily Agent Workflow)

| Rank | Issue | Epic | Area | Impact | Effort | Depends | Summary & Value Rationale |
|---:|---|---|---|---:|---|---|---|
| 4 | #355 | #93 | engineering | 4 | S | — | Rust perf & build speed: benchmark `ignore` vs `walkdir`, trim `semantic.rs` hot-loop allocs, evaluate `cargo-nextest` (`line-tables-only` done in #368) |
| 5 | #382 | #91 | usage | 5 | M | #355 | Lean C1b: native ripgrep-engine (`ignore` + `grep-searcher`) symbol-annotated search with `.gitignore` & output budget |
| 6 | #381 | #91 | usage | 4 | S | — | Lean D3: `footnote gain --missed`, uniform JSON-array compaction & consecutive repeat-run failure delta in `footnote run` |
| 7 | #344 | #84 | library | 4 | S | — | Library: expose shared docs cache over MCP, tighten keygen perms & align network docs |
| 8 | #356 | #83 | memory | 5 | M | — | Memory recall packing: benchmark dual relevance floors, pointer packing & compact subagent context |
| 9 | #357 | #84 | library | 5 | M | — | Embeddings: benchmark overlapping 510-token max-sim windows instead of hard truncation |
| 10 | #345 | #85 | code | 4 | M | — | Code: standalone `code-index` roots in MCP, `code-map` MCP mode & multi-indexer status |
| 11 | #240 | #91 | usage | 5 | M | — | Lean C1: graph-first & outline-first reads to cut whole-file `Read` tokens (7x larger than Bash volume) |
| 12 | #242 | #91 | usage | 4 | M | — | Lean C3: smaller fixed overhead and cache-stable hook/prompt injections (`#352` merged) |
| 13 | #241 | #91 | usage | 4 | M | — | Lean C2: memory-first recall before rediscovery |
| 14 | #238 | #91 | usage | 4 | M | — | Lean A: `footnote gain` real bill per session and per-session context breakdown |
| 15 | #343 | #86 | launcher | 4 | M | — | Launcher: bring Memory, Library/Docs and Usage screens to parity with Code and CLI |
| 16 | #239 | #91 | usage | 3 | S | — | Lean B: write less (lean code and lean prose rules) |

---

## P2 — Pure-Rust Replacements & Dependency Reduction (Zero C/C++/Python Runtimes)

| Rank | Issue | Epic | Area | Impact | Effort | Depends | Summary & Value Rationale |
|---:|---|---|---|---:|---|---|---|
| 17 | #369 | #93 | engineering | 4 | S | — | Replace `tokenizers` C `onig_sys` with pure-Rust `fancy-regex` or WordPiece (verified token-ID parity on all 4 models) |
| 18 | #358 | #94 | bench | 5 | L | — | Benchmark pure-Rust BERT GEMM (zero-ORT) & static lookup embeddings (`Model2Vec`); #1 binary-size lever (~47 MiB -> ~10 MiB) |
| 19 | #370 | #84 | library | 5 | L | — | Benchmark pure-Rust Office (`.docx`/`.xlsx`/`.pptx`) & PDF extraction to eliminate the Python `.venv` helper |
| 20 | #371 | #88 | encryption | 4 | M | — | Evaluate pure-Rust/zero-OpenSSL encryption path to drop vendored C OpenSSL + Perl build dependency |

---

## P3 — Product Depth, Encryption Rollout, Code & Launcher Polish

| Rank | Issue | Epic | Area | Impact | Effort | Depends | Summary & Value Rationale |
|---:|---|---|---|---:|---|---|---|
| 21 | #301 | #84 | library | 4 | M | — | Docs crawl follow-ups: sealed Sphinx set, fetch on miss, pre-push hook under `GIT_DIR` |
| 22 | #277 | #84 | library | 4 | M | #301 | Library: close documentation-search quality gap vs Grounded Docs & Context7 |
| 23 | #299 | #85 | code | 4 | M | — | Code graph recall: remaining work after #298 |
| 24 | #359 | #85 | code | 4 | L | #345 | Code: atomic workspace refactor planner (read-only MCP) and CLI executor |
| 25 | #360 | #86 | launcher | 4 | L | #343 | Launcher Code views: nested container zoom, symbol blast-radius toggle & static flow-column graph |
| 26 | #129 | #88 | encryption | 4 | M | #371 | Encryption: migrate live memory on a copy with rollback |
| 27 | #130 | #88 | encryption | 4 | M | #129 | Encryption: recovery in practice (QR in password manager, restore on another machine) |
| 28 | #133 | #88 | encryption | 3 | M | #129 | Encryption: macOS and Linux key stores |
| 29 | #134 | #88 | encryption | 4 | M | #130 | Encryption: independent security review |
| 30 | #125 | #87 | installer | 4 | M | — | Windows installer (NSIS or cargo-dist) |
| 31 | #123 | #86 | launcher | 3 | M | — | Linux and macOS builds of the launcher |
| 32 | #121 | #86 | launcher | 3 | S | — | Icon family: one icon per product |
| 33 | #120 | #86 | launcher | 3 | M | #343 | Independent QLoop critic over every launcher screen |
| 34 | #112 | #85 | code | 3 | M | — | Code: inputs and outputs map (files, config, env vars, tables, routes) shown on demand |
| 35 | #113 | #85 | code | 2 | M | — | Code: whole-graph overview toggle beside the useful map |
| 36 | #107 | #84 | library | 3 | S | — | Library: measure cross-version doc chunk sharing (React 18/19, Python 3.12/3.13) |
| 37 | #108 | #84 | library | 3 | M | — | Library: larger embedding model for documentation only |
| 38 | #99 | #83 | memory | 3 | S | — | Memory review milestone |
| 39 | #101 | #83 | memory | 3 | M | #356 | Memory: abstention, second attempt |
| 40 | #283 | #93 | engineering | 2 | S | — | Landing page: denser 3D particle-wave background with pointer interaction |
| 41 | #327 | #93 | engineering | 3 | S | #335 | Marketing: zero-budget launch plan for an open-source dev tool |

---

## P4 — Research Benchmarks, Curated Skills, Personal Corpus & Enterprise (Later / Gated)

| Rank | Issue | Epic | Area | Impact | Effort | Depends | Summary & Value Rationale |
|---:|---|---|---|---:|---|---|---|
| 42 | #244 | #91 | usage | 4 | L | #240,#241,#242 | Lean E: head-to-head on cost per accepted delivery |
| 43 | #222 | #94 | bench | 3 | M | — | Flow benchmark: truly bare baseline |
| 44 | #224 | #94 | bench | 3 | M | #222 | Flow benchmark: cost per accepted delivery, token vs cost saving per category |
| 45 | #225 | #94 | bench | 3 | L | #222 | Flow benchmark: complexity-ladder matrix (6 configurations x 4 levels) |
| 46 | #226 | #94 | bench | 3 | L | #225 | Flow benchmark: cross-provider reference |
| 47 | #227 | #94 | bench | 3 | L | #225 | Flow benchmark phase 2: per-layer savings attribution |
| 48 | #228 | #94 | bench | 3 | M | #225 | Flow benchmark: level where useful facts are only reachable through Footnote |
| 49 | #217 | #94 | bench | 3 | L | — | Agent benchmark: vanilla, harness, graph, and external graph tool |
| 50 | #163 | #94 | bench | 3 | M | #225 | Final ranking across engines and layers |
| 51 | #158 | #94 | bench | 3 | M | — | Model matrix (license x hardware x language) and setup playbook |
| 52 | #159 | #94 | bench | 2 | L | — | Test 5 stage A3 and llama.cpp speed grid (including ROCm) |
| 53 | #160 | #94 | bench | 2 | L | — | Storage backends: SQLite + Tantivy vs Postgres + pgvector, LanceDB, HelixDB |
| 54 | #102 | #83 | memory | 3 | L | — | Memory rematch against mem0, Cognee graph, Hindsight, Graphiti, OpenViking, EverOS |
| 55 | #278 | #229 | bench | 3 | M | — | Experiment: context-aware skill injection |
| 56 | #279 | #229 | bench | 3 | M | #278 | Skill injection: ranking eval and corpus-size cap |
| 57 | #280 | #229 | bench | 3 | M | #279 | Skill injection: hook that injects only matching skills |
| 58 | #281 | #229 | bench | 3 | M | #280 | Skill injection: ladder arm `footnote_matched_skill` |
| 59 | #282 | #229 | bench | 2 | S | #280 | Skill injection: curation rules for the marketplace |
| 60 | #231 | #229 | engineering | 3 | M | #282 | Skills catalogue: Agent Skills format and plugin marketplace index |
| 61 | #232 | #229 | bench | 3 | L | #231 | Make each curated skill useful: measure, improve or drop |
| 62 | #233 | #229 | launcher | 3 | M | #231 | Skills tab in Footnote: search, theme tabs, on/off per skill |
| 63 | #234 | #229 | bench | 2 | M | #232 | "Proven" badge: measured gain per skill |
| 64 | #135 | #89 | personal-corpus | 3 | M | — | Personal corpus step 1: sensitivity triage tool (user-gated) |
| 65 | #136 | #89 | personal-corpus | 3 | M | #135 | Personal corpus step 2: OCR benchmark (French, accents) |
| 66 | #137 | #89 | personal-corpus | 3 | M | #136,#371 | Personal corpus step 3: encrypted Library corpus |
| 67 | #138 | #89 | personal-corpus | 3 | M | #137 | Personal corpus step 4: conversion pipeline linked by SHA-256 |
| 68 | #139 | #89 | personal-corpus | 3 | M | #138 | Personal corpus step 5: folder-by-folder migration |
| 69 | #124 | #86 | launcher | 2 | L | #123 | Launcher: phones and tablets over LAN/tailnet with one-time QR pairing |
| 70 | #149 | #92 | enterprise | 3 | L | — | Enterprise: access control and data tiers |
| 71 | #150 | #92 | enterprise | 3 | L | #149 | Enterprise: compliance kit (DPO, IT security, compliance, works-council packs) |
| 72 | #151 | #92 | enterprise | 3 | M | #149 | Enterprise: cost per 1,000 users on an idle machine |
| 73 | #141 | #90 | alias | 2 | M | — | Alias (Enterprise): underline-and-explain prototype |
| 74 | #142 | #90 | alias | 2 | M | #141 | Alias (Enterprise): evaluation set v2 |
