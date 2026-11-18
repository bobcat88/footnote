---
name: google-apps-script
description: Improve Apps Script throughput and reliability using batched service
  calls, quota-aware execution and resumable operations. Use for Sheets/Drive scripts,
  triggers and GAS web apps.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: cf146b801762cd3a80035ca88b5d4c2afe616667dd2fc2885950dec67bb89f4d
  source-card: AI-Agents/skills/google-apps-script/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Google Apps Script



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Minimize remote work

- Identify account class, execution type, required scopes and the specific service quotas. Recheck official quotas before sizing a job; they can change and differ by account.
- Read a range once, transform in memory, and write a range once where feasible. Avoid alternating individual reads and writes.
- Reduce repeated service calls; cache only data that can tolerate staleness. A cache miss must remain correct.
- Chunk jobs with durable cursors and stable record IDs. Stop before the applicable execution limit and resume through an appropriate trigger only when authorized.
- Use locking for conflicting updates and make retries idempotent. Lock acquisition timeouts, execution termination, trigger overlap and partial writes require explicit handling. For Sheets, flush pending changes before releasing exclusive access when the protected operation needs them committed.
- Keep authorization and business logic separate. Do not add broad scopes, expose deployment endpoints or run user-owned triggers as a routine optimization.
- For web apps, validate server inputs and treat client-supplied identity and document IDs as untrusted.

## Measurements and checks

Record rows processed, service calls, elapsed time, failures, retries and duplicate writes per execution. Measure cold and warm cache paths and the actual deployed account. Set batch size from observed runtime and service limits rather than a universal row count.

Test empty ranges, schema changes, overlapping triggers and interruption after partial progress. Preserve outputs and formatting required by the task. A local JavaScript unit test cannot establish live Apps Script quota or authorization behavior.

## Candidate helper for round 2

Validate pure transformation functions and checkpoint transitions offline. Separately instrument service-call counts in approved deployments; do not install triggers or access production Sheets during fixture tests.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://developers.google.com/apps-script/guides/support/best-practices
- https://developers.google.com/apps-script/guides/services/quotas
- https://developers.google.com/apps-script/reference/lock/lock
- https://developers.google.com/apps-script/guides/web

