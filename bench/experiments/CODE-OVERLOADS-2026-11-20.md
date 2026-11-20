# Call-graph recall: overloaded definitions are one symbol (20 November 2026)

Goal (#299): raise recall without lowering the default tier's precision.

Finding: in Python (`@overload`, property setters), TypeScript and JavaScript (overload signatures), a function with several
definitions in one file under one owner is one symbol, but the default tier abstained whenever a name had more than one definition
(`[one]` only). click's `get_current_context`, `convert_type`, `group`, `edit` and `prompt`, and many hono and nest functions, were
never linked at their call sites. Change (`pick` in `crates/code/src/ts.rs`): for Python, TypeScript, TSX and JavaScript, several
definitions in the same file with the same owner and kind are linked to the first one, in the `import`, `receiver_type` and `type_path`
rules. Java and Rust are unchanged (their overloads are different functions, still abstained). No new flag, no new tier name: the
links keep their rule names.

Method: same fixtures, truth and scorer as `CODE-TIERS-2026-11-19.md` (`<local-codescore-dir>`, scip-python and
scip-typescript graphs built by the same binary, parser-only run with `FOOTNOTE_CODE_RULES` and `FOOTNOTE_CODE_GUESSES=1`,
`score_tiers.py`). The "before" row is a fresh run of the unchanged build (it matches the 19 November table, except two small
drifts: TypeScript default 7745 links against 7743, nest 3575 against 3573, from the compiler graph being rebuilt).

Default tier, before then after:

| Scope | Links | Same | No edge | Analyzed precision | Recall |
|---|---:|---:|---:|---:|---:|
| Python | 9214 -> 9284 | 7047 -> 7070 | 1098 -> 1110 | 0.865 -> 0.864 | 0.455 -> 0.458 |
| TypeScript | 7745 -> 8251 | 6571 -> 7073 | 468 -> 472 | 0.933 -> 0.937 | 0.579 -> 0.609 |
| click | 2331 -> 2391 | 757 -> 770 | 508 -> 520 | 0.597 -> 0.595 | 0.351 -> 0.359 |
| httpx | 1486 -> 1486 | 898 -> 898 | 588 -> 588 | 0.604 -> 0.604 | 0.224 -> 0.224 |
| rich | 5397 -> 5407 | 5392 -> 5402 | 2 -> 2 | 0.999 -> 0.999 | 0.595 -> 0.597 |
| hono | 3927 -> 4284 | 3090 -> 3447 | 328 -> 328 | 0.904 -> 0.913 | 0.256 -> 0.278 |
| nest | 3575 -> 3724 | 3281 -> 3426 | 128 -> 132 | 0.962 -> 0.963 | 0.742 -> 0.778 |
| zustand | 243 -> 243 | 200 -> 200 | 12 -> 12 | 0.943 -> 0.943 | 0.578 -> 0.578 |

Default plus all guesses (`FOOTNOTE_CODE_GUESSES=1`): Python recall 0.558 -> 0.561 (precision 0.872 -> 0.871), TypeScript 0.626 -> 0.657
(0.929 -> 0.927). Analyzed precision is the share of judged links that are the compiler's own edge; the two Python projects that fall
by 0.001 to 0.002 each gain 13 correct links and 12 uncorroborated ones (the compiler has no edge on the line), i.e. precision is flat
within noise, not improved. Java and Rust were not rescored: the change does not touch them.

What this does not show: only the file of the target is scored (same unit as before), so linking an overload to its first signature
instead of its implementation is not distinguished. Rust projects, private-ts-1 and Spring are not rescored (not part of these
fixtures). Unknown-type receivers (the largest bucket) are untouched.

Test: `overloaded_definitions_in_one_file_are_one_symbol` (tests/code.rs). Also in this change: the `tests/code.rs` helper clears
`GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE` so `rust_in_a_subfolder_gets_the_compiler_index` passes under a git hook.
