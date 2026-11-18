---
type: reference
tags:
  - domain/ai/agents
  - domain/dev/workflow
  - type/reference
  - project/borg
  - status/active
aliases:
  - Footnote Flow Runbook
date_maj: 2026-11-17
---
# Footnote Flow Runbook

Up: [[AI-Agents/projects/footnote-framework/PROJECT|Footnote Framework]] · Related: [[AI-Agents/projects/footnote-framework/ARCHITECTURE|Architecture]]

#domain/ai/agents #domain/dev/workflow #type/reference #project/borg #status/active

## What runs today

A standard-library reference implementation coordinates a frozen project plan through dependent specialist attempts, reports, deterministic verification, optional review, local memory admission and recovery. It uses a project SQLite database and audit events; it does not replace RingMaster's executor, money or OS lease kernel.

Python 3.12 is the verified environment. The core has no third-party dependencies. Building capability exports separately uses the pinned PyYAML dependency. No skills are installed and no external model is called by the fixture adapters.

From this project directory:

```powershell
rtk proxy python -m unittest orchestration.test_governor -v
rtk proxy python -m unittest orchestration.test_verifiers -v
rtk proxy python -m orchestration.demo build/choose-a-new-demo-directory --skills-root ../skills/skills
rtk proxy python -m orchestration.code_change build/choose-a-new-change-directory --skills-root ../skills/skills
rtk proxy python -m orchestration.benchmark --skills-root ../skills/skills --runs 10 --warmups 2 --output build/choose-a-new-benchmark.json
```

Use a new demo destination and benchmark output. The demo changes a disposable RingMaster latency-summary fixture, not the real repository. Values remain labelled summary statistics rather than raw samples. It writes an artifact, deliberately interrupts, reopens durable state, reconciles the lost report, verifies the conversion, checks a dependent handover, records a labelled fixture review and admits one scoped observation. The normalizer must run once. `result.json` records fixture mode and `real_integration: false`.

The test suite also terminates an actual child process after artifact production and proves the persisted attempt can be reconciled without another executor effect. This is process-interruption evidence, not a power-loss or real remote-agent recovery certification.

## Core API

```python
from orchestration import Governor

governor = Governor(database_path, existing_workspace, exported_skills_directory)
try:
    governor.plan(project_contract)
    attempt_id = governor.dispatch(task_id, trusted_executor, trusted_token_layer)
    governor.verify(task_id)
    state = governor.snapshot()
finally:
    governor.close()
```

The database must be outside the worker workspace. A plan includes schema version, goal, optional frozen decisions and tasks. Each task declares identity, objective, owner, owned paths, criteria, optional capabilities/context/dependencies, constraints, permitted actions, stopping condition, attempt limit and review requirement. `orchestration/demo.py` contains a complete example.

Plans are validated and frozen. Unknown tasks, cycles, outside-scope criteria and arbitrary command verifiers are refused. Capability manifests and every listed file hash are checked; the manifest identity is frozen with the project. A changed pack requires a new explicit project revision. The source/cards can remain usable without importing any orchestration module.

The basic verifier types are `contains` and `json_equals`, comparing actual bounded artifacts with criteria and report-bound hashes. JSON equality preserves types and rejects duplicate keys/non-finite values. A task can also declare `verifier: {id, version, sha256}` matching a trusted host object registered through `Governor(..., verifiers={id: object})`. It exposes its descriptor and `run(request)` callback. Missing or changed registration refuses verification. Reports cannot register code or select commands.

`orchestration.repository_tests.PythonUnittest` registers one configured suite and its hash. It launches current Python through RTK in isolated mode, compiles test bytes directly, uses a fresh import-cache location, and requires nonzero tests run plus a successful launcher-completion receipt. It retains exit status and a bounded transcript. This executes trusted repository code in a disposable workspace. Disk/process-tree limits remain the runtime host's responsibility; this is not hostile-code containment. Timeout or invalid receipt leaves verification uncertain.

The code-change demo copies the frozen comparator and nine original tests from evidence/code-change-inputs, adds a duplicate-key regression, requires baseline failure for the expected reason, then dispatches a fixture editor. The governor runs ten real Python tests before acceptance and local memory admission. Only the disposable destination changes. The accepted patch was separately promoted to the canonical helper after baseline-equality and persisted-acceptance checks; promotion is not an automatic demo side effect. See evidence/code-change-acceptance.json.

## Runtime and token-layer boundary

For the three-arm evaluation only, Johan authorized a measurement-only adapter. See FULL-EVALUATION.md and evaluation/measurement_adapter.py. It requires the frozen decision string exported as DECISION and a runtime adapter name beginning flow-evaluation:. Its receipts explicitly disable predictions. Actual per-call counts must come from the trusted host collector; activation alone does not claim captured usage. The core production dispatch code was not changed and this adapter is not Footnote's predictor.

Executors declare mode (`fixture` or `real`), owner/actor, adapter identity/version and engine. A real adapter must provide version and engine explicitly. Its `execute(brief, activation)` returns the structured report only after its trusted runtime determines the execution outcome. No real runtime adapter is shipped here.

