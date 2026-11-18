---
type: project-plan
tags:
  - domain/ai/agents
  - domain/dev/workflow
  - type/planning
  - project/borg
  - status/active
aliases:
  - Flow Status
  - Footnote Framework Status
date_maj: 2026-11-17
---
# Flow status

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Footnote Framework]] · Review: [[AI-Agents/projects/footnote-framework/SOURCE-REVIEW|Source review]] · Evaluation: [[AI-Agents/projects/footnote-framework/EVALUATION|Evaluation]]

#domain/ai/agents #domain/dev/workflow #type/planning #project/borg #status/active

## Current product direction

One explicitly requested Footnote-only addition now passed the same six checks: Luna medium, 45.425 seconds, nine calls, 323,776 total tokens. The earlier Footnote-plus-skills run used 31.482 seconds and four calls. This exploratory single addition does not establish a skill effect; details and evidence are in [[AI-Agents/projects/footnote-framework/LUNA-PILOT|Luna calibration]]. No additional workload or model tier was launched.

The first requested lower-tier batch is complete: [[AI-Agents/projects/footnote-framework/LUNA-PILOT|GPT-6 Luna medium calibration]]. Four fresh delegates, one synthetic task per arm, all six external checks passed, zero delivery reruns. Skills plus Footnote used four calls and 31.482 worker seconds; baseline nine and 51.334; Flow prototype seven and 37.392; skills nine and 41.083. Actual child usage reconciles; no billing saving or general ranking is established. Stopped at four launches to preserve the user's remaining OpenAI resources. Cross-model and real-maintenance confirmation remain unrun.

Johan added model tier as a benchmark dimension: lower-cost models may benefit more from Footnote tools/context, while stronger models may complete difficult work with fewer repairs. These remain hypotheses. [[AI-Agents/projects/footnote-framework/MODEL-STUDY|Model x stack x task study]] specifies attribution, per-agent/run consumption, repair definitions, linked overview charts and staged sizes of 24 development runs then 144 confirmation sequences for two model configurations. Exact model/settings, resource envelope, independent tasks and full timing/Usage attribution are not frozen; no new model-comparison run was launched.

A first real product loop now works in an isolated profile; see [[AI-Agents/projects/footnote-framework/INTEGRATED-LOOP|Working integration]]. Real Code/Memory/Library CLI responses fed one working agent per session, external checks gated a parent-reviewed lesson, and a fresh second session recalled it and preserved compatibility during a follow-up change. Real stale-Code/stale-Library refusal and foreign-project memory exclusion were exercised. This is a controlled fixture, not effectiveness proof or shipped product integration. The external bridge is integration/footnote.py; reproducible commands are integration/proof.py. Durable receipts are under evidence/integrated-loop-v1. Usage attribution, production Rust integration, full governor lifecycle and safe automated admission remain unfinished.

Johan clarified that Footnote Flow must extend the family products and improve output beyond curated skills. See [[AI-Agents/projects/footnote-framework/FOOTNOTE-FLOW-DESIGN|Integrated design]]. The default proposed path is one working agent with deterministic context assembly, trusted checks, reviewed learning and Usage attribution. Existing Code-to-Memory/Library links are the starting point; scope/freshness/version metadata must be sufficient before automatic delivery. Product integration and demonstrated benefit are unfinished. PROJECT.md stage D records acceptance. The old 54-run study is not the release gate for this revised product.

This design pass inspected current Footnote source/graph and probed Memory MCP health; it changed no Footnote source or live memories and launched no benchmark agents. Inputs/limits are recorded in evidence/footnote-integration-design-inputs.json. The source tree advanced during inspection, so working-file hashes are the evidence identity. The real token predictor remains unidentified/deferred; the measurement-only evaluation exception remains authorized.

## Historical reference handover

The standalone curated toolkit and project-execution foundation satisfy PROJECT.md. See [[AI-Agents/projects/footnote-framework/HANDOVER|Handover]] for the acceptance audit, source map, performance interpretation and deferred integration proposal. Johan chose the name Flow on 17 November 2026. The existing folder path is retained; naming availability was not checked.

At the reference handover, product integration was deferred. The user's later direction above supersedes that product boundary. The prediction product's location/API remains unidentified. The reference implementation requires bound activation for every specialist dispatch and refuses fixture/real mismatches. Fixture success is never called live integration.

## Final evidence

