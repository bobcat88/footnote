---
type: protocol
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Flow Full Evaluation
  - Flow Three Arm Benchmark
date_maj: 2026-11-17
---
# Flow three-arm evaluation

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Flow]] · Related: [[AI-Agents/projects/footnote-framework/EVALUATION|Earlier review pilot]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## Status and scope

Current product-evaluation design: [[AI-Agents/projects/footnote-framework/FOOTNOTE-FLOW-DESIGN|Footnote Flow integration]]. The earlier 54-run proposal below remains a reference protocol, not the release gate for the integrated product. First validate a real Footnote-backed task sequence across fresh sessions. Then compare the same Footnote stack alone, with skills, and with skills plus Flow; use a separate Footnote-disabled control to attribute family benefits. No new confirmation runs have been launched for that design.

The first live three-arm development calibration is complete; see [[AI-Agents/projects/footnote-framework/LIVE-CALIBRATION|Live calibration]]. All three coded artifacts passed six common independent checks. Provider response usage was recovered for the baseline, skills-only worker, Flow coordinator and Flow specialist. The repeated confirmation study has not run. The earlier six review runs and deterministic fixtures remain separate evidence. This protocol measures delivered work, including failures and repairs, rather than treating unit tests or shorter instructions as proof of agent efficiency.

Johan explicitly authorized a measurement-only adapter for this evaluation on 17 November 2026. The actual predictor remains unidentified and deferred. evaluation/measurement_adapter.py implements the authorized boundary: purpose=measurement-only, prediction_enabled=false, null predictions and an evaluation run ID. It requires the explicit frozen project decision and an evaluation runtime name. Normal production dispatch requirements remain unchanged. Baseline and skills runs alone cannot complete this comparison.

The adapter accepts actual per-call usage supplied by a trusted runtime host, rejects duplicates/cumulative snapshots, and preserves unknown counts. It does not itself launch models, authenticate provider receipts, prove exhaustive child capture or persist recovery state. A live session runner and receipt collector are still required. Its real-mode contract test deliberately makes no model call and is calibration evidence only.

## Arms and comparisons

| Arm | What changes |
|---|---|
| A, baseline | Same harness/model/tools and common safety rules, no curated Flow cards/helpers, no Flow orchestration |
| B, skills | Same baseline, selected frozen cards/references/helpers available, no Flow orchestration |
| C, Flow | Same selected pack as B, plus real governor planning/dispatch/verification/review/recovery; all coordinator and specialist activity charged to this arm |

A is no curated skills, not an instruction-free model. Record globally surfaced skills/rules and contamination. Use a clean isolated session configuration where supported; never weaken safety requirements. Freeze the shared development policy, including ordinary delegation permissions. Do not give C tools or reference material unavailable to A except the declared treatment. All arms receive the same user objective and initial project information. C's generated plans and extra verification are part of its time/token overhead.

B minus A estimates the selected skill bundle effect. C minus B estimates added orchestration under the chosen runtime configuration. C minus A estimates the combined system. This does not establish individual-card effects or isolate the predictor. A later card-removal experiment can answer which skill earns its cost.

## Metrics and clocks

Use parent run IDs, agent/attempt IDs and call IDs. Record raw events and authoritative usage separately from derived summaries. Use a monotonic clock for same-host durations; UTC timestamps link logs. Never subtract independent machines' monotonic clocks. Preserve null when a timestamp or token count is unavailable.

| Metric | Definition |
|---|---|
| Time to start | Submission to first executor ready; report queue/session preparation separately where observable |
| Time to useful action | Submission to first task-relevant read/edit/test, excluding status messages and the RTK boot probe; classify from logs using frozen rules |
| Time to first artifact | Submission to first artifact eligible for independent evaluation; not automatically acceptance |
| Time to accepted delivery | Submission to independently accepted artifact, including planning, tests, critic, repair and orchestration |
| Terminal duration | Submission to success, failure, timeout or unresolved interruption for every run, including unsuccessful ones |
| Model latency | Per-call request to first token and completion, only when provider/runtime events expose them; separate queue/tool time and do not infer TTFT from response length |
| Token usage | Per inference call, agent, role, attempt and run: input, cached input, output, total. Cache is a subset of input; reasoning is not added twice. Include coordinator, specialists, model critic and retries |
| Workflow overhead | Spans for planning, skill loading, token activation, dispatch, tools, deterministic verification, review, persistence and recovery. Spans may overlap; sum is agent/service effort, not wall time |
| Quality | Hidden acceptance checks plus blinded rubric, first-pass acceptance, final acceptance, regressions, material security/correctness defects and incomplete requirements |
| Rework | Failed verification rounds, rejected artifacts, retries, duplicate effects, abandoned work and unnecessary changed files/dependencies |
| Human burden | Active clarification/review/repair minutes and intervention count, separately from passive waiting. Unknown time remains null |
| Recovery | Separate matched interruption scenarios: terminal reconciliation time, duplicate effects, lost progress and unsafe retry attempts |
| Cost | Actual or explicitly estimated provider-priced usage, with model/date/cache rates. Subscription quota and invoices are separate; token totals alone are not currency |
| Context/agent overhead | Agents launched, context bytes/tokens where exposed, skills loaded, inference count, peak concurrent agents and repeated input. Do not infer semantic redundancy from bytes alone |

Primary outcomes: final acceptance rate under a fixed cap, all-run consumed tokens, and wall time to accepted delivery. Report failed/time-limited runs alongside successful ones. Also report total suite resource cost divided by accepted outcomes, including failed attempts; undefined if none succeed. Never count an early failure as an efficiency win. Report resource and quality results separately before any composite score.

Tool/model/verification spans follow the start/end and parent-child concepts in OpenTelemetry. This protocol does not require adding an OpenTelemetry dependency. Point-in-time phase events alone cannot establish TTFT or an accurate tool/model latency breakdown.

## Workload and fairness

Initial confirmation target: six independent held-out task snapshots, three repetitions per arm, 54 model runs. This is a local starting design, not a statistically justified sample size or guarantee of power. Choose/freeze actual tasks and acceptance tests before running; task selection is still pending. Cover bug repair, feature implementation, performance improvement with correctness checks, cross-module integration, and two distinct multi-step tasks. Keep language/domain strata explicit rather than hiding them in an aggregate.

Use isolated copies with matching input/environment hashes. No task may use the already-developed duplicate-key fix or known pilot answers as held-out evidence. Material must not have been used to tune the cards. If suitable untouched tasks cannot be found, label the study development evidence. Write hidden checks independently and keep them inaccessible to task agents. The external evaluator must run identically across all arms; Flow's internal checker is part of treatment and cannot replace the common evaluator.

Block by task and repetition and randomize arm order within each block. Precompute a balanced schedule and store its seed. Start with sequential blocks to avoid CPU/provider contention between treatments. Record cache eligibility/hits and avoid calling a fresh session a cold provider cache. Run separate process-cold and process-warm conditions only when controllable; otherwise report observed cache state and uncertainty.

Freeze provider/model/version/effort/tool permissions, cap on concurrency, time and tokens, source/pack/runtime revisions, environment, retrieval/network policy and all deviations. Shared caps are local resource policies chosen before the experiment, not performance claims. Track Flow's actual concurrency and include every child in totals. If a cap is reached, save a failed or censored result; do not quietly rerun until successful. Independent model scoring is charged separately as evaluation cost in every arm, while Flow's internal model critics count as execution cost.

Run instrumentation calibration first, then a labelled development smoke block, then freeze the confirmation workload. Do not tune cards or framework between confirmation arms. If tuning is needed, archive the development series, increment revisions and restart the confirmation series on untouched tasks.

## Analysis and decision

Keep every raw run, including failure and missing telemetry. First aggregate repetitions within each task, then compare paired task-level outcomes. Do not treat agents, model calls or repeated runs on the same task as independent task samples. Report per-task raw values and paired changes, medians/ranges, cache counts, failures and uncertainty. With six tasks, uncertainty remains wide; do not claim broad generalization or a stable p95. Tail estimates need a separately justified larger sample.

Blind the final artifact evaluator to arm where feasible and disclose unavoidable metadata leakage. Freeze quality margins and worthwhile time/token differences before viewing confirmation results. Until those margins and enough data exist, report trade-offs rather than naming a universal winner. Flow can legitimately help complex tasks while costing more on small tasks; report that interaction instead of averaging it away.

Decision rule: no efficiency promotion if acceptance worsens materially, a serious defect is introduced, telemetry omits coordinator/children, or remaining uncertainty spans both a worthwhile improvement and harm. Keep skills experimental until supported by repeated comparable work.

## Execution prerequisites

The independent telemetry normalizer is evaluation/receipts.py. It derives start/useful-action/artifact/acceptance/terminal durations and per-agent/per-run counts from explicit same-host events and per-call receipts. It rejects duplicate calls, cumulative snapshots, wrong-mode counts and invalid times. Missing counts or incomplete host capture do not become a zero-token run. Its six tests and CLI calibration are fixture evidence only, not real agent measurements.

```powershell
rtk proxy python -m unittest evaluation.test_receipts -v
rtk proxy python -m evaluation.receipts path/to/normalized-run-receipt.json
```

1. Freeze six actual task snapshots and hidden acceptance checks.
2. Implement/verify a real session runner with complete coordinator/child inference receipts and terminal handles.
3. Completed: evaluation-only measurement adapter authorized and implemented without changing normal dispatch requirements. Wire it to live authoritative usage; do not call its calibration tests actual model execution.
4. Validate clocks, usage deduplication, null handling and failed-run accounting with calibration fixtures.
5. Freeze revisions, caps, run order and blinded scoring, then execute all three arms.

## Sources

- [NIST randomized block designs](https://www.itl.nist.gov/div898/handbook/pri/section3/pri332.htm), accessed 17 November 2026. Blocking/randomization rationale; the particular task counts here are local choices.
- [OpenTelemetry traces](https://opentelemetry.io/docs/concepts/signals/traces/), accessed 17 November 2026. Span structure and parent/child operations.
- [[AI-Agents/projects/footnote-framework/RUNBOOK|Current implementation boundaries]] and evidence/pilot-runtime-usage.json.