The brief is bounded to 64 KiB and contains the task contract, relevant project decisions, selected capability paths/hashes, pack identity, context file paths/hashes and accepted dependency evidence. It does not copy the entire conversation or all skill bodies. Artifact reads are bounded to 1 MiB; larger outputs should supply bounded receipts. These are local technical limits, not research-derived optimal budgets.

Context files are immutable. If a task needs to transform a file, stage a separate baseline snapshot under an input path and give the worker a distinct owned output path. The plan refuses context files inside that task's owned scope. Launch inputs are rechecked after token activation; later source changes invalidate verification/completion/memory freshness. Real runtime adapters must still enforce their read/write leases and check inputs when accessing them.

Before every launch, the token layer's `activate(brief)` must return active status, matching mode, task/attempt IDs, SHA-256 of the canonical brief, adapter/version/activation identity and nullable input/output predictions. Missing/stale/wrong-mode activation cannot authorize launch. Fixture activation cannot authorize real mode. There is no command-line flag that simply asserts activation.

Adapter objects are trusted host integrations. Strings such as mode, actor or adapter name are not authentication credentials or proof that Footnote is connected. The future bridge must supply authenticated principals, approved runtimes, terminal handles, actual containment and authoritative usage. It must also route any nested delegation through the governor rather than letting a worker create untracked children. The current canonical brief serialization is the Python contracts.encoded function, not a claim of RFC 8785/JCS compatibility; a Rust bridge needs shared serialization test vectors before it can reproduce activation hashes.

Reports declare task/attempt IDs, summary, artifact paths, unresolved issues, usage source/counts, candidate memories and optional runtime turn ID. They cannot set accepted status. Cached input is a subset of input, not extra usage. Unknown counts remain null. Available actual counts are reconciled with predictions; API cost remains null unless a future authoritative pricing/billing integration supplies it.

## Review and memory

`review(task_id, reviewer=..., approve=..., evidence_path=..., rationale=...)` requires verified fresh evidence and a reviewer distinct from the executor. The evidence file's hash is retained. Caller-supplied identities are a local contract check, not independent authentication. The fixture critic is explicitly simulated.

`admit_memory(task_id, candidate_index, actor=..., kind=..., rationale=..., supersedes=...)` requires an accepted task, fresh evidence, a governor actor distinct from the executor and an explicit admission rationale. Kind is observation, fact or decision. No report automatically promotes its proposals. Supersession preserves old records and prevents duplicate admission of the same candidate. This local store does not mutate Footnote, the vault notes or the user's Codex memory files.

## Recovery and operating limits

Reopening preserves running attempts; it never assumes they stopped and never automatically repeats them. First consult the runtime's live/terminal handle when one exists. `mark_uncertain` records an observation and blocks retry; it does not terminate a process or certify that it is dead. Reconcile a recovered report into the same attempt with `submit_report(..., actor=...)`, then verify actual artifacts. If outcome remains unknown, leave it unresolved rather than trying again.

The foundation intentionally has no automatic no-effect retry resolution, distributed scheduler, public RPC server or background daemon. Token activation failures and verification/review failures consume bounded attempts as documented by their events. A missing predictor before reservation does not consume an attempt.

Named suites reserve verifying state before callback execution. Interruption preserves verifying or verification_uncertain, blocking dispatch, worker-report replacement and another suite run. After observing a terminal host process, call `mark_verification_uncertain(verification_id, actor=..., reason=...)` if needed, then `reconcile_verification(verification_id, recovered_result, actor=..., rationale=...)`. This imports the original request-bound receipt without rerunning tests. Missing reliable evidence stays unresolved. A test exercises an actual host-process exit after saving a receipt. Version-1 state upgrades transactionally to schema 2; back up valuable databases before a version upgrade.

Active, reported, awaiting-review or uncertain tasks reserve their owned scopes against conflicting dispatch. Accepted artifact hashes and transitive dependency evidence are checked before dependent work or memory admission. Treat accepted evidence as immutable: deliberately changing an accepted artifact requires explicit project revision/revalidation, not silently calling an old receipt fresh.

SQLite transactions keep task state and audit events together and serialize competing claims. Reads use a consistent database transaction. This is not a hostile-filesystem sandbox or tamper-proof event ledger. Unknown database identity/schema and structurally invalid persisted task/report bindings fail visibly. Source/pack integrity and workspace boundary checks do not replace OS containment.

## Evidence scope

Fixture success establishes the local workflow contracts exercised by the code/tests. It does not establish real model effectiveness, authenticated independent review, a live predictor, Footnote integration, production financial accounting, UI acceptance or general performance at scale. These remain explicit integration/validation work for the handover.

## Sources

- `orchestration/contracts.py`, `store.py`, `governor.py`, `fixtures.py`, `demo.py`, `benchmark.py` and tests.
- [Python sqlite3 transaction control](https://docs.python.org/3/library/sqlite3.html#transaction-control).
- [[AI-Agents/projects/footnote-framework/PROJECT|Project acceptance contract]] and [[AI-Agents/projects/footnote-framework/ARCHITECTURE|integration boundaries]].
