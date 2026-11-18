---
type: project-plan
tags:
  - domain/ai/agents
  - domain/dev/workflow
  - type/planning
  - project/borg
  - status/active
aliases:
  - Flow
  - Footnote Framework
  - Footnote Flow
date_maj: 2026-11-17
---
# Flow

Up: [[AI-Agents/README|AI Agents Vault]] · Related: [[AI-Agents/_shared/skill-authoring-standard|Skill authoring standard]]

#domain/ai/agents #domain/dev/workflow #type/planning #project/borg #status/active

## Objective

Build an in-house project execution framework for the Footnote family, replacing the useful project-lifecycle functions of GSD. Deliver the independently usable curated skills first. Build the orchestration foundation second and hand it over with explicit integration boundaries and evidence.

Current direction, confirmed by Johan: Footnote Flow extends the actual Footnote family to improve output. The standalone governor is reference code, not the finished product. [[AI-Agents/projects/footnote-framework/FOOTNOTE-FLOW-DESIGN|Integrated design]] defines the next stage: deterministic context/verification/learning around the working agent, reusing Memory, Library, Code and Usage. Skills remain independently usable. A separate model coordinator is optional and must justify its cost. The earlier integration deferral is superseded for product design and integration work; the unidentified real prediction product remains deferred, with the explicitly authorized measurement-only adapter for evaluation.

The user's instruction of 17 November 2026 authorizes completing this project autonomously. It does not authorize publishing, paid service purchases, unrelated repository changes, or automatic installation of the curated packs.

## Product decisions

- Two independently usable layers: curated capability packs and orchestration. Orchestration depends on capabilities; capabilities do not require orchestration.
- Packs contain skills, progressively loaded references, and justified scripts. Installing a pack must not load all its content into a task.
- Skills identify relevant expertise. The coordinator owns agent creation; reading a skill must not silently start a workflow.
- Subagents report to a governor. The governor owns dispatch, integration, acceptance, and decisions about durable memory.
- Routine accounting and state transitions are deterministic. Model judgment is reserved for decisions that require it.
- Every framework-launched subagent must activate the user's token prediction layer. A failed or missing integration blocks real dispatch, with a visible reason. Test doubles must be labelled and cannot establish integration acceptance.
- Fit the current products first. No speculative multi-harness adapter platform.
- Current Footnote and RingMaster repositories are read-only discovery inputs until a concrete integration change is justified. Build the handover artifact here first.
- Text-only round 1 precedes executable helpers. No claim of measured benefit before a paired evaluation.
- Subsequent user steering: Footnote integration can be done later. The real token-layer connection and live Footnote execution are deferred acceptance items, not blockers to building the standalone foundation. Preserve a launch-time integration seam; never call a test predictor the real activated product.

## Naming

Chosen name: **Flow**, confirmed by Johan on 17 November 2026. The existing footnote-framework folder and previous names remain as paths and aliases for continuity. The governor remains an internal role. Brand and package availability have not been checked.

## Definition of done

### A. Standalone toolkit

- [x] Eleven competence text drafts: rust-performance, tauri-desktop-app, react-app-performance, python-performance, llm-rag-quality, sqlite-sql-performance, ci-release-engineering, backtesting-rigor, web-scraping-automation, nix-hyprland-desktop, google-apps-script.
- [x] Six job text drafts: ship-rust-tool, ship-desktop-app, ship-client-website, build-llm-rag-feature, deliver-data-analysis, build-automation. Reuse website-launch-checklist rather than copying its content into the website job.
- [x] Each skill has precise triggers, scope, actionable decisions, applicable completion criteria, sources and a research date. Source review and behavioral status are recorded separately.
- [x] Performance criteria specify workload, correctness checks, environment, baseline, repetitions, uncertainty and evidence. Local policies are distinguished from external standards. Unsupported universal floors were removed.
- [x] Material factual clusters were checked against primary sources; independent coverage, corrections, parent gap closure and scope limitations are recorded in SOURCE-REVIEW.md. No site/strategy certification or full mathematical-paper audit is implied.
- [x] Exported helpers run outside the project without governor state; all card references resolve within the bundle or are explicitly declared external prerequisites. Standalone review pilot completed; UI prerequisites remain conditional and are never implicitly installed.
- [x] The comparator and pack builder address repeated deterministic work, declare dependencies/side effects and pass seventeen behavioral tests. Other proposed helpers remain candidates rather than unnecessary code.
- [x] Instruction/script/export review and static scans completed for the capability layer. Five tool-scope leads remain documented; no runtime sandbox guarantee or automatic installation is inferred.
- [x] Three paired project-review tasks record matched input hashes, model/effort, quality scores, original answers, total/cached/output tokens and elapsed time. API cost remains unknown. This is a nonblind development pilot, not general superiority evidence or implementation acceptance across every skill.
- [x] Behavioral claims remain experimental. No automatic install or unsupported performance claim.

### B. Framework foundation

