---
type: reference
tags:
  - domain/ai/agents
  - domain/dev/architecture
  - type/reference
  - project/borg
  - status/active
aliases:
  - Flow Handover
  - Footnote Framework Handover
  - Footnote Flow Handover
date_maj: 2026-11-17
---
# Flow handover

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Footnote Framework]] · Related: [[AI-Agents/projects/footnote-framework/RUNBOOK|Runbook]] · [[AI-Agents/projects/footnote-framework/ARCHITECTURE|Architecture]]

#domain/ai/agents #domain/dev/architecture #type/reference #project/borg #status/active

## Delivered scope

This is the historical standalone reference handover. The current product stage is [[AI-Agents/projects/footnote-framework/FOOTNOTE-FLOW-DESIGN|Footnote Flow integration]], required by Johan's clarified direction. Earlier foundation acceptance does not mean integrated product completion. Preserve these receipts while implementing and measuring the new stage.

The standalone toolkit and project-execution foundation satisfy the scoped PROJECT.md contract. They are ready for technical handover, not production rollout or full GSD feature parity. The user explicitly deferred Footnote and its token-prediction product. No real model/runtime adapter is shipped, and fixture activation never authorizes a real-mode executor. The predictor's actual project/API remains unidentified; usage accounting was not assumed to be prediction.

Chosen name: Flow, confirmed by Johan on 17 November 2026. The existing footnote-framework folder stays in place for path continuity. No brand or package availability check was performed.

## Where things live

| Deliverable | Source and evidence |
|---|---|
| Eleven competence skills and six job skills, plus reused website checklist | AI-Agents/skills; CAPABILITIES.md maps the seven packs |
| Authoring and performance contracts | AI-Agents/_shared/skill-authoring-standard.md; cards link primary sources and research dates |
| Reproducible uninstalled distribution | capabilities/build_packs.py and packs.json; current build/toolkit-v0.2.0 contains 18 cards, 31 hashed artifacts and manifest |
| Standalone receipt comparator | capabilities/scripts/compare_receipts.py; ten tests; no governor, network or subprocess dependencies |
| Durable project coordinator | orchestration/contracts.py, store.py, governor.py; standard-library Python, verified on 3.12.10 |
| Trusted named suite adapter | orchestration/repository_tests.py; frozen host descriptor, actual runner completion and bounded transcript |
| Repeatable examples | orchestration/demo.py, code_change.py and benchmark.py; choose new output paths |
| Content/security reviews | REVIEW.md, SOURCE-REVIEW.md and CORE-REVIEW.md; raw static reports and logs under evidence |

build is ignored and disposable. Rebuild from source using RUNBOOK.md; durable accepted-code receipts are retained under evidence. No card was installed or registered with RingMaster, and no signatures were created.

## Acceptance audit

| PROJECT.md requirement group | Evidence and interpretation |
|---|---|
| A: researched text, selective composition, performance discipline | 11 competence plus 6 job cards; reused website dependency; source-review matrix distinguishes facts, local policies and review limits |
| A: independent use and justified scripts | 7 builder and 10 helper tests; export-validation-v0.2.0.json validates all 18 formats/31 hashes and runs shipped helper outside the project under isolated Python |
| A: audit before installation | skillspector-export-v0.2.0.json: 18 inspected skills, zero failed/incomplete/uninspected files; five identical LP3 medium tool-scope leads, aggregate per affected card 7/100 LOW. These are scope leads, not confirmed exploits. Manual helper review finds bounded caller-selected file reads and stdout only; host permissions still matter |
| A: realistic evaluation and honest benefit claims | Three paired real-project review tasks, frozen inputs, six exact child responses and runtime token/time receipts. Single-run nonblind development evidence; baseline already strong. No universal skill advantage or cost-saving claim |
| B: complete existing-code change | code-change-acceptance.json contains baseline failure, candidate hash, actual ten-test passing receipt, plan/report/events and memory decision. Frozen originals under code-change-inputs; canonical promotion checked baseline equality and durable acceptance |
| B: governor, bounded handoffs, authority and memory | 45 framework tests in governor-tests.txt; reports cannot self-accept, contexts/artifacts/dependencies are checked, local admission requires governor rationale and preserves supersession |
| B: interruption and verification recovery | Tests terminate actual child processes in execution and verification phases, then reconcile the same attempt without repeating effects. Unknown outcomes stay reserved. Duplicate/competing claims, corruption, failed activation and bounded revision are tested |
| B: optional judgment | Summary fixture reaches a distinct simulated critic and recorded review evidence. Native independent code review checked actual code; caller strings are not authenticated reviewer identities |
| B: token-layer requirement | Every dispatch requires attempt/brief/mode-bound activation; fixture/real mismatch fails. Missing prediction counts remain null. Real connector is explicitly deferred |
| C: reproducibility and product handover | RUNBOOK.md, this audit, architecture/review files, source/test logs, source inventory and raw benchmark receipts |

