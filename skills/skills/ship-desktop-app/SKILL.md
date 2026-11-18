---
name: ship-desktop-app
description: Deliver a desktop application change with tested user flows, native lifecycle
  and release evidence. Use for Tauri or comparable desktop delivery.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: dc010de3aa36ce89656ef9ce536a32682d0f113ba9a387aa5fe40e1c7f4003f7
  source-card: AI-Agents/skills/ship-desktop-app/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Ship a desktop app



## Route only the relevant work

This job procedure works in the current agent without a governor. It never creates subagents or installs dependencies implicitly. The following capability skills are candidates, not an instruction to load every one:

- [tauri-desktop-app](../tauri-desktop-app/SKILL.md)
- [react-app-performance](../react-app-performance/SKILL.md)
- [ci-release-engineering](../ci-release-engineering/SKILL.md)

If a required capability is unavailable, name the gap and use available project guidance; do not silently claim that its checks ran. Read the minimum sufficient context and preserve applicable repository rules.

## Delivery procedure

Identify the actual host stack first. Tauri guidance does not apply unchanged to Electron or a native Windows application. Load only the matching capability; React guidance applies only where React is used.

Define primary flows, supported desktop display, data boundaries and installation lifecycle. Follow the existing product/design contract and the [QLoop protocol](../../_shared/qloop-ui-protocol.md) for UI acceptance. This is a declared pack dependency: if unavailable, UI acceptance remains pending. Do not start critic agents implicitly. Keep backend and frontend responsibility explicit.

**Ship:** the requested release-build flow works, recovery/error states are usable, affected automated checks pass, and startup/interaction/memory budgets are recorded. **Best:** additionally verify clean install, applicable upgrade/signature behavior, representative long sessions and final UI acceptance evidence.

Check the built artifact rather than substituting development-server behavior. For AlphaBar the minimum physical display is 1920 by 1080. Do not add mobile scope to desktop work.

Round-2 opportunity: assemble app/runtime/build IDs, existing UI evidence and release receipts without controlling unrelated windows.

## Return evidence

State what changed, why, checks performed, measurements, artifact paths and remaining limits. Record pass/fail/not-applicable for the task's agreed criteria. Unrun checks remain pending. Framework-specific state updates and memory admission belong to orchestration.

## Sources

Composed 2026-11-17 from the linked capabilities and the user's project requirements. Primary technical sources remain with each capability. Independent composition review is recorded in the project source-review ledger. Behavioral benefit remains experimental.

