---
type: decision
tags:
  - domain/ai/agents
  - domain/dev/architecture
  - type/reference
  - project/borg
  - status/active
aliases:
  - Footnote Flow Architecture
date_maj: 2026-11-17
---
# Footnote Flow Architecture

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Footnote Framework]] · Related: [[AI-Agents/projects/footnote-framework/CAPABILITIES|Capability toolkit]]

#domain/ai/agents #domain/dev/architecture #type/reference #project/borg #status/active

## Status

The current product design is [[AI-Agents/projects/footnote-framework/FOOTNOTE-FLOW-DESIGN|Footnote Flow integration]]. This document describes the tested standalone reference and its boundaries. Johan clarified that the next product must extend Footnote's existing components and improve output; a separate model coordinator is optional, not the default. Product integration is now an open required stage; the actual prediction product remains deferred.

Implementation contract for the standalone foundation. The reference governor implements transactional project state, bounded dispatch/reporting, typed and host-registered repository verification, review, memory admission and interruption recovery. A real parser change in an isolated copy passed its registered Python suite after failing the baseline regression. Review and regressions are recorded in CORE-REVIEW.md. Footnote and its token layer will be connected later, per the user's steering; fixture activation never establishes real integration.

## Two layers

The capability layer consists of standard skill cards, conditional references and independent measurement helpers. Seven thematic distributions reuse the same source cards; physical bundling does not mean loading all content. There are no governor imports, workflow databases or implicit model launches in exported capability code.

The orchestration layer tracks the objective, frozen task contracts, dependencies, attempts, evidence, acceptance and memory admission. It selects capabilities and delegates when a bounded independent task justifies the added context and coordination. It does not turn every skill link into another agent.

The three-task development pilot showed that fresh children still carry substantial shared harness context. Therefore dispatch decisions must account for full harness/tool overhead and actual outcomes, not just the length of the task brief. Token and elapsed-time receipts are available; they do not establish a general efficiency winner.

## Reuse boundary with RingMaster

RingMaster is not in the current graph inventory. These are bounded source observations, not a full architecture or correctness audit:

| Existing source | Observed responsibility | Foundation decision |
|---|---|---|
| `rm-core/src/contract.rs` | TaskContract, Budget and delegation limits | Do not create another money/lease system in the project coordinator |
| `rm-core/src/state.rs` | Turn lifecycle and replay transition validation | Distinguish project-task states from executor-turn states; preserve turn IDs as external references |
| `rm-kernel/src/kernel.rs` | KernelConfig, run/run_swarm, reconciliation and settlement entry points | Keep actual executor launch, budget reservation and OS containment behind the runtime boundary |
| `rm-harness/src/skills.rs` | Skill metadata/body hashing, registry and signature checks | Standard pack exports are not automatically RingMaster-registered or approved; do not sign them implicitly |
| `rm-core/src/governor.rs` | Tier/engine routing and RoutePlan | Project-level governor decides work and acceptance; runtime routing remains a separate responsibility |

The current skill ID source hashes name, description and body. That observation does not prove coverage of helper/reference files. A future bridge must verify the exported manifest and transitive file hashes, and bind them to the runtime's approval/signature mechanism. A copied SKILL.md alone is not a trusted package.

No RingMaster source has been changed or its full test suite run here. The prototype will define the project-level contract in a small standard-library implementation; it is a handover reference, not an additional production runtime commitment. Footnote's Rust product boundary remains the eventual integration decision.

## Authoritative project state

Use a local transactional store for the foundation. It holds frozen project/task contracts, task dependencies and ownership, immutable attempt identities, reports, verification evidence, review decisions and memory admission. Append audit events in the same transaction as state changes. Keep retrieval/indexing outside authority over execution state.

Validate the plan before saving: unique IDs, nonempty objectives/acceptance, valid owned paths, supported criteria, bounded attempts and an acyclic dependency graph. Once execution starts, a changed plan requires an explicit new revision rather than silently rewriting a running task.

The coordinator's database must be outside worker-owned paths. This structural check is not an OS sandbox; real launch adapters must use the runtime's containment/lease mechanism. Unknown schema versions and corrupt state must fail visibly rather than initialize over an existing database.

Declared context files are immutable receipts. A task that transforms a file must use a separate immutable input snapshot and an owned output path. Context/pack/dependency hashes are rechecked after token activation before launch; context and transitive artifact evidence are checked again during verification, completion and memory admission. These are point-in-time checks in a trusted workspace, not filesystem locking against hostile external writers.

