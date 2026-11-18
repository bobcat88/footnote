---
type: decision
tags:
  - domain/ai/agents
  - domain/dev/architecture
  - type/reference
  - project/borg
  - status/active
aliases:
  - Footnote Flow Integrated Design
  - Flow Product Direction
date_maj: 2026-11-17
---
# Footnote Flow: the integration design

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Flow]] · Related: [[AI-Agents/projects/footnote-framework/ARCHITECTURE|Reference implementation]] · [[AI-Agents/projects/footnote-framework/FULL-EVALUATION|Evaluation]]

#domain/ai/agents #domain/dev/architecture #type/reference #project/borg #status/active

## Decision and scope

Johan's direction: Flow extends the Footnote family. Curated skills are an independently useful part; the next product must make the family's components work together to improve agent output. A standalone coordinator that cannot demonstrate practical value beyond those skills is insufficient.

The product direction is decided. The integration mechanisms below are a source-grounded proposal, not shipped functionality. Earlier standalone-foundation acceptance is reference-code acceptance only; it does not establish Footnote Flow product completion. Keep the existing tests and failed/limited evaluation evidence. Do not port the expensive coordinator pattern simply because it already exists.

The initial runtime path is one working agent with deterministic support. A governor is an authority over task state and evidence, not necessarily a separate model. An optional specialist must have a concrete purpose, isolated ownership, sufficient context and a bounded stopping condition. Use a model to resolve genuine ambiguity or exercise independent judgment, not to relay tool results and change statuses.

## What the source already gives us

Inspected working files on 17 November 2026, using the current graph and exact source. The repository advanced during the inspection; fingerprints in evidence/footnote-integration-design-inputs.json identify the read inputs rather than asserting one commit-pinned deployed build.

| Existing component | Observed capability | Integration consequence |
|---|---|---|
| Core/search and composition root | ADR 0001 specifies one shared engine, one-way dependencies, CLI-testable logic and independently enabled products | Implement cross-product coordination at the binary/application boundary; do not make products depend on each other or introduce another engine |
| Code | lookup returns definitions, callers, callees, project root and stale file information | Retrieve structure when the task needs it; read changed or uncertain source before relying on a relationship |
| Code plus related context | app::code_lookup calls find::related for Memory and Library context when symbols exist | Extend this existing route rather than add a second cross-product search service |
| Memory | Project-scoped recall, records, tags and source-observed code anchors | Reuse memory storage and retrieval; bind a lesson to the project and evidence that justify it |
| Library | Registered document corpora and bounded related passages | Select permitted project/version corpora, then expand passages only as needed |
| Usage | Session-file reader, counts, model/day/agent breakdowns, limits and measured-savings fields | Reuse this implementation; extend run/attempt/response attribution where necessary instead of shipping the benchmark Python reader as another production accounting system |
| Runtime and product switches | Shared daemon, hooks and independently enabled products | Detect capabilities and respect disabled products. Missing optional context must remain explicit without creating duplicate startup paths |
| RingMaster boundary | Prior bounded inspection found launch/routing/budget/lease ownership | Reuse only after refreshing its current interface; project coordination does not own another money or OS-containment kernel |

The Code parser/resolution base is a documented next direction, not established by these source reads as shipped. The inspected lookup still reports compiler origin. Code anchors were present in the advancing working tree; recheck release state before depending on them. Memory MCP health succeeded, but that establishes connectivity only, not that every new product feature is present in the connected server.

The current related-context path is useful groundwork. It is not yet the complete Flow contract: snippets alone are insufficient for project identity, version, freshness and acceptance authority. Before feeding them to an executor, bind or validate that metadata. Same-named symbols in another project must not be treated as relevant simply because they match by text or an unqualified anchor.

## The useful loop

1. **Understand the requested outcome.** Freeze the actual objective, allowed project, acceptance checks and stopping conditions. Reuse explicit user decisions; request clarification only for consequential ambiguity. Small tasks do not require a model-generated project plan.
2. **Resolve the task's evidence.** Start with supplied files/symbols. Use Code for structural questions, Memory for prior decisions and failures, Library for applicable API documentation. A direct file edit need not trigger all three. Detect disabled/missing/stale sources first.
3. **Assemble a bounded working context.** Select relevant evidence and skills, deduplicate repeated hook/tool results, preserve source references, and keep full bodies available on demand. Return nothing when nothing is relevant. Initially use transparent scope/version/freshness rules; do not claim an uncalibrated relevance threshold is reliable.
4. **Let the working agent act.** It receives the objective, evidence references, selected guidance and ownership. It can ask for deeper source or documentation. Flow records why context was provided and which capability supplied it. An exact count is preferred; unknown token counts remain unknown.
5. **Check the actual change.** Refresh changed-file/relationship evidence, run trusted repository checks, compare against requirements and record remaining uncertainty. Use an independent model or human critic when judgment is required. Passing tests does not certify an architectural claim or global semantic truth.
6. **Offer a useful learning.** Tie a proposed observation/decision to the accepted change and its file/symbol/version evidence. Admission follows the existing Memory authorization/review policy. Do not store entire transcripts, every successful test, credentials or generated speculation. Avoid duplicate writes from Flow and existing memory hooks.
7. **Measure and adapt.** Join runtime usage and phase timings to the run/attempt. Evaluate whether context improved acceptance, prevented a repeated mistake or reduced repair work. Change routing policy only from comparable evidence; do not silently rewrite skills or trust a model's self-reported savings.

