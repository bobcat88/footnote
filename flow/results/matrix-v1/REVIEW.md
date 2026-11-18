# Independent review: complexity-ladder matrix v1 (pre-launch)

Reviewer: independent review sub-agent, 2026-11-18. Read-only apart from running the checkers. Every mutation and dry run ran on scratch copies outside `build/matrix-v1` (session scratchpad `mutants/` and `collector-dry/`). No models were launched and nothing was committed.

## Verdict: **ready with fixes**

The plumbing is sound. Counts, order, fairness, limits, controls and the collector all check out with evidence. One blocking fix is needed before any L4 result can be trusted: a worker can find the answer key with one unscoped search, and nothing would detect it. The main open question is whether the matrix can tell setups apart. A Sonnet 5.5 medium baseline with no help passed L3 and L4 on the first try in 8 to 10 calls. Footnote context also adds almost nothing that the baseline does not already have. As frozen, L3 and L4 are likely to hit a ceiling, so the matrix would mostly measure token noise. That is should-fix, not blocking, because the matrix is labelled a development overview. Read it before spending the ~45 M tokens.

## Findings

| # | Severity | Finding | Evidence | Smallest fix |
|---|---|---|---|---|
| 1 | **blocking** | The answer key is reachable from the worker's working directory, and nothing detects a read. The recorded Codex child ran with `cwd = D:\Second Brain`, the vault root. An unscoped `rg iter_events`, which is a natural first move on a diagnosis task, finds `evaluation/level4_fixture.py:4` (the docstring states the planted root cause), `evaluation/level4_pilot.py` (`KNOWN_VALID`), `build/matrix-v1/controls/L{3,4}/known-valid/`, `build/sonnet-baseline-v1/L*/` (solved workspaces with a delivery naming every root cause), and every earlier cell's solved sibling workspace. The only guard is a sentence in the prompt. The collector does not scan the session commands. | Session `archived_sessions/rollout-2026-11-17T21-46-42-01a1eb43-….jsonl`, first line `cwd: D:\Second Brain`. `prepare_matrix.py:251-255` holds the prompt-only isolation. `collect_matrix.py:425-504` has no command audit. | In `collect_matrix.collect`, scan the session's `function_call` arguments for paths outside {workspace, skill, handoff} and for `evaluation`, `controls`, `known-valid`, `matrix-v1`, `sonnet-baseline` and `luna-medium-`. Record `outside_reads` in `result.json` and flag or exclude contaminated cells. Also add "Run every command from the workspace (cd there first)" to `worker_task`. The archived sessions allow the scan after the fact, but it must exist before any L3 or L4 number is reported. |
| 2 | should-fix | **The setups are unlikely to differ on L3 and L4 (ceiling).** A Sonnet 5.5 medium baseline with no help passed L3 15/15 in 8 calls and 139 s, and L4 17/17 in 10 calls and 103 s, against limits of 16 and 20 responses. Its L4 delivery names all three planted causes correctly. | `build/sonnet-baseline-v1/results.json`, `build/sonnet-baseline-v1/L4/workspace/delivery.txt` | Before launching L3 and L4 at scale, harden them (see "Making L3/L4 harder" below). Pilot the hardened version once on Luna-medium baseline and Sonnet baseline, and continue only if at least one of them fails a check. |
| 3 | should-fix | **The Footnote treatment duplicates inputs every arm already has.** In every level, `footnote-context.json` holds DECISION.txt verbatim (memory), a contract.txt excerpt that is CONTRACT+DECISION (library) and a call graph of one symbol. TASK.txt already points every arm at CONTRACT.txt and DECISION.txt, so the context offers nothing to retrieve that the baseline cannot read directly. Any difference between setups will therefore come from prompt length and order, not from Footnote. | `sol-high/L4/flow/workspace/footnote-context.json` (memory = DECISION text, `code.symbols` = `recover` only). `level4_pilot.py:139-145`, `level3_pilot.py` analogue. | Put the decisive knowledge where retrieval matters (see the hardening list), or record in MODEL-STUDY that L1–L4 v1 test delivery overhead, not knowledge access. |
| 4 | should-fix | **The L4 contract points at the culprit.** CONTRACT.txt states "db.iter_events … yields every event with id > after exactly once, in id order, for any positive batch size". Together with "busy weeks only", this effectively names the bug. | `level4_fixture.py` CONTRACT. `grep` finds `iter_events` in all 35 L4 CONTRACT.txt copies (30 cells and 5 controls). | Drop the batch-size clause from the contract and keep test_01 hidden. The symptom alone stays unambiguous, because the contract still says report/catch_up cover "every stored event". |
| 5 | should-fix | **L3 does not check the explicit-transaction requirement.** I validated rows first and then upgraded in autocommit, without BEGIN, COMMIT or ROLLBACK. That version passes 15/15, although DECISION requires "one explicit transaction". | Scratch mutant `L3-nontransactional-upgrade`: `hidden_exit=0`. | In test_02 or test_03, attach `conn.set_trace_callback` to a connection, call `store.migrate(conn)` on a v0 file, and assert that a `BEGIN` comes before the `ALTER` and a `COMMIT` comes after the `user_version` change. This needs no fault injection and leaves no room for interpretation. |
| 6 | should-fix | **L4 does not test that ship is atomic or that recovery identifies lines correctly.** (a) A ship that validates input, commits the order row and then commits each line passes 17/17. The contract says "Recording an order is all-or-nothing". (b) A recovery that resumes by *count* of present lines instead of by `(order_id, line_no)` passes 17/17, because the crash always leaves a contiguous prefix. | Scratch mutants `L4-ship-true-per-line`: OK. `L4-recover-by-count`: `hidden_exit=0`. | (a) Use the same trace-callback check on `ship` (one transaction per call), or patch `sqlite3`'s commit hook so it fails after the first line and assert nothing was written. (b) Add a pending order whose stored line is line 1 but not line 0, and assert that only line 0 is written. |
| 7 | should-fix | **The worker prompt carries model and setup names through the path, and TASK.txt mentions the evaluation.** Workspace paths `build/matrix-v1/<sol-high>/<L>/<footnote_only>/workspace` give the model family, the effort and the setup in every prompt. TASK.txt in every arm says "other arms, hidden checks". It is identical across arms, so the comparison stays fair, but it contradicts the RUN-MATRIX claim that the prompt "names no model, no other setup and no hidden checks". | `prepare_matrix.py:246,284`. TASK.txt grep: `130 hidden`, `130 other arms`. Context JSON includes the path (`luna`/`terra`/`sol`/`footnote_only` hits). | Use opaque cell directories (`build/matrix-v1/cells/<seq>/workspace`) and re-run `prepare_matrix` (delete `build/matrix-v1` first). Or keep the paths and correct the RUN-MATRIX sentence. Keeping them stays consistent with the 10 recorded cells, which used named paths. |
| 8 | should-fix | **The aggregate silently drops mismatched cells' resources.** `aggregate()` leaves cells without `configuration_match` out of `total_tokens_valid_cells` and `accepted`. A cell whose receipt failed to parse (`receipt_error`) therefore vanishes from tokens per accepted outcome. MODEL-STUDY says failed and infrastructure-affected runs must be kept, and their usage counted. | `collect_matrix.py:517-523` | Also report `total_tokens_all_cells`, and list `receipt_error` cells separately from wrong-model cells. |
| 9 | note | `handoff.json` (Flow only) contains `engine: t3:codex:<model>:<effort>`. Only the Flow worker is told to read it. A Codex model already knows its own identity, and the effort label has no plausible behavioural effect, so this does not matter in practice. It is a small asymmetry worth one line in the limitations. | `level*_pilot.py`, `executor.engine=...`. grep: `gpt-6-luna` ×10, `gpt-6.1-terra` ×8, `gpt-6.2-sol` ×8 in handoff.json. | Disclose it, or set `engine='t3:codex'`. |
| 10 | note | The skill card for L3 and L4 is `sqlite-sql-performance`, which is about performance, while the tasks are about migration and recovery correctness. The "skills" arm effect is likely to be nil by construction. | `level3_pilot.py` and `level4_pilot.py`: `SKILL`. | Disclose it, or choose a card that covers transactions and migrations if the pack has one. |
| 11 | note | `timestamps_match` is computed but takes no part in `accepted`. Session binding relies on the session ID plus the workspace appearing in the prompt, which is adequate. | `collect_matrix.py:448-450,485` | None needed. Optionally warn when it is false. |
| 12 | note | The preparation run left `__pycache__` in `controls/L{2,3,4}`, dated 18:53–18:54 (the L2 checker and the cli subprocesses run without `-B`). Harmless, because controls are never cells. | `find build/matrix-v1 -name __pycache__`: controls only. | None needed. |

