---
name: ship-rust-tool
description: Deliver a Rust CLI or daemon change from requirements to tested release
  evidence. Use for Rust tool delivery, not isolated syntax questions.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 21bfecadda22b5c16ea094798aea067597be6a75891e952e9bb7f9e5c033d116
  source-card: AI-Agents/skills/ship-rust-tool/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Ship a Rust tool



## Route only the relevant work

This job procedure works in the current agent without a governor. It never creates subagents or installs dependencies implicitly. The following capability skills are candidates, not an instruction to load every one:

- [rust-performance](../rust-performance/SKILL.md)
- [ci-release-engineering](../ci-release-engineering/SKILL.md)
- [sqlite-sql-performance](../sqlite-sql-performance/SKILL.md)

If a required capability is unavailable, name the gap and use available project guidance; do not silently claim that its checks ran. Read the minimum sufficient context and preserve applicable repository rules.

## Delivery procedure

Define the CLI/service contract: inputs, outputs, errors, state persistence and supported platforms. Preserve current behavior unless the change explicitly replaces it.

Implement the smallest complete slice. Measure Rust hot paths only when relevant; inspect SQL when persistent state is touched. Use the existing CI/release process. Package the actual tested artifact and document recovery for state-changing operations.

**Ship:** behavior and failure cases satisfy the task, affected checks pass, and representative runtime/resource budgets are measured. **Best:** additionally demonstrate installation, upgrade/state compatibility, clean-build reproducibility and release provenance where applicable.

A daemon needs bounded shutdown and request/error behavior; a batch CLI needs meaningful exit statuses and explicit partial-failure behavior. Do not impose daemon concerns on a stateless command.

Round-2 opportunity: a release receipt assembler that references existing checks rather than rerunning the full suite unconditionally.

## Return evidence

State what changed, why, checks performed, measurements, artifact paths and remaining limits. Record pass/fail/not-applicable for the task's agreed criteria. Unrun checks remain pending. Framework-specific state updates and memory admission belong to orchestration.

## Sources

Composed 2026-11-17 from the linked capabilities and the user's project requirements. Primary technical sources remain with each capability. Independent composition review is recorded in the project source-review ledger. Behavioral benefit remains experimental.

