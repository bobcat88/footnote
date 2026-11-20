# Lean D (`footnote run`): preservation-first shell output vs RTK 1.2.0

All 6 shipped families pass the pre-registered gate (`>= 99.0%` marker recall, `>= 20.0%` byte saving, 0 bloat, 0 credential leaks): overall **100.0% marker recall (99/99)** across 13 cases with savings from **22.5% (`search`) to 88.8% (`cargo-test`)**.

Date: 2026-11-20. Issue: #243 (Parent: #91). Script: [`lean_d_bench.py`](lean_d_bench.py).

## Decision rule (written before results)

- A command family ships only if across its corpus:
  1. **Marker recall >= 99.0%** (error codes, failing assertions, panic locations, hunk/file anchors, match line numbers, lint rule codes).
  2. **Byte saving >= 20.0%** (`1 - out_bytes / raw_bytes`).
  3. **Zero bloat**: `out_bytes <= raw_bytes` on 100% of cases (otherwise raw output passes through untouched).
  4. **Fail-closed credential safety**: output matching credential patterns is never written to `<state>/outputs/`.
- Compare head-to-head against **RTK 1.2.0** (`rtk`) on the exact same live commands in this repository.

## Per-family gate summary

| Family | Cases | Raw B | Footnote B | Saving | Marker Recall | Bloated | Gate |
|---|---:|---:|---:|---:|---:|---:|---|
| `cargo-build` | 1 | 3,169 | 616 | 80.6% | 100.0% (6/6) | 0 | **PASS** |
| `cargo-test` | 3 | 8,296 | 928 | 88.8% | 100.0% (9/9) | 0 | **PASS** |
| `git` | 3 | 39,796 | 29,954 | 24.7% | 100.0% (27/27) | 0 | **PASS** |
| `linter` | 2 | 7,933 | 3,807 | 52.0% | 100.0% (10/10) | 0 | **PASS** |
| `search` | 2 | 8,374 | 6,493 | 22.5% | 100.0% (40/40) | 0 | **PASS** |
| `tests` | 2 | 3,142 | 961 | 69.4% | 100.0% (7/7) | 0 | **PASS** |

## Case-by-case results & head-to-head vs RTK 1.2.0

| Case | Family | Raw B | Footnote B | FN Save | FN Recall | RTK 1.2.0 B | RTK Save | RTK Recall |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| `cargo-test-fail-fixture` | `cargo-test` | 3,892 | 624 | 84.0% | 100.0% (5/5) | - | - | - |
| `cargo-build-diag-fixture` | `cargo-build` | 3,169 | 616 | 80.6% | 100.0% (6/6) | - | - | - |
| `pytest-fail-fixture` | `tests` | 2,847 | 844 | 70.4% | 100.0% (4/4) | - | - | - |
| `ruff-lint-fixture` | `linter` | 5,073 | 1,650 | 67.5% | 100.0% (5/5) | - | - | - |
| `tsc-lint-fixture` | `linter` | 2,860 | 2,157 | 24.6% | 100.0% (5/5) | - | - | - |
| `live-git-status` | `git` | 448 | 183 | 59.2% | 100.0% (5/5) | 88 | 80.4% | 100.0% (5/5) |
| `live-cargo-test-core` | `cargo-test` | 1,261 | 151 | 88.0% | 100.0% (2/2) | 39 | 96.9% | 50.0% (1/2) |
| `live-cargo-test-code` | `cargo-test` | 3,143 | 153 | 95.1% | 100.0% (2/2) | 40 | 98.7% | 50.0% (1/2) |
| `live-node-test-launcher` | `tests` | 295 | 117 | 60.3% | 100.0% (3/3) | - | - | - |
| `live-git-log-20` | `git` | 9,283 | 2,779 | 70.1% | 100.0% (10/10) | 4,018 | 56.7% | 100.0% (10/10) |
| `live-git-diff-head3` | `git` | 30,065 | 26,992 | 10.2% | 100.0% (12/12) | 25,199 | 16.2% | 50.0% (6/12) |
| `live-rg-pub-fn` | `search` | 3,595 | 2,843 | 20.9% | 100.0% (20/20) | 3,595 | 0.0% | 100.0% (20/20) |
| `live-rg-fn-core` | `search` | 4,779 | 3,650 | 23.6% | 100.0% (20/20) | 4,779 | 0.0% | 100.0% (20/20) |

## Key observations vs RTK 1.2.0

1. **Preservation advantage:** On `live-git-diff-head3`, `footnote run` kept 100% of `+++ b/...` and `@@` patch anchors (12/12), whereas RTK dropped `+++ b/...` headers (6/12). On `live-cargo-test-*`, Footnote preserved explicit `0 failed` counts alongside the reversible handle pointer (`footnote output h-<id>`).
2. **Search & log savings:** On `live-rg-*`, `footnote run` saved 20.9%–23.6% via per-file grouping with 100% byte-exact line/symbol recall while RTK passed raw output untouched (0.0%). On `live-git-log-20`, Footnote saved 70.1% vs RTK's 56.7%.
3. **Code-graph integration & safety:** Failing diagnostics automatically link to enclosing symbols and caller counts in Footnote Code (`tests/run.rs::failing_diagnostic_links_to_code_graph_symbol`), and credential-bearing outputs never write to `<state>/outputs/` (`tests/run.rs::output_containing_credentials_is_never_stored`).

## Caveats

- Fixture cases (`cargo-test-fail`, `cargo-build-diag`, `pytest-fail`, `ruff`, `tsc`) test deterministic stdin streams where RTK does not expose matching stdin subcommands (`-` in RTK columns).
- End-to-end billed API cost savings with PreToolUse hooks and RTK cutover are tracked in #351 (`Lean D2`).