- Eleven competence skills, six jobs and the reused website dependency; seven thematic distributions. Current export build/toolkit-v0.2.0 has 18 cards and 31 hashed artifacts plus manifest. Exports are experimental and uninstalled.
- Independent content/source/code reviews with parent corrections. Source curation is separate from behavioral benefit. The final verifier receipt/cache corrections have regression evidence, not another independent review pass.
- All 62 tests pass: 45 governor/verifier, 10 standalone comparator and 7 pack-builder. Logs: evidence/governor-tests.txt, helper-tests.txt and pack-builder-tests.txt.
- All 18 exported cards pass format validation; 31 hashes match; the shipped helper runs under isolated Python outside this project. See evidence/export-validation-v0.2.0.json.
- Current scan evidence/skillspector-export-v0.2.0.json: no failed/incomplete/uninspected files, five identical LP3 medium tool-scope leads on helper-bearing cards. Manual scope review remains explicit; a static grade is not a sandbox or exploit proof.
- Three paired real-project review tasks, six exact child outputs and runtime input/cache/output/time receipts are preserved. Cost remains unknown. Small nonblind development evidence does not establish a general winner.
- evidence/code-change-acceptance.json records the existing parser change through a frozen plan, fixture editor and actual host-registered Python suite: new regression fails on baseline, ten candidate tests pass. Canonical promotion checked baseline equality and durable acceptance. Original helper/tests remain under evidence/code-change-inputs.
- build/workflow-demo-v0.2.1/result.json records dependent summary-fixture completion, simulated review, local memory admission and one normalization effect. Tests also terminate real child processes in both execution and verification phases, reconciling without a second effect.
- evidence/governor-benchmark-v0.2.1.json records ten local fixture measurements after two warmups, median about 199 ms. This excludes startup, models, named test-suite execution, runtime containment and Footnote; no SLA/p95/general-efficiency claim.

## Reproduction and future work

Live three-arm calibration has now run on one synthetic coding task, with four fresh native agents including Flow's coordinator and specialist. All arms passed six common independent acceptance checks. Actual per-response usage reconciles to the selected sessions; see [[AI-Agents/projects/footnote-framework/LIVE-CALIBRATION|Live calibration]] and evidence/live-calibration-results-v1.json. Flow used more token volume and time on this task. This is not a controlled efficiency result: fixed order/cache differences, one task, prewritten plan and incomplete end-to-end timing. The manual native handoff is not yet an unattended runner. The 54-run confirmation study remains pending; no general winner is claimed.

The new three-arm efficiency study is specified in [[AI-Agents/projects/footnote-framework/FULL-EVALUATION|Full evaluation]]. Johan authorized the measurement-only evaluation adapter on 17 November 2026. evaluation/measurement_adapter.py implements labelled activation and host-supplied per-call usage with null predictions; five adapter tests and six telemetry-normalizer tests pass alongside the 45 framework tests, 56 total. These are calibration checks, not measured model benefits. Actual held-out tasks and a live runtime/session receipt collector remain prerequisites for three-arm confirmation. The foundation handover remains complete; efficiency acceptance is a separate open workstream.

Use RUNBOOK.md. Choose a new build destination for each demonstration/export and a new benchmark output. Source is standard-library Python except the pack builder's pinned PyYAML. build is ignored/disposable; durable validation and accepted-code receipts are under evidence.

Future product work is mapped in HANDOVER.md: decide the owning Footnote module/ADR, reuse RingMaster launch/budget/containment, connect the actual predictor, bind whole-pack trust, reconcile authoritative usage and connect approved memory admission. Add shared serialization vectors before a Rust bridge. Authenticate actors and provide real runtime handles before production execution. Capabilities remain usable independently throughout.

The graph inventory was refreshed: this vault project and RingMaster are not indexed. Bounded source observations and working-file hashes are in evidence/integration-source-inventory.json. No full runtime audit or graph-completeness claim is made.

## Workspace and process safety

No prototype process or child remains live. Review/pilot agents and scans completed. No D:/Projets source was changed. No installation, registration, signing, commit, push or publishing occurred. Preserve unrelated modifications to the vault style guide, AI-Agents/skills/README.md and Agents/Sentinel/log.md.

Vault normalization is dry-run only, with output in evidence/vault-maintain-dry-run.txt. The python3 name resolves to the Windows Store alias, so the repaired equivalent uses RTK-wrapped uv and python. No --apply is authorized or executed.

## Sources

- [[AI-Agents/projects/footnote-framework/PROJECT|Project contract]], [[AI-Agents/projects/footnote-framework/HANDOVER|Handover]], [[AI-Agents/projects/footnote-framework/CORE-REVIEW|Core review]] and [[AI-Agents/projects/footnote-framework/RUNBOOK|Runbook]].
