---
type: reference
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Flow Live Calibration
date_maj: 2026-11-17
---
# Flow live calibration

Up: [[AI-Agents/projects/footnote-framework/FULL-EVALUATION|Full evaluation]] · Related: [[AI-Agents/projects/footnote-framework/STATUS|Status]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## Actual execution

Three arms executed a real coding task using fresh native agents, inherited gpt-6.1-sol and medium effort. Baseline had no selected Flow cards. Skills-only loaded python-performance. Flow used the real Governor, measurement-only activation, a separate model coordinator and one native specialist. The coordinator validated the saved contract/activation and launch hashes, waited for the child, reran public tests, imported its report into the original attempt and verified it. The same parent-only acceptance suite checked all three artifacts afterward.

All three passed six independent checks, including exact arithmetic beyond floating-point integer precision, invalid input, units, fractional-byte rejection, whitespace and delivery files. This is a synthetic development calibration authored during runner construction. It is not a held-out confirmation task or evidence of general framework effectiveness. No source in D:/Projets changed.

| Observed quantity | Baseline | Skills only | Flow |
|---|---:|---:|---:|
| Native task duration, seconds | 61.571 | 52.211 | 129.407 |
| Launch-call to native-task-entry, ms | 220 | 185 | 193 |
| Agent count | 1 | 1 | 2 |
| Inference calls | 7 | 6 | 18 |
| Total input plus output tokens | 265212 | 228801 | 812517 |
| Cached input, subset of input | 225024 | 224256 | 786304 |
| Uncached input | 38182 | 2833 | 21803 |
| Output tokens | 2006 | 1712 | 4410 |
| Independent acceptance checks passed | 6/6 | 6/6 | 6/6 |

Flow's duration includes waiting for its specialist; do not add the specialist's 51.950 seconds to the coordinator's 129.407 seconds. Its token total includes both coordinator and specialist. Actual provider response IDs and per-call counts reconcile with cumulative child-session usage. Unknown report usage was initially left null, then actual native receipts were exported separately through the measurement adapter; the governor report was not retroactively fabricated.

The baseline's first inference had no cached input, whereas later arms benefited from cache. Arm order was baseline, skills, Flow, without randomization. Task duration ends at native agent/coordinator completion, not external acceptance. Setup, source preparation and common parent evaluation were excluded from all arms. Flow used a prewritten plan, not autonomous plan generation. These boundaries prevent claiming a controlled efficiency result even though the execution and usage records are real.

## Timing and accounting limits exposed

- Native launch timestamps and task-entry timestamps can be linked from the parent and child logs. Their difference uses same-machine UTC, not monotonic instrumentation. It measures entry into the native task, not model readiness or first useful action.
- Provider response IDs expose actual input/cache/output counts. Repeated cumulative snapshots are checked and cannot silently double-count a response. Five telemetry-reader tests cover reconciliation, wrong parent/thread, duplicate response and incomplete tasks.
- TTFT, first artifact creation and submission-to-external-acceptance were not instrumented completely. They remain null. First tool time is not automatically first useful action.
- Selected-session capture covers the model coordinator and child in this calibration. Experimental setup by the parent is excluded and recorded as a limitation, not called zero overhead. API costs remain unknown.
- The Python handoff bridge reserves an attempt before a native launch. SystemExit means waiting for the host, not terminal worker failure. The host reconciles the same attempt without a second dispatch. This manual bridge validates the boundary but is not yet an unattended 54-run benchmark runner.

## Implication and remaining work

On this small task, Flow incurred extra time and token volume with equal check coverage. Skills-only also passed, but its lower observed usage/time is not a causal improvement estimate. No winner is established. The result supports measuring complexity-dependent overhead instead of assuming more agents always help.

The repeated six-task, three-repetition study remains unexecuted. Before that study, freeze actual independent task snapshots and hidden checks, balance/randomize order and cache handling, and automate timestamps plus external acceptance in the host bridge. Retain failures and resource use. Do not rerun this known calibration as held-out confirmation. The live predictor remains deferred under the user's explicit measurement-only authorization.

## Evidence

- evidence/live-calibration-results-v1.json: normalized arm totals, timing boundaries and limitations.
- evidence/live-calibration-baseline-v1.json, live-calibration-skills-v1.json, live-calibration-flow-governor-v1.json and live-calibration-flow-implementer-v1.json: model/effort, exact session identities and provider response usage.
- evidence/live-calibration-flow-measurements-v1.json: labelled adapter activations and reconciled actual calls, with prediction_enabled=false.
- evidence/live-calibration-baseline-checks.txt, live-calibration-skills-checks.txt and live-calibration-flow-checks.txt: independent six-check outcomes.
- build/live-calibration-v1: disposable arm workspaces and persisted governor/handoff/host-result. The frozen objective/source also live in evaluation/live_smoke.py so the experiment can be rebuilt at a new destination.
- evaluation/native_receipts.py, test_native_receipts.py and smoke_checks.py. These collect/check evidence; they do not supply candidate implementations.