## Verification evidence

**1. Counts and order.** The scratch script `verify.py` loads the manifest and the ladder:
- `counts {'cells': 120, 'existing': 10, 'prepared': 110}`, 110 entries, 120 ladder cells, union equal, overlap empty.
- Existing cells: luna-medium L1 and L2 × 5 setups.
- Configurations: luna-medium/high `gpt-6-luna`, terra-medium/high `gpt-6.1-terra`, sol-medium/high `gpt-6.2-sol`. Per (model, effort): 20 cells each, and 10 for luna-medium.
- `order ok True` (configuration → L1..L4 → seeded setup order), `seq ok True` (1..110).
- Orders: L1 `footnote, footnote_only, baseline, flow, skills`; L2 `flow, footnote_only, skills, footnote, baseline`; L3 `flow, footnote, footnote_only, baseline, skills`; L4 `footnote_only, flow, baseline, footnote, skills`. These match RUN-MATRIX.
- All 110 workspace directories exist. The ladder status is `cells_prepared_aggregate_budget_pending`, with 110 `prepared_not_run` and 10 `existing_exploratory_observation`.

**2. Fairness.**
- All 110 workspaces have top-level file hashes equal to `levels.<L>.initial_hashes` (0 mismatches, no subdirectories). Within each level, the hashes are the same across all configurations and setups.
- Each setup gets exactly its intended inputs (0 violations):
  - a skill card for skills, footnote and flow
  - `footnote-context.json` for footnote_only, footnote and flow
  - `handoff.json` for flow only
