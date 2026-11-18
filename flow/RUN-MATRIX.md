---
type: runbook
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
date_maj: 2026-11-18
---
# Run the complexity-ladder matrix

Up: [[AI-Agents/projects/footnote-framework/MODEL-STUDY|Model study]]

The 110 unrun cells are prepared in `build/matrix-v1/cells/<opaque-id>/`, and `build/matrix-v1/launch-manifest.json` lists them in run order. The folder name is the first 10 hex characters of `sha256(cell_id)`, so a workspace path names no configuration, model, level or setup. The mapping lives only in the manifest and in the per-configuration `build/matrix-v1/<config>/<L>/study.json` files, and the worker prompt never points at either. L3 and L4 use fixture v2 (review fixes, see `build/matrix-v1/REVIEW.md`, "Fixes applied"). The 10 Luna-medium L1/L2 cells already ran: the manifest lists them under `existing_cells` and they are never relaunched. Launch once the Codex quota returns (22 November 2026, 14:43). Freeze the aggregate usage envelope first (see MODEL-STUDY).

## Launch, one cell at a time

For each `cells[i]` in the manifest, in `sequence` order:

1. Call `delegate_task` with:
   - `task`: `entry.task`, verbatim
   - `title`: `entry.title`
   - `role`: `"implementation"`
   - `target`: `{providerInstanceId: "codex", model: entry.model, options: {reasoningEffort: entry.options.reasoningEffort}}`
   - `mode`: `"async"`
   - `isolation`: `"shared"`

   Call `orchestrator_capabilities` once before the first launch and confirm that all three models and the `reasoningEffort` option exist. If a model is missing, stop. Do not substitute another model.
2. When the completion wakes you, call `t3_thread_read` on the child thread with `runLimit: 1`. Write `{threadId, link, model, recentRuns}` to `entry.t3_run_file` (`<cell_dir>/t3-run.json`). This is the same shape as `build/luna-medium-dependency-v1/t3-runs.json`.
3. Run `entry.collector_command` from this project root: `rtk proxy python -m evaluation.collect_matrix <cell_id> [CODEX_SESSION_ID]`. If you leave out the session ID, the collector finds the one Codex session, in `~/.codex/sessions` or `archived_sessions`, whose prompt names the cell workspace. It runs the level checker and the existing tests, checks the model and the reasoning effort, reconciles usage call by call, then writes `<cell_dir>/result.json` and rebuilds `build/matrix-v1/results.json`. It also scans every tool command and path in the Codex session. Accesses outside the cell folder are listed in `outside_accesses`; the Python install, rtk and the assigned skill folder are excluded. If any access reaches the project outside the cell, or a parent folder from which the project is reachable (for example a search from the vault root), the cell is `contaminated` and is not accepted. `rtk proxy python -m evaluation.collect_matrix --self-test` checks the scan.
4. Move on to the next entry. Never relaunch, repair or replace a failed cell. A failed, incomplete or mismatched run stays in the results. If a run never starts, record it by hand in `<cell_dir>/infrastructure-note.txt` and include it in the report.

Running cells one after another is the default. It matches the recorded pilots and avoids quota and cache contention that could favour one setup. If you need parallel runs, use at most 2 or 3 slots. Fill the slots strictly in `sequence` order, and never run two setups from the same configuration and level at the same time. Record that the runs overlapped. `--aggregate` rebuilds `results.json` without collecting anything. The summary keeps every attempt's usage: `total_tokens_all_cells` and `failed_attempt_tokens` include mismatched, contaminated and failed cells. Cells whose receipt failed to parse have no token count and are listed under `receipt_error_cells`, separately from `configuration_mismatch_cells`.

Requested limits per level are soft and not enforced by the provider: L1 8 responses and 5 min, L2 12 and 8, L3 16 and 10, L4 20 and 12. Within each level, setups run in the same seeded order for every configuration. The order is recorded in `levels.<L>.order`:

- L1 (seed 6117): footnote, footnote_only, baseline, flow, skills
- L2 (seed 6118): flow, footnote_only, skills, footnote, baseline
- L3 (seed 6119): flow, footnote, footnote_only, baseline, skills
- L4 (seed 6120): footnote_only, flow, baseline, footnote, skills

L2 keeps its recorded order. L1 now has five setups: the five-setup list is shuffled with the original seed, so this order differs from the recorded four-setup order, and footnote_only was added afterwards. Here `footnote` means Footnote plus skills (`footnote_and_skills` in the ladder).

## Token estimate (an estimate, not a budget)

This estimate comes only from the recorded Luna-medium usage (child workers; parent work is excluded):

| Level | Basis | Tokens per cell |
|---|---|---:|
| L1 | mean of the 5 recorded L1 cells | ≈ 274,000 |
| L2 | mean of the 5 recorded L2 cells | ≈ 332,000 |
| L3 | 36,875 tokens per call (L2) × 75% of the 16-response limit | ≈ 443,000 |
| L4 | same rule, 20-response limit | ≈ 553,000 |

- One configuration (20 cells) comes to about 8.0 M tokens.
- The 110 unrun cells come to about **45 M tokens**. Of that, roughly 13%, about 5.8 M, is uncached input, and output is under 1%.
- If every cell used its full requested response limit, the total would be about 58 M.

High reasoning effort and the stronger models could move these numbers either way, through more reasoning output or through fewer calls. L3 and L4 have no observed data. Treat every figure as a planning estimate. None is a price.

## Preparation record

Preparation (`rtk proxy python -m evaluation.prepare_matrix`, already run once, launched nothing) did the following:

- Ran each level's broken and known-valid controls once, in `build/matrix-v1/controls/<L>/`. The exit codes are in `levels.<L>.controls`.
- Checked that the five setups of each configuration and level start with the same files, and that these files match across configurations.
- Checked that the L1 and L2 cells use the same fixture, knowledge files, skill card and Footnote context as the recorded pilots. The context check ignores the workspace path and `context_bytes`, the context's byte length, which changes only with the length of that path.
- Built the worker prompt from the recorded L2 Codex prompts (`build/luna-medium-dependency-v1/session-identities.json`). It names no model, no other setup and no hidden checks. It tells the worker to run every command from the workspace and never to read, list or search outside it. `prepare_matrix` asserts that every prompt and workspace path is neutral and that L3/L4 TASK, CONTRACT and DECISION files never mention hidden checks or other arms.
- Re-prepared from clean on 2026-11-18 after the independent review. Results: 110 cells, initial hashes identical within each level, all controls as expected (broken exit 1, known-valid exit 0, L3/L4 known-valid existing tests exit 0), and L1/L2 reproduce the recorded pilots.

## Known limitations

- L1/L2 TASK.txt keep their recorded frozen text, which says "other arms" and "hidden checks". Changing it would break reproduction of the recorded L1/L2 cells. The text is the same in every arm.
- The Flow `handoff.json` carries `engine: t3:codex:<model>:<effort>` (Flow arm only).
- The Footnote context still repeats CONTRACT and DECISION, which every arm already reads (REVIEW item 3, out of scope here). The L3 release notes (`CHANGELOG.txt`) are in the workspace for every arm but not in the Footnote context.
- The access scan reads the paths named in tool calls. It cannot see a path that a script builds at run time. A code-mode call that chains several commands with different working directories is checked against a single working directory.