## Dispatch and handoff

The minimum sufficient brief contains objective, constraints, context/evidence references, selected skill paths and hashes, ownership, acceptance criteria, permitted actions, attempt ID and stopping condition. It omits unrelated conversation. It may not omit constraints required to do the work correctly.

Before every real specialist launch, invoke the connected token layer for that exact attempt and brief. Bind its activation receipt to task/attempt/brief identity and record adapter/version, mode and predictions where available. A missing, failed, stale or fixture activation cannot authorize real execution. No command-line flag may simply assert that the layer is active.

Fixture execution uses explicitly labelled fixture prediction and executor implementations. It must never produce a live-integration acceptance claim. Authoritative runtime usage, when available, is reconciled with predictions; unknown values stay null, and cached tokens are not counted again as additional input.

Save dispatch intent before invoking the executor. An interruption after dispatch makes the outcome uncertain; reopening the project must not automatically re-run it. The runtime adapter should reconcile a known live/terminal handle. Without one, require explicit evidence-based reconciliation.

## Reports and acceptance

Reports return task/attempt identity, outcome summary, artifacts, verification evidence, unresolved issues, usage and candidate memories. They cannot contain an authoritative accepted flag. The coordinator verifies actual artifacts against the task's criteria and recorded hashes.

Support a small set of typed deterministic checks for the first slice, with a named verifier boundary for repository-specific checks. Do not execute arbitrary shell commands embedded in an agent report or retrieved document. Validate paths and preserve the distinction between a fixture predicate and a real repository test.

Named verifier IDs, versions and configuration fingerprints come from trusted host registration and are frozen in the task. Persist intent before invoking the callback outside database transactions. The Python adapter requires a launcher-completion receipt with nonzero tests run and a successful result, as well as exit zero. Completion is bound to the request; artifact/context/dependency/pack hashes are rechecked before acceptance. It executes controlled repository code, not a hostile-code sandbox.

Where judgment is required, require a recorded review decision and evidence with a reviewer distinct from the executor. Caller-supplied names alone do not establish authenticated independence; the future harness bridge must supply trustworthy principals. Failed checks trigger an explicit bounded revision or escalation. Never loop until a model says success.

## Recovery and memory admission

Task states must distinguish running, reported, verified/awaiting review, accepted, revision, escalated and uncertain outcomes. Duplicate reports/acceptance must not create duplicate side effects. Changed evidence must be detected before dependent work or memory promotion relies on it.

Named verification adds verifying and verification_uncertain phases. Recovery never reruns suites automatically or accepts replacement worker reports to clear verifier uncertainty. A trusted host observes a terminal outcome, marks verification uncertain if needed, then reconciles the original request-bound receipt. Missing reliable evidence stays unresolved. Known version-1 local state upgrades transactionally to schema 2 by adding verifier records; unknown schemas remain refused.

Keep execution history separate from durable knowledge. Specialists propose memories; the governor chooses whether they are useful and classifies observations, verified facts and decisions. Admission records source attempt, accepted evidence, actor and any superseded memory. Supersession preserves history. The prototype writes only its own project store; it does not mutate Footnote or the user's Codex memory folder.

## Acceptance slice

Demonstrate a small change in a disposable existing-project fixture: define objective/criteria, plan dependent tasks, dispatch a specialist with selected capabilities, verify its artifacts, record a review where required, admit one justified memory and produce a handover. Interrupt and reopen the run between dispatch and report; reconcile without re-executing the completed artifact-producing action.

Tests must additionally reject invalid/cyclic plans, missing token activation, fixture/real mode mismatch, self-approval, outside-scope artifacts, stale evidence, duplicate completion, exceeded attempts, unknown/corrupt state and unsafe automatic retry after uncertainty. No real project, model endpoint or integration is certified by fixture tests.

## Sources

- [RingMaster contract](D:/Projets/RingMaster/crates/rm-core/src/contract.rs), [turn state](D:/Projets/RingMaster/crates/rm-core/src/state.rs), [kernel](D:/Projets/RingMaster/crates/rm-kernel/src/kernel.rs), [skill registry](D:/Projets/RingMaster/crates/rm-harness/src/skills.rs), [routing](D:/Projets/RingMaster/crates/rm-core/src/governor.rs): bounded reads on 17 November 2026.
- [Footnote product structure ADR](D:/Projets/BorgKnowledge/docs/adr/0001-repository-structure.md).
- [Pilot usage receipts](evidence/pilot-runtime-usage.json) and [project acceptance contract](PROJECT.md).