The practical distinction is that accepted work produces evidence the next related task can use. Retrieval alone cannot decide whether the evidence still applies; orchestration alone cannot supply it. Flow connects those responsibilities using the existing products.

## Context and state contracts

Proposed context receipt fields: run/task/attempt identity, canonical project identity, source-product capability/version, source ID and content hash, scope/classification, documentation version where applicable, freshness, relationship origin/grade when available, reason for inclusion, body/reference size, and whether it was already delivered. A truncated passage must be labelled and retain an expansion reference. Model text cannot upgrade a source's trust status.

Budget allocation includes skill bodies, remembered decisions, code structure and documentation, not only the task prompt. Start with a configured per-task bound and expose exclusions; calibrate it on task outcomes rather than declaring an optimal universal token budget. Prefer exact file/symbol context to a project-wide dump. Do not eagerly read every selected reference or repeat the full pack each turn. Cross-product links are candidates, not instructions or automatic authorization.

Execution state records the current objective, attempts, dependencies if any, external runtime handle, selected-context manifest, artifact hashes, verification and review outcomes. This remains authoritative even if search indexes are rebuilt. Memory stores admitted knowledge; Library stores documentation; Code stores structural indexes. References connect them. A retrieval result cannot mark an attempt accepted.

Use Footnote's application composition layer for orchestration, with core-owned typed state and the shared migration conventions chosen in an implementation ADR. Do not bury workflow authority inside a disposable retrieval index. The Python governor database remains a reference implementation until this ownership is explicitly implemented. No empty new crate or duplicate memory store is required now.

Runtime usage and execution authority are separate. Footnote Usage supplies accounting; the host must supply reliable child/attempt handles and call attribution. The user-authorized measurement-only adapter remains evaluation-only, with no predictions. The actual prediction product is still unidentified; neither Usage nor its limits should be relabelled as prediction.

## Failure and escalation policy

- No relevant memory or missing Library corpus: continue with available evidence and state the gap. Missing evidence required for correctness is a reason to investigate or stop, not fabricate an answer.
- Stale graph/source: read the actual file, recheck affected evidence, and request reindexing through the product's existing path if needed. A graph miss is not proof that no caller exists.
- Conflicting current source, docs and memory: keep their provenance, identify the conflict and investigate. A remembered decision is not automatically stronger than current requirements or source.
- Interrupted executor: reconcile its runtime handle and existing artifacts before repeating work. Unknown outcomes remain visible. A new index generation cannot erase task history.
- Failed verification: bounded repair within the same task contract. Escalate when the contract, ownership, confidence or attempt cap requires it. Do not add agents solely to make a failed result look reviewed.
- Scope/privacy mismatch: exclude the item, even if useful-looking. Respect product switches, corpus allowlists and existing personal-store protections. Traces record identifiers/counts by default rather than private text.
- Specialist launch: record the reason, expected benefit, selected context, ownership, budget and stopping condition. Initially an explicit policy choice, not a purported proven predictive classifier.

## First vertical slice: one repository maintenance task, two sessions

An initial mechanics proof now exercises the real products across two fresh agent sessions in an isolated controlled fixture; see [[AI-Agents/projects/footnote-framework/INTEGRATED-LOOP|Working integration]]. It establishes interoperability and scope/freshness checks, not the independent real-repository usefulness criterion below.

Build the first slice around an existing-code change whose correctness depends on a documented constraint and a previous project decision. Use a fresh isolated copy, real Footnote products and one working agent. Do not reuse the known byte-size parser calibration as confirmation.

**Session one:** locate a target through Code; retrieve its permitted Memory/Library context; deliver a compact evidence pack plus the relevant skill; implement; verify through the real repository suite; propose one useful evidence-bound lesson. Record every failure, context expansion and optional component unavailable.

**Session two:** start a genuinely fresh agent on a related follow-up. Retrieve the admitted lesson together with current code/docs, avoiding manual retelling. Deliberately include a stale source or wrong-version decoy in a separate controlled test, and confirm it is not relied upon. An interruption/resume case must reconcile existing effects without duplicate execution.

Minimum acceptance before claiming the integrated slice works:

- Code, Memory and Library references identify their permitted project/corpus and actual source version/hash; context is bounded, deduplicated and expandable.
- The working agent can correctly use the relevant evidence; held-out checks detect violation of the previous decision or documented API constraint.
- Disabled products, missing docs, stale anchors and same-named foreign-project symbols are handled explicitly. No private or cross-project data is silently promoted into the task.
- External acceptance checks inspect the changed code, not a report saying tests passed. Relevant quality judgments remain independent.
- The useful lesson reaches the next fresh task only under the existing authorized admission policy, with provenance; rejected/stale candidates are not silently made authoritative.
- Footnote Usage and runtime receipts reconcile coordinator/worker/retry attribution; unknown counts/costs remain null. Host timing spans include prepare, ready, context, execution, checks and terminal state. TTFT is reported only if the host exposes it.
- No extra model coordinator runs in the default path. Optional specialists and review cost are included where used.

## Evaluation and release decisions

Do not run the old 54-run parser-oriented study as a release gate for the integrated product. Preserve its protocol and calibration as historical evidence of the reference path's overhead. First establish the new slice's actual integration and measurement, then freeze independent task sequences.

Primary three-arm study, with the same Footnote capabilities and initial corpus snapshot in every arm: A Footnote with the normal agent, B the same plus curated skills, C the same plus skills and Flow. This estimates the added value of skills and Flow without crediting them for unequal infrastructure. A separate Footnote-disabled control measures the family's contribution. End-to-end competitor comparisons must disclose their tools, recommended small/full workflow, initialization cost and retained knowledge.

For learning tasks, each arm gets an independent identical initial database copy. It may accumulate its own authorized knowledge across its own sequence; never share learned outputs between treatments or reset memory between steps when measuring continuity. Freeze document versions, code snapshots, harness rules, caches where controllable, provider/model/effort and common external acceptance tests. Include single-edit tasks where Flow should add little overhead and multi-session tasks where the proposed integration should help. This tests a hypothesis, not an entitlement to claim a benefit.

Primary decisions use accepted outcomes, requirements coverage, repeated mistakes, repair rounds, human intervention and complete time/resource consumption. Count security/privacy and recovery correctness separately from speed. Report retrieval relevance, useful/unused context, cache, tool calls and daemon costs as diagnostics. An ablation can identify which component contributes; do not double-count memory, skills and routing savings against the same baseline.

Ship the integration only if it either improves output at an acceptable total cost or preserves quality while reducing work on the targeted workloads. Freeze practical margins before the confirmation runs. There is no justified numerical threshold yet. A component can be useful while its combination is inefficient; both statements must remain possible in the results.

## Implementation order

1. Refresh the shipped CLI/MCP capability inventory, product switches and project/corpus identity rules. Close metadata/scope gaps in the existing related-context route before using it automatically.
2. Add the deterministic prepare/context-receipt boundary at Footnote's composition root. Reuse Code/Memory/Library and Footnote Usage; do not install another framework or daemon.
3. Connect one real agent/runtime handle to that receipt and the existing verified acceptance/recovery contracts. Implement same-host phase timing with the actual host, not reconstructed guessed timestamps.
4. Connect reviewed evidence-bound learning to the existing Memory path, avoiding duplicate hook writes. Validate the second fresh session.
5. Run the integrated three-arm task sequences. Add optional model planning/delegation only if their measured contribution justifies them.

Proposed CLI/MCP names, storage schema, budgets and routing thresholds are deliberately not presented as existing APIs. This design changes the product target; it does not modify the advancing Footnote source tree or claim the integration is built.

## Evidence and sources

- [Repository layering ADR](D:/Projets/BorgKnowledge/docs/adr/0001-repository-structure.md).
- [Code lookup composition](D:/Projets/BorgKnowledge/src/app.rs), [related context](D:/Projets/BorgKnowledge/src/find.rs), [Memory storage/anchors](D:/Projets/BorgKnowledge/crates/memory/src/lib.rs), [Code lookup](D:/Projets/BorgKnowledge/crates/code/src/lib.rs), [Usage](D:/Projets/BorgKnowledge/src/usage.rs).
- [Current Code design decisions](D:/Projets/BorgKnowledge/docs/CODE-GRAPH-DESIGN.md), including roadmap items distinguished from implemented source.
- evidence/footnote-integration-design-inputs.json: source fingerprints, MCP health and inspection limits. Graph generation 2026-11-17T18:15:19Z; cited code paths reported metadata_match/no recorded issue, best effort only.
- [[AI-Agents/projects/footnote-framework/LIVE-CALIBRATION|Live calibration]] and [[AI-Agents/projects/footnote-framework/CORE-REVIEW|Reference-code review]]. No product source edits, live memory writes or new benchmark model runs in this design pass.