- Once the cell path is normalised, the Footnote context is byte-identical across all three context setups and all configurations of a level (1 distinct value per level).
- Caveat: every arm folder, baseline included, contains `bin/`, `docs/`, `library.db` and `state/`. Workers are told not to read them.

**3. Controls.** Re-run in place with `PYTHONDONTWRITEBYTECODE=1`:

| Level | broken | known-valid | known-valid existing tests |
|---|---|---|---|
| L1 | exit 1 (6 tests, failures=6, errors=2) | exit 0 (`build/integrated-loop-v1/workspace`) | n/a |
| L2 | exit 1 (14/14 fail) | exit 0 | n/a |
| L3 | exit 1 | exit 0 (15 OK) | OK |
| L4 | exit 1 | exit 0 (17 OK) | OK |

Wrong solutions, each a scratch copy of known-valid with one change. L4 mutants recovered their own copy of the seed `warehouse.db` with their own code first.

| Mutant | Caught? | By |
|---|---|---|
| L3 non-transactional upgrade (validate first, autocommit) | **no** | (see finding 5) |
| L3 upgrade without legacy row validation | yes | test_03 |
| L3 drop legacy caller shape (currency positional) | yes | test_05 |
| L3 balance sums all currencies | yes | test_06, test_14 |
| L3 export opens target before refusing | yes | test_09 |
| L3 restore ignores conflicting ids | yes | test_12 |
| L3 import without a transaction | yes | test_12 |
| L4 fix in report only (iter_events still buggy) | yes | test_01, 04, 06, 12, 15, 16 |
| L4 huge default batch instead of fixing cursor | yes | test_01 |
| L4 rebuild stock from zero | yes | test_04, 06, 12, 15, 16 |
| L4 recovery rewrites every line | yes | test_06, 09, 12, 15, 16 |
| L4 recovery not atomic across orders | yes | test_08 |
| L4 code fixed, warehouse.db left unrecovered | yes | test_16 |
| L4 ship commits per line (validated first) | **no** | (see finding 6a) |
| L4 recovery resumes by line count | **no** | (see finding 6b) |

**4. Leakage.**
- Grep over all TASK, CONTRACT, DECISION, footnote-context and handoff files: no hidden-check content, no `test_NN` names, no known-valid code, and no "skips the first" or `+ 1` root cause.
- `iter_events` appears only in L4 CONTRACT.txt, as an interface clause (finding 4).
- "hidden checks" and "other arms" appear in every TASK.txt (finding 7). Model and setup names reach the worker only through the path, plus the Flow handoff engine string (findings 7 and 9).
- The worker prompt itself contains no model string, setup name, "hidden" or "known" (0 hits after removing the path).
- The real leakage risk is the reachable filesystem (finding 1), not these files.

**5. Prompts and manifest.**
- For every entry: provider `codex`, and model and `reasoningEffort` equal to its configuration.
- Limits in both the fields and the prompt text: L1 8/5, L2 12/8, L3 16/10, L4 20/12.
- The workspace exists and its path appears in the prompt.
- `collector_command == "rtk proxy python -m evaluation.collect_matrix <cell_id> [CODEX_SESSION_ID]"`, and cell IDs follow `<config>-<L>-<stack>`.
- 0 violations across all 110 entries.

