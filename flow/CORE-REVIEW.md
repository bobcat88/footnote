---
type: reference
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Footnote Governor Review
date_maj: 2026-11-17
---
# Footnote Governor Review

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Footnote Framework]] · Related: [[AI-Agents/projects/footnote-framework/RUNBOOK|Runbook]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## Independent pass

A separate reviewer read architecture/runbook and contracts, store, governor, fixture adapters, demo, benchmark and tests. It ran 24 governor tests successfully in 1.525 seconds. It did not run live model services, change sources, or claim authentication/OS containment/production integration. Existing demo and benchmark artifacts were read but not independently rerun.

| Finding | Severity | Parent correction and regression evidence |
|---|---|---|
| Input context was hashed but not checked after dispatch; post-activation pack/dependency freshness was not revalidated | Medium | Declared context is now explicitly immutable; owned-input transformation requires a separate snapshot. Launch inputs are checked after activation; context participates in later freshness checks. Tests cover context changes during activation, after report, after acceptance, and pack/dependency changes during activation |
| Existing zero-byte database could be initialized as new | Medium | File existence is distinguished from size. Existing empty files are refused and their bytes remain unchanged in a regression test |
| Benchmark comparator did not enforce capability-pack identity | Low | Pack identity is included in the comparison workload ID. A behavioral regression builds two different packs and confirms the comparator refuses to compare their receipts |

The parent added regressions for uncertainty during activation and fixture counts labelled as runtime usage. The second independent pass inspected the three earlier corrections and their tests.

## Exercised contracts

- Frozen valid plans and acyclic dependencies, with task/attempt identity and bounded correction.
- Transactional reservation and audit events; competing dispatch produces one executor effect.
- Missing/stale/wrong-mode token activation refusal and explicit runtime/engine identity.
- Report self-approval refusal, typed checks against actual artifact hashes, independent-review labels and evidence.
- Immutable context and transitive dependency freshness before relying on results.
- Memory admission only after accepted fresh evidence and an explicit governor rationale; duplicate admission and supersession history.
- Known/unknown usage accounting, cached-input subset semantics and prediction deltas without cost invention.
- Reopening interrupted state without automatic re-execution, including a real child-process exit followed by report reconciliation.
- Unknown schema, invalid persisted task/report state and existing empty/corrupt files fail visibly.

## Scope limits

This is a standard-library foundation with trusted in-process host adapters. It is not a hostile-filesystem sandbox, an authenticated reviewer service, a tamper-proof ledger, a remote process supervisor or a power-loss certification. Real Footnote/token integration remains deferred by the user. There is no live model adapter in this prototype.

The second independent pass covered named verification, schema upgrade, recovery and the actual helper diff. Its initial 42-test run passed. It found one medium issue: an import exiting zero could certify a suite that never ran. The parent added a launcher-completion receipt requiring nonzero testsRun and successful completion. A regression exposed stale bytecode when equal-size test sources changed within one timestamp interval; the adapter now compiles registered test bytes directly and uses a fresh import-cache location. The final parent run passes 45 tests. The reviewer's concurrent final run saw the intermediate failing regression; that failure was corrected before current evidence. These last corrections have not received a separate independent pass.

The original comparator accepted repeated JSON keys with last-key-wins interpretation. A local synthetic receipt reproduced ambiguity in correctness metadata; this is a parser strictness improvement, not a demonstrated production vulnerability. The isolated real-code slice rejects repeated keys at every object depth. Its new regression failed on original source, then ten helper tests passed through the registered verifier. The reviewer inspected matching hashes, the patch and stored test receipt. After the final verifier corrections, the workflow was repeated successfully and its accepted candidate promoted after baseline-equality and persisted-acceptance checks.

## Evidence

- [Current governor test log](evidence/governor-tests.txt).
- [Current fixture benchmark](evidence/governor-benchmark-v0.2.1.json), including source fingerprints, workload/pack identity and explicit exclusions.
- `build/workflow-demo-v0.2.1/result.json`: labelled fixture completion, one normalization effect, review and local memory admission.
- [Comparator ambiguity baseline](evidence/comparator-duplicate-keys-baseline.json).
- [Accepted real-code change and suite receipts](evidence/code-change-acceptance.json); frozen originals remain in evidence/code-change-inputs.
- [[AI-Agents/projects/footnote-framework/PROJECT|Full acceptance contract]] and [[AI-Agents/projects/footnote-framework/ARCHITECTURE|implementation boundaries]].
