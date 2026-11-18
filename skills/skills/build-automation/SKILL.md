---
name: build-automation
description: Deliver repeatable collection or workflow automation with idempotency,
  bounded execution and recovery evidence. Use for scraping, Python jobs or Apps Script
  automation.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: edbc8d7782132a9d9ce201c50ffebf5e4383ac16ec39208f3e37b244b6b9141f
  source-card: AI-Agents/skills/build-automation/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Build automation



## Route only the relevant work

This job procedure works in the current agent without a governor. It never creates subagents or installs dependencies implicitly. The following capability skills are candidates, not an instruction to load every one:

- [web-scraping-automation](../web-scraping-automation/SKILL.md)
- [python-performance](../python-performance/SKILL.md)
- [google-apps-script](../google-apps-script/SKILL.md)

If a required capability is unavailable, name the gap and use available project guidance; do not silently claim that its checks ran. Read the minimum sufficient context and preserve applicable repository rules.

## Delivery procedure

Map the actual operation, sources, output destination, credentials, rate/usage limits and irreversible side effects. Select only the capabilities matching the implementation and source.

Define stable IDs, retry boundaries and checkpoints before adding concurrency. Separate pure transformations from external writes. Simulate timeouts, duplicate inputs and partial progress using fixtures.

**Ship:** the authorized workflow produces valid output, bounded retries and visible errors; repeated execution does not create unintended duplicates. **Best:** additionally demonstrate recovery from interrupted external operations, deployed-account quotas and monitoring/ownership.

A lost response is not evidence that an external write failed. Reconcile before retrying a consequential action. Do not bypass source controls to achieve a throughput target.

Round-2 opportunity: offline checkpoint/idempotency fixture tests plus a run-receipt collector; live effects stay outside default checks.

## Return evidence

State what changed, why, checks performed, measurements, artifact paths and remaining limits. Record pass/fail/not-applicable for the task's agreed criteria. Unrun checks remain pending. Framework-specific state updates and memory admission belong to orchestration.

## Sources

Composed 2026-11-17 from the linked capabilities and the user's project requirements. Primary technical sources remain with each capability. Independent composition review is recorded in the project source-review ledger. Behavioral benefit remains experimental.