**6. Collector.** `collector_dry.py` ran `collect()` against the recorded L2 `skills` cell. The workspace was read-only, and the cell directory, manifest and aggregate were redirected to scratch (`cm.ROOT` patched).
- With the session ID `01a1eb43-…` from `session-identities.json`, the result reproduces the recorded one exactly: `accepted True`, 11 calls, input 404,684, cached 364,416, output 2,888, total 407,572, 66.823 s. `timestamps_match True`, and independent and existing checks both exit 0.
- Model set to `gpt-6.2-sol`: `configuration_match False`, `accepted False`.
- Effort set to `high`: `configuration_match False`, `accepted False`.
- `import numpy` added to `billing.py` in a scratch copy: `foreign_imports → ['numpy']`, so acceptance would fail.
- The checker-hash guard resolves correctly for L1 (`integration.check_intervals`) and for L2–L4.
- `git status` showed no change under `build/luna-medium-dependency-v1`. Its existing `__pycache__` dates from 2026-11-17.

**7. Difficulty.** See findings 2–6. With these suites, at most the three un-caught mutants could separate setups, and the Sonnet baseline found all three planted L4 causes from the symptom list and contract alone.

## Making L3/L4 harder without making them ambiguous

Each item adds a requirement that is stated somewhere authoritative and checked by a hidden test with one correct answer.

1. **Close the three gaps above** (transaction trace check in L3; ship atomicity and non-prefix pending lines in L4). This catches plausible shortcuts, not new requirements.
2. **Remove the culprit pointer in L4** (finding 4). Keep the symptoms only.
3. **Make knowledge retrieval matter (L3).** Add `docs/` with about 10 ADR or changelog files to the workspace, and do not name the relevant one in TASK.
   - One ADR states a legacy quirk the upgrade must handle. Example: v0 files written by release 0.3 store `account` with trailing whitespace that must be trimmed, documented with an example.
   - One older ADR says the opposite and is explicitly marked "Superseded by ADR-007".
   - The hidden test builds such a v0 file.

   The answer is unambiguous once found, and the task is no longer solvable by reading three named files. This is what Footnote Library and Memory are claimed to help with.
4. **Add a second, independent L4 cause that the symptoms do not reveal.** Example: `catch_up` commits stock updates and `applied_through` in separate statements without a transaction, and the hidden test injects a failure mid-catch-up. The contract already says "each applied exactly once", which settles the expected outcome.
5. **Add consumers in L4.** For example, an `audit.py` export and a `metrics.py` that also use `iter_events`, or that duplicate the paging logic with the same bug. Hidden tests would cover them, so a fix confined to `db.py` plus `report.py` stays incomplete. Each new consumer is covered by the existing "every stored event" clause.
6. **Use a two-session L4.** Session 1 has a small budget (for example 6 responses) and may only diagnose and write notes. Session 2 starts fresh with a new prompt and must fix and recover. This is the "fresh-session follow-up" stratum in MODEL-STUDY, and the only place Memory and Flow continuity can show an effect. Acceptance stays the same hidden suite.
7. **Make the limits matter.** The baselines used 8–10 calls against 16 and 20. Either report calls and time as the primary outcome for L3 and L4, with acceptance as a gate, or lower the requested limits to L2's 12. The limits are not enforced, so the first option is the honest one.

Pilot items 1–5 against the Luna-medium and Sonnet baselines before the full run. Continue only if at least one baseline fails a check, and freeze a new `complexity-ladder-v2` rather than editing v1, since v1 has recorded L1 and L2 observations.

## Fixes applied (2026-11-18, implementation sub-agent)

No model was launched and nothing was committed. `build/matrix-v1` was deleted and re-prepared; this review was restored afterwards. Wrong solutions ran on scratch copies outside `build/`.

