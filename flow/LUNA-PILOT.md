---
type: experiment
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Luna Medium Calibration
date_maj: 2026-11-17
---
# Luna medium calibration

Up: [[AI-Agents/projects/footnote-framework/MODEL-STUDY|Model study]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## Observed results

Four fresh T3 delegates executed `gpt-6-luna`, reasoningEffort `medium`, in serial fixed-seed order Flow, Footnote, baseline, skills. Each repaired the same half-open interval function, added an opt-in touching-boundary flag, validation and public tests. All four passed the same six pre-existing independent checks on their first delivery. The broken baseline failed that checker and a retained known-valid candidate passed before launch. Same starting source and task/contract/decision hashes were asserted across arms. No stronger-model fallback or model coordinator was used.

| Stack | Worker seconds | Model calls | Input tokens | Cached subset | Output | Total |
|---|---:|---:|---:|---:|---:|---:|
| Baseline | 51.334 | 9 | 316,596 | 278,528 | 2,002 | 318,598 |
| Skills | 41.083 | 9 | 322,267 | 283,776 | 1,597 | 323,864 |
| Skills + Footnote | 31.482 | 4 | 141,993 | 104,064 | 1,284 | 143,277 |
| Flow prototype | 37.392 | 7 | 257,943 | 218,240 | 1,499 | 259,442 |

Footnote plus skills was fastest in this sample, about 38.7% below baseline worker time and 55.0% below total tokens. Flow was about 27.2% below baseline worker time and 18.6% below total tokens. Skills alone used about 1.7% more total tokens and finished about 20.0% sooner. These are observations from one task per arm, not causal savings or a ranking.

Uncached input was 38,068 baseline, 38,491 skills, 37,929 Footnote and 39,703 Flow. Most of the total-token difference was repeated cached context and call count. Cache states differed. Total child usage was 1,045,181 tokens, including 884,608 cached input, 154,191 uncached input and 6,382 output. This excludes the common parent preparation, measurement and reporting model usage. No dollar estimate, billing saving or subscription quota conversion is claimed.

## Scope and limits

### Exploratory Footnote-only addition

Johan requested one additional run with Footnote but no curated skills, keeping Luna medium and the same task/source/contract/decision hashes. It passed all six independent checks on its first delivery with zero delivery reruns. Worker time was 45.425 seconds, nine model calls, 321,975 input tokens including 283,520 cached input, and 1,801 output, totaling 323,776. Uncached input was 38,455. The requested eight-response soft limit was exceeded by one response. Per-response usage reconciled to cumulative snapshots.

Compared with the earlier skills-plus-Footnote run at 31.482 seconds and 143,277 total tokens, this observation favours retaining the skill card for further testing. It does not establish causality: one sample each, differing cache state and a later, unbalanced addition after viewing the first batch. The common runtime instruction context remains present even without curated skill loading. No stronger-model fallback or additional run followed this addition. Evidence and exact T3/provider identities are in `evidence/luna-footnote-only-v1/`. The initial four-run total above remains historical; this addition consumed 323,776 further child tokens.

One synthetic development task and one repetition per arm. This tests launch/access, current integration and telemetry, not difficult repository maintenance or model-tier superiority. Baseline receives the same original documentation and project decision as the other arms. All delegates inherit common T3 runtime instructions. Isolation is by assigned paths and prompts in the shared checkout, not a security sandbox.

The Footnote arm reads an actual bounded Code/Memory/Library receipt. Flow adds the current reference governor's frozen plan, ownership, dispatch-bound measurement activation and host-gated acceptance. Both reuse the real integration bridge. This is the current Flow prototype, not completed production integration or a continuity/learning test. No memory writes were admitted from these runs.

Worker time spans T3 request to terminal completion. Full submission-to-acceptance time is separately recorded, but checks ran after the batch, so those delays contain host scheduling and must not rank arms. Initial fixture/index preparation timings are separate and include uniform setup even for the baseline; they are instrumentation costs, not a fair estimate of each product's production setup. Native task timing, host start delay and independent check time are also retained. TTFT is unknown.

Every arm had one delivery attempt and zero delivery reruns. Internal local edit/check repair counts were not instrumented. The requested eight-response limit was soft; baseline and skills each used nine responses. There was no enforceable token cap. The hard campaign bound was four launches with no parent repair or replacement. No more model runs were launched after this batch.

Provider response IDs and usage sums reconcile to runtime cumulative snapshots. T3 child logs lack native parent/agent fields; collection binds each explicit session ID to its unique task workspace and matching T3 start/end timestamps, then applies the existing reconciler. Each arm has a measurement-only activation and recorded calls. Adapter capture_complete remains false because it does not itself prove exhaustive collection; the host reconciliation result is recorded separately. Reasoning is already included in output and is not added again.

## Reproduce and inspect

- Preparation: `rtk proxy python -m evaluation.luna_pilot build/NEW-UNUSED-DESTINATION` from this project. Creates fixtures and controls; launches no agents.
- Frozen launch/terminal identities: `evidence/luna-medium-development-v1/t3-runs.json`.
- Result, per-response receipts, activation records, checker controls, accepted Flow state and candidate hashes: `evidence/luna-medium-development-v1/`.
- One-off collector with explicit session identities: `build/luna-medium-development-v1/collect.py`. It is an original receipt import, not an idempotent replay tool; do not repeat governor report submission.
- Owned candidate files: `build/luna-medium-development-v1/<arm>/workspace/`.

The larger [[AI-Agents/projects/footnote-framework/MODEL-STUDY|study]] still needs independent real maintenance tasks, production Usage attribution and a stronger-model matched comparison. This batch is not blended with the older parser calibration.
