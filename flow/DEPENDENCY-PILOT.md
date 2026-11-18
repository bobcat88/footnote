---
type: reference
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
aliases:
  - Dependency pilot
  - Luna dependency study
date_maj: 2026-11-17
---
# Dependency pilot

Up: [[AI-Agents/projects/footnote-framework/MODEL-STUDY|Model study]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg

## Frozen protocol

User authorized the harder follow-up to [[AI-Agents/projects/footnote-framework/LUNA-PILOT|Luna pilot]]. Five fresh Luna medium workers, one delivery each, serial seeded order Flow, Footnote only, skills only, Footnote plus skills, baseline. No repair delegate, model escalation or additional repetitions. Twelve responses and eight minutes are requested soft limits, not enforced provider caps.

The synthetic service has interval merging, availability, billing, API fan-out and a JSON CLI. The migration adds an opt-in policy while preserving billing segment fees and reusing a one-shot iterable across consumers. Every arm starts with identical source, task, contract and decision hashes. Baseline receives the same facts as retrieved arms. Skills arms load the same exported Python performance card. Footnote arms receive an actual isolated Code/Memory/Library context receipt. Flow adds a frozen deterministic governor plan and measurement activation. No separate LLM coordinator or prediction product is used.

Fourteen parent-only checks were frozen before launch. The broken starting source fails and a separately retained known-valid control passes. Three existing public tests are immutable. Independent checks cover transitive joins, invalid policies/intervals/bounds, purity, generators, availability clipping, strict billing, API fan-out, CLI output and a 5,001-segment chain. The large input is a correctness smoke check, not proof of asymptotic performance.

Primary outcomes are independent acceptance, actual per-call input/cache/output usage, worker time and delivery reruns. Cached input is a subset of input. Parent setup and reporting are excluded from child usage. One sample per arm cannot establish causal savings, a universal ranking or overhead dilution across complexity levels. The task is more connected than the first pilot but remains a small synthetic service, not a difficult production repository.

Sources: `evaluation/dependency_fixture.py`, `evaluation/dependency_pilot.py`, and `build/luna-medium-dependency-v1/study.json`. Results pending the bounded five-run campaign. No production Footnote profile or live user memory is modified.

## Completed results

All five candidates passed 14 independent functional checks and three existing tests. Full delivery acceptance differs. Baseline stopped after an unconfirmed patch result, with regression tests and delivery notes missing. Footnote plus skills introduced unavailable pytest in its tests, violating the standard-library-only requirement. This host contract audit corrects the earlier provisional acceptance. Original automated result retained as result-before-contract-audit.json.

| Setup | Seconds | Calls | Total tokens | Uncached input | Full delivery |
|---|---:|---:|---:|---:|---|
| Baseline | 38.565 | 8 | 282,163 | 37,502 | Incomplete |
| Skills | 66.823 | 11 | 407,572 | 40,268 | Accepted |
| Footnote only | 50.709 | 10 | 365,381 | 39,486 | Accepted |
| Footnote + skills | 53.260 | 8 | 298,985 | 40,340 | Test dependency violation |
| Flow prototype | 47.591 | 8 | 305,287 | 41,328 | Accepted |

Flow was fastest and used fewest total tokens among accepted deliveries in this sample. It also used the most uncached input. No billing saving is established. All code candidates passed the dependency checks, so there is no observed downstream correctness advantage. The skill-overhead-dilution hypothesis remains unproven. Baseline's partial delivery and tool uncertainty prevent treating its shorter time as time to an accepted result. One sample per setup, differing cache states, a skills-arm path clarification, and parent response interruptions limit comparison. No delivery repair, replacement, predictor or stronger-model run followed.

Runtime usage reconciled per call and to cumulative snapshots. Total child usage is 1,659,388 tokens, including 1,449,856 cached input, 198,924 uncached input and 10,608 output. Parent work is excluded. Exact receipts, immutable-input hashes, source candidates, controls and completed T3 metadata are in build/luna-medium-dependency-v1/. Production Footnote integration remains outside this test.