| # | Fix |
|---|---|
| 1 (blocking) | (a) The `level4_fixture.py` docstring no longer lists the planted causes, and the ladder's L4 description no longer says "pagination". (b) The worker prompt now says: "Run every command from the workspace (set it as the working directory). Never read, list or search outside it, except the paths named here; no searches from a parent folder." (c) `collect_matrix` scans every tool call in the Codex session: exec_command cmd and workdir, JSON `function_call` arguments and apply_patch file headers. It records `outside_accesses` in `result.json`, excluding the Python install, rtk and the assigned skill folder. It sets `contaminated` when an access reaches the project outside the cell, or a parent folder from which the project is reachable. A contaminated cell is never `accepted`. `--self-test` covers clean, harmless-outside and five contaminated patterns. The recorded L2 skills session scans clean. |
| 7 | Cells live in `build/matrix-v1/cells/<sha256(cell_id)[:10]>/`. The mapping is kept only in the manifest and in `<config>/<L>/study.json`. L3/L4 TASK.txt now read "No network, installs, commits, child agents or unrelated files. An independent acceptance step checks the delivery." The wording is the same in every arm. L1/L2 keep their frozen text, which is recorded as a known limitation in RUN-MATRIX and in `known_limitations` in the manifest. |
| 5 | L3 test_16 wraps `sqlite3.connect` with `set_trace_callback`. Every upgrade write (ALTER, INSERT, UPDATE, DELETE, DROP, CREATE without IF NOT EXISTS, `PRAGMA user_version =`) must run inside a transaction, no COMMIT/END/ROLLBACK may come between the first and last write, and a COMMIT must follow. |
| 6, 4 | L4 test_19 adds a trigger that fails the line-1 insert. `ship` must raise and leave no order and no events. L4 test_20 adds a pending order with lines 0 and 2 stored. Recovery must add only line 1. The contract sentence naming `db.iter_events` became "Every reader of the event log sees every event newer than its starting point exactly once, in id order, whatever page size it reads with." |
| 2 (harder) | L3: `CHANGELOG.txt` (release notes, newest first, with a superseded 0.3.2 note). CONTRACT says the release notes are part of the contract but does not name the file. When a v0/v1 file is upgraded, trailing U+0020 spaces are removed from accounts, and an account left empty aborts the upgrade. v2 files are never rewritten (test_17). L4: `projection.catch_up` in the source now commits every 50 events. CONTRACT requires exactly-once "also when an earlier catch-up was interrupted part way". test_18 injects a failure at the 120th stock write, then re-runs the catch-up and expects the logged stock. Known-valid solutions were updated (rtrim in the transaction; catch_up in one transaction). |
| 8 | The `results.json` summary adds `total_tokens_all_cells`, `failed_attempt_tokens`, `contaminated_cells`, `configuration_mismatch_cells` and `receipt_error_cells`. `unknown_usage_cells` now covers every cell. |
| 3 | Not changed (out of scope). The Footnote context still repeats CONTRACT and DECISION. The new L3 release notes are not in it. |

**Checks:** L3 has 17 (was 15), L4 has 20 (was 17). Fixture v2 is recorded in `complexity-ladder-v1.json` (`fixture_version`, `fixture_changes`).

**Controls after re-prepare:** L1/L2/L3/L4 broken exit 1 and known-valid exit 0. L3/L4 known-valid existing tests exit 0. 110 cells. Initial hashes are identical within each level, with 0 mismatches. Every prompt and workspace path is neutral (0 violations). L3/L4 TASK/CONTRACT/DECISION files have 0 hits for "hidden", "other arms" or `iter_events`. The recorded pilot folders and `sonnet-baseline-v1` were not touched: no file in them is newer than 19:00, and this pass started at 19:12. `build/` is git-ignored, so git status cannot show this.

**Wrong solutions (scratch copies of known-valid, one change each):**

| Mutant | Before | Now |
|---|---|---|
| L3 non-transactional upgrade (validate first, autocommit) | passed | caught (test_16, test_17) |
| L3 upgrade without the release-notes trim | n/a | caught (test_17) |
| L3 trim on every open, v2 included | n/a | caught (test_17) |
| L3 trim of all whitespace, not only spaces | n/a | caught (test_17) |
| L4 ship commits per line (validated first) | passed | caught (test_19) |
| L4 recovery resumes by line count | passed | caught (test_20) |
| L4 catch-up commits every 50 events (the new source defect) | n/a | caught (test_18) |
| L4 huge default batch instead of a cursor fix | caught | still caught (test_01) |

A migrate that uses `with conn:` in legacy isolation mode passes test_16. It is still correct, because its trim UPDATE opens an implicit transaction before the ALTER, so the upgrade is atomic.

**Still open:** whether v2 is hard enough to separate setups. Neither the Luna-medium baseline nor the Sonnet baseline has been piloted on v2 (finding 2 recommends that before the full run). The `handoff.json` engine string is still present (note 9). The skill card is unchanged (note 10). The ladder keeps the name `complexity-ladder-v1` with per-level `fixture_version` rather than a new v2 file, because the user asked for a version bump in place.
