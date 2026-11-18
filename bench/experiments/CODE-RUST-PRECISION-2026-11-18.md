# Rust call graph precision on 15 projects

18 November 2026, issue #214. Parser (tree-sitter base layer) links scored against a fresh rust-analyzer graph for each project.

## Corpora

| Group | Projects | How chosen |
|---|---|---|
| Tuning | toml_edit 0.22.27 | used while writing receiver typing (#201); reported apart |
| Held out, online | clap_builder 4.6.7, zip 6.0.0 | picked by hand on 17 Nov, never used for tuning |
| Held out, Johan's | sherlock, RingMaster, GovernanceOS (`src-tauri`), simple-code-gui (`src-tauri`), AlphaBar (`crates/alphabar`), Bonsai (`experiments/queen-drone-rust-v2`) | every Rust project in `D:\Projets` with more than 5 Rust files, minus Footnote, its copies, Bonsai2 (copy of Bonsai) and HyprlandConfig (vendored sherlock) |
| Held out, random | tokio-util 0.7.20, gix-index 0.58.0, lindera-dictionary 3.0.7, core-foundation-sys 0.8.7, brotli 9.1.3, zerovec 0.11.9 | `random.seed(20261118)`, 6 drawn from the 195 crates in the local Cargo registry (latest version each) with at least 30 `.rs` files under `src`, excluding toml_edit, clap_builder, zip and platform binding families (windows, winapi, libsqlite, openssl, ring, aws-lc) |

The random draw comes from the local registry, not from GitHub: indexing runs a crate's build scripts and macros, and those
crates had already been built on this machine, so no new third-party code ran. The draw is therefore biased toward popular
libraries that Johan's projects depend on. core-foundation-sys is macOS-only and produced no scorable link on Windows; it stays
in the draw.

## Method

Each project copied to scratch storage without `target/`, `.git` or `node_modules`; every non-unstable feature made default
in the copy's `Cargo.toml`; rust-analyzer graph built fresh; parser run with rust-analyzer off PATH and
`FOOTNOTE_CODE_RULES` writing every link with its rule; `/tests/` left out. Each parser link is classed by what the compiler
graph says at that line (`score_rules.py`, #215): **same**; **compiler links elsewhere** (a real disagreement); **analyzed,
no edge** (the compiler analyzed the line but holds no link for that name); **not analyzed** (code behind a feature or
platform rust-analyzer did not enable). Analyzed precision = same / (same + elsewhere + no edge). Conflict rate = elsewhere /
(same + elsewhere).

## Results (after the two fixes below)
| Corpus | Rule | Links | Same | Compiler links elsewhere | Analyzed, no edge | Not analyzed | Analyzed precision | Conflict rate |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| held-out | same_file | 3615 | 3301 | 25 | 56 | 233 | 0.976 | 0.008 |
| held-out | receiver_type | 2011 | 1336 | 1 | 135 | 539 | 0.908 | 0.001 |
| held-out | import | 1037 | 806 | 2 | 225 | 4 | 0.780 | 0.002 |
| held-out | type_path | 795 | 613 | 0 | 181 | 1 | 0.772 | 0.000 |
| held-out | module_path | 249 | 249 | 0 | 0 | 0 | 1.000 | 0.000 |
| held-out | all | 7707 | 6305 | 28 | 597 | 777 | 0.910 | 0.004 |
| tuning | same_file | 204 | 162 | 1 | 18 | 23 | 0.895 | 0.006 |
| tuning | receiver_type | 99 | 88 | 0 | 0 | 11 | 1.000 | 0.000 |
| tuning | type_path | 53 | 50 | 0 | 3 | 0 | 0.943 | 0.000 |
| tuning | import | 43 | 33 | 2 | 8 | 0 | 0.767 | 0.057 |
| tuning | module_path | 12 | 12 | 0 | 0 | 0 | 1.000 | 0.000 |
| tuning | all | 411 | 345 | 3 | 29 | 34 | 0.915 | 0.009 |

Recall against all compiler function and type references stays low (0.04 to 0.20 per project): the parser only links calls
and refuses when unsure, while the compiler graph also holds type uses. The hybrid keeps the compiler graph as the main source
whenever it can run (#218).

## Fixes found by this run

1. **`type_path` used the wrong `impl` (brotli).** `ByteBuf::default()`, a derived `Default`, was linked to the neighbouring
   `impl Default for Phase` because the rule took "the `default` in the file that defines `ByteBuf`". For Rust the rule now
   uses the method table: only a method of that very type, and no link when the project type has no such method.
2. **Bare calls linked to methods (clap_builder).** `write_help(..)` was linked to `Command::write_help` in the same file. A
   bare Rust call never reaches a method, so `same_file` now skips methods for bare calls. Conflicts fell from 37 to 25 with
   the same 3,301 correct links.

## The "analyzed, no edge" links

`import` (0.780) and `type_path` (0.772) sit below 0.95 on analyzed lines, yet almost never conflict with the compiler (2 and 0
links). A seeded sample of about 40 of their no-edge links across RingMaster, Bonsai, lindera-dictionary, brotli, gix-index and
zip was correct on reading the source every time. Causes:
- `Type::default()` and `Type::from(..)` on a hand-written trait impl: rust-analyzer files the call under std's trait, not the
  project's impl, which is the code that runs.
- Calls in `#[cfg(test)]` modules, which rust-analyzer's index barely covers.
- Methods on types with a lifetime parameter (`Sink<'a>::emit`, `Iter<'a>::new`): rust-analyzer's own SCIP output names
  `Sink::emit` 7 times for about 40 calls.

So both rules stay on. This review was done by the implementing agent, not blind; an independent review is still open.

The 25 remaining conflicts include rust-analyzer's own mistakes: zip's bench files each define `generate_random_archive`, and
rust-analyzer links all of them to the one in `benches/read_metadata.rs`.

## Verdict

The strategy holds on Rust: link only through a proven path (same file, import, module path, the type's own method table, a
typed receiver) and refuse otherwise. Across 7,707 held-out links, 28 conflict with the compiler (0.4%), and the receiver
typing added in #201 conflicts once in 1,337 comparable links. It is sound enough to copy to TypeScript and Python (#216),
where the same scoring applies against scip-typescript and scip-python.