- [x] An existing comparator change moved through objective, frozen plan, fixture editor, real registered Python tests and delivery. A new regression failed on the original source, then ten helper tests passed on the candidate. Canonical promotion checked source equality and durable acceptance. This demonstrates code/workflow correctness, not model effectiveness.
- [x] Authoritative state records task IDs, dependencies, attempts, ownership, decisions, acceptance evidence and timestamps. Retrieval indexes are not authoritative workflow state.
- [x] Governor dispatches a bounded brief with objective, constraints, context/evidence hashes, selected capability paths, ownership, permitted actions, criteria, stopping condition and task/attempt/runtime identity.
- [x] Specialist reports contain artifacts, unresolved issues, usage provenance and candidate memories; they cannot set accepted status. Code checks actual artifact hashes.
- [x] Typed artifact checks, a host-registered named repository verifier and separate review decisions are implemented. Names are trusted host contracts, not authenticated identities. Independent code review and parent corrections are recorded in CORE-REVIEW.md.
- [x] Memory admission records observation/fact/decision classification, provenance, governor rationale and supersession while keeping execution history separate. This is a local project store, not Footnote integration or semantic truth certification.
- [x] Interruption/reopen/report reconciliation is tested without re-executing completed effects, including an actual child-process exit. Unknown outcomes remain visible and are not automatically retried.
- [x] Token-layer contract is required and bound to each attempt/brief/runtime, with wrong-mode/stale/missing activation refused. Actual/unknown/cached usage and prediction deltas are preserved. Real Footnote connection is deferred, not represented as implemented.
- [x] Standalone capability exports and helper work without orchestration imports or governor state.
- [x] Forty-five governor/verifier tests cover launch, report, verification, duplicate/retry/corruption/recovery boundaries, registered-suite completion and actual process exits in both execution and verification phases.
- [x] Labelled summary-conversion fixture completes through review and memory admission with one normalization effect. The separate real-code slice adds repository-suite acceptance; live integration remains deferred.

### C. Handover

- [x] RUNBOOK.md and HANDOVER.md provide source, repeatable commands, 62 tests, security-review scope and benchmark receipts. Exports are experimental and uninstalled.
- [x] HANDOVER.md maps Footnote/RingMaster ownership, adapter contracts and deferred work without creating duplicate runtime/usage/memory systems.
- [x] HANDOVER.md maps criteria to evidence and explicitly records deferrals. The standalone foundation is complete within this contract; real predictor/Footnote integration is deferred by the user, not claimed implemented.

## Sequence

### D. Integrated Footnote Flow, required for product acceptance

The completed A-C criteria describe the standalone toolkit/reference foundation, not this product stage.

- [ ] Reuse the current Footnote composition boundary and existing Memory, Library, Code, Usage and runtime responsibilities; no duplicate engine, memory or accounting system.
- [ ] Prepare bounded project/version/freshness-aware context with provenance, deduplication, expansion and explicit unavailable/disabled products.
- [ ] Complete a real repository task through one working agent by default, trusted verification and runtime-bound recovery; justify any additional model agent.
- [ ] Carry a reviewed evidence-bound lesson to a related fresh session through existing Memory policy without duplicate hook writes or stale/foreign-project leakage.
- [ ] Attribute complete run/agent usage and host phase timing, preserving unknown values and the measurement-only prediction exception.
- [ ] Evaluate integrated task sequences with equivalent Footnote stacks, frozen external checks and explicit practical margins. Demonstrate better output or preserved quality with less work before claiming practical benefit.

## Original foundation sequence

1. Establish this contract, inventory existing assets and identify token-layer ownership.
2. Research and complete text packs; review claims and routing.
3. Implement only justified helpers; audit and evaluate the toolkit.
4. Build the smallest complete orchestration slice, reusing existing kernel capabilities where verified.
5. Exercise recovery and mandatory token integration; prepare the Footnote handover.

## Initial evidence and open issues

- Existing drafts: python-performance, react-app-performance and tauri-desktop-app. They require review; their current curated labels are not evidence of an independent pass.
- Shared authoring standard exists. Its hard module-count limits overgeneralize SkillsBench observations; revise them into measured design heuristics.
- Footnote ADR 0001 establishes a shared Rust engine, one-way product dependencies and CLI-testable logic. Respect these boundaries in the handover.
- RingMaster README describes an existing task pipeline, budget ledger, acceptance predicates and curated memory gateway. These are documentation claims, not verified implementation findings; inspect before reusing or reproducing them.
- Footnote backlog records a usage-accounting module, including local session token counts. Accounting is not established as the requested prediction layer. Exact prediction product/API remains unidentified.
- Preserve unrelated vault edits, especially the style guide, existing skills README and Sentinel log.

## Sources

- User decisions in this conversation, 17 November 2026.
- [Footnote repository ADR](D:/Projets/BorgKnowledge/docs/adr/0001-repository-structure.md).
- [Footnote roadmap](D:/Projets/BorgKnowledge/docs/roadmap/BACKLOG.md).
- [RingMaster overview](D:/Projets/RingMaster/README.md).
- [Agent Skills specification](https://agentskills.io/specification).
- [SkillsBench](https://arxiv.org/abs/2602.12670).