The final three suites pass 62 tests: 45 framework, 10 helper and 7 builder. The second reviewer found the early-zero-exit verifier defect; parent corrections and regressions pass. The final completion-receipt/cache corrections have not had another independent pass. Static skill scanning is not a framework penetration test or proof of absence of vulnerabilities.

## Performance and bloat

The final local summary fixture benchmark records ten samples after two warmups, median about 199 ms, in evidence/governor-benchmark-v0.2.1.json. It includes coordination, SQLite and artifact I/O on the recorded Windows/Python machine. It excludes Python startup, model inference, runtime containment, Footnote and external services. It does not measure the named suite or full project latency and establishes no SLA, tail percentile or universal baseline. Raw samples and source/pack/environment fingerprints are retained; older receipts are historical, not a causal speedup comparison.

The capability layer remains independent. Loading a job selects relevant cards and conditional references; it does not load an entire pack or create children. The governor owns delegation. Fresh agents still carry substantial shared harness context, as the pilot receipts show. Track correctness/accepted outcomes alongside full input, cached-input, output tokens, elapsed time, retries and human review. Keep unknown cost null. Use paired repeated tasks before deciding whether a capability or extra agent earns its overhead; do not optimize card length alone.

## Footnote/RingMaster integration proposal, deferred

This mapping comes from bounded source inspection with recorded working-file hashes in integration-source-inventory.json. The vault project and RingMaster were not graph-indexed, and their full production behavior/test suites were not audited. Recheck current source before implementation.

| Responsibility | Proposed owner and next integration action |
|---|---|
| Project objective, plans, dependency/attempt state, acceptance and admission decisions | Project-coordination module. Port tested contracts into the chosen Footnote Rust product boundary after an ADR; preserve CLI-testable logic and one-way dependencies. Do not turn a retrieval index into workflow authority |
| Model launch/routing, budgets, leases, process trees and OS containment | Existing RingMaster runtime, not a second governor money/runtime kernel. Bridge task/attempt to durable runtime-turn/terminal handles; retain uncertain state until runtime reconciliation |
| Capability registration/trust | Export manifest plus all transitive helper/reference hashes, bound to existing runtime approval/signature policy. Standard SKILL.md export is not automatic approval. Register only selected capabilities |
| Per-child token prediction | Actual prediction product adapter once identified. Activate before each authorized child with task/attempt/runtime/brief identity; refuse missing, wrong-mode, stale or failed activation. No waiver flag |
| Usage and costs | Runtime usage/billing authority. Reconcile available counts and predictions; cache is a subset of input. Reuse accounting instead of maintaining a second billing ledger |
| Durable memory | Governor decides admission with rationale/provenance; an eventual Footnote adapter writes approved candidates through the product's memory API. Execution audit history stays separate; local SQLite is only the reference store |
| Identity and independent judgment | Authenticated host principals and real critic/runtime receipts. Current actor strings and simulated critic establish protocol behavior only |
| Cross-language interoperability | Shared serialization test vectors for encoded briefs, receipts, null counts and hashes. Current Python encoding is not an RFC 8785/JCS implementation |

Suggested integration order: choose owning product/module and ADR, connect authenticated runtime handles and containment, identify/connect prediction, bind capability trust, connect authoritative usage and memory admission, then run real end-to-end/recovery evaluations. Preserve standalone skills throughout. This is future work, not silently created production integration.

## Operating limits and workspace status

Trusted Python tests execute repository code. The adapter's temporary receipt, hashing and path checks do not contain malicious source, enforce disk quotas, authenticate host actors or make the audit log tamper-proof. The host must manage those guarantees before real execution. No distributed scheduler, background daemon, automatic uncertainty retry, UI product, deployment or GSD command-parity layer was added.

No source was changed in D:/Projets repositories. No skills were installed, no model was called by the prototype, and nothing was committed, pushed or published. Unrelated vault edits remain untouched. Vault maintenance was dry-run only; its global proposals were not applied. The standalone handover is complete; real product integration and broader efficacy validation remain explicit later work.

## Sources

- [[AI-Agents/projects/footnote-framework/PROJECT|Acceptance contract]], [[AI-Agents/projects/footnote-framework/CORE-REVIEW|Core review]], [[AI-Agents/projects/footnote-framework/SOURCE-REVIEW|Source review]] and [[AI-Agents/projects/footnote-framework/EVALUATION|Pilot protocol]].
- [Footnote structure ADR](D:/Projets/BorgKnowledge/docs/adr/0001-repository-structure.md) and [RingMaster runtime source](D:/Projets/RingMaster/crates/rm-kernel/src/kernel.rs), bounded inspection only.
- evidence/export-validation-v0.2.0.json, skillspector-export-v0.2.0.json, governor-tests.txt, helper-tests.txt, pack-builder-tests.txt, code-change-acceptance.json, governor-benchmark-v0.2.1.json and pilot-runtime-usage.json.
