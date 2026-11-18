# Rust receiver typing, development check

17 November 2026. This is a development check, not a held-out accuracy result or an agent benchmark.

The parser now records each Rust function's impl/trait owner and declared return type. It follows explicitly typed parameters
and local variables, constructor returns, struct fields, `Self`, and `Result`/`Option` unwrapping to resolve method calls.
Unknown steps stop resolution. Duplicate type names and ambiguous method definitions abstain. Imported or qualified receiver
types also abstain until their module identity can be established. Custom `clone` and `borrow` methods require declared
return types; their names alone do not establish the result's type. Compiler edges retain their existing precedence.

## Method and results

Windows, `toml_edit` 0.22.27 copied from the local Cargo registry into scratch storage. Only the scratch Cargo.toml was
changed: `serde` and `perf` were added to the default `parse` and `display` features so rust-analyzer could cover more code.
This is not literally every available feature. The registry and live user state were not changed.

The previous run's compiler graph was reused after checking that no benchmark source was edited during this continuation.
Its Cargo.toml SHA256 is `3582BDCF980E72BD5A4E7C83BA16481505A2177E9184079F62688E6D957B452F` and graph SHA256 is
`466D4B1BA4625ACF0DA4F35CB34DFCA61A3F960F805081FB27E0FBBEA74A29B4`.
The parser was rerun with the compiler indexer absent from PATH, with state and rule output in scratch storage.
The [rule receipt](code-graph/receiver-review-2026-11-17.tsv) contains every predicted link, including unmatched ones.

`score_rules.py <compiler.db> <rules.tsv> /tests/` reports:

| Rule | Links | Compiler agreement |
|---|---:|---:|
| receiver_type | 99 | 88/99, 0.889 |
| same_file | 217 | 0.793 |
| type_path | 49 | 0.816 |
| import | 43 | 0.767 |
| module_path | 12 | 1.000 |
| all | 420 | 0.821 |

`compare_graphs.py` deduplicates edges. On 45 compiler-covered files it reports 3,580 compiler edges, 418 parser edges,
343 shared edges, agreement 0.821 and recall 0.096. These figures compare parser calls with compiler function/type references;
they do not establish complete compiler coverage or measure agent task success. No baseline improvement is claimed.

## Review of the 11 typed links absent from compiler truth

All 11 have no compiler edge for the same source line and method name, rather than an edge to another file. Direct source
inspection found the following calls and matching definitions. This was the implementing agent's review, not an independent
blind judgment. It does not replace the raw compiler agreement score.

| Caller file and line | Method | Source evidence |
|---|---|---|
| src/array.rs:403 | push_formatted | `self` inside `Extend` for Array; inherent method on Array |
| src/array_of_tables.rs:122 | push | `self` inside `Extend` for ArrayOfTables; inherent method on that type |
| src/document.rs:200 | into_mut | ImDocument returned by `ImDocument::from_str`; its declared into_mut method |
| src/inline_table.rs:483 | extend | InlineTable constructed immediately before its Extend call |
| src/key.rs:233 | cmp | `self` in PartialOrd for Key; Ord implementation on Key |
| src/parser/state.rs:30,283,306 | finalize_table | `self` in ParseState; its declared method |
| src/parser/state.rs:289 | start_table | `self` in ParseState; its declared method |
| src/parser/state.rs:312 | start_array_table | `self` in ParseState; its declared method |
| src/table.rs:499 | extend | Table constructed immediately before its Extend call |

The initial permissive attempt had 300 typed links and agreement 0.790. Those numbers motivated source review, not tuning
on a held-out set. Qualified/imported types, aliases, generic substitution, closures, match bindings and more complex
receivers remain limitations. A fresh corpus and an independent agent benchmark are still required before claiming an
accuracy or productivity improvement.

## Held-out check, 18 November 2026

Two crates never used while writing or tuning the rule, copied from the local Cargo registry into scratch storage with
every non-unstable feature made default: `clap_builder` 4.6.7 and `zip` 6.0.0. Fresh compiler graph per crate, parser run
with rust-analyzer off PATH, `/tests/` left out. Each typed link is sorted by what the compiler graph says at that line:

| Crate | Typed links | Same as compiler | Compiler links elsewhere | Line analyzed, no edge | Line not analyzed (cfg) |
|---|---:|---:|---:|---:|---:|
| clap_builder | 419 | 369 | 1 | 7 | 42 |
| zip | 91 | 74 | 0 | 5 | 12 |

Counting every analyzed-line mismatch as wrong, precision is 0.979 on clap_builder and 0.937 on zip (443 of 456, 0.971,
together). Only one link contradicts the compiler. The raw `score_rules.py` figures (0.881 and 0.813) count lines
rust-analyzer never analyzed as errors.

The older `import` rule scored 0.151 raw on zip. Its 100 links without a compiler edge are mostly
`SimpleFileOptions::default()` linked to `impl Default for FileOptions`, which rust-analyzer files under std's
`Default::default` and not the impl, and `DateTime::from_date_and_time` inside `#[cfg(test)]` modules, which rust-analyzer's
index barely covers (9 references in all). Spot checks found the parser's targets correct, so this is a gap in the truth
graph rather than a parser error. A per-line comparison is not a substitute for an agent benchmark, which is still to do.

## Validation

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace` passed, with 137 tests passed
and one ignored. Seven code-crate tests include receiver chains, shadowing, ambiguous types, external imports and custom
method return types. This change adds no dependency.
