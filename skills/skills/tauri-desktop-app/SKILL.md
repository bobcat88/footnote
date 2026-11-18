---
name: tauri-desktop-app
description: Measure and improve Tauri startup, IPC, resource use and release behavior
  while preserving capability boundaries. Use for Tauri desktop delivery and diagnosed
  native/webview bottlenecks.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 56d79d91ca66104608fb73990beb8dfc6b02944c28d4735cd03647db65436617
  source-card: AI-Agents/skills/tauri-desktop-app/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Tauri desktop app



## Establish the shipped configuration

Record Tauri/runtime versions, target OS, release build, features, installer type and WebView runtime. Trace startup from process launch to a usable primary action; first paint is a separate milestone.

Before tuning, record a correctness-checked baseline. Repeat baseline and candidate runs under matched workload, machine, runtime and cache conditions; retain raw samples and report variability. Do not infer tail latency from a handful of launches or claim improvement inside measurement noise.

Tauri uses message passing between frontend and backend. Measure representative IPC payloads and concurrency before changing transport or batching. Avoid blocking work on the interaction path; preserve cancellation and error reporting.

Review the permissions and capabilities actually needed by each window and command. Follow the installed version's security model; do not infer a security failure from the number of capability files. Do not expose a command or disable CSP for benchmark convenience.

Measure the app's whole process tree, including webview children, using an explicit memory definition and settling interval. Keep startup cache state, backend initialization and frontend loading separate.

## Acceptance and evidence

**Ship:** required flows work in the release build, startup/interaction/resource budgets belong to the product, and commands expose only required access. Installer size includes distribution assumptions: a webview bootstrapper and an offline bundle are not comparable artifacts.

**Best:** additionally verify install/upgrade/recovery on a clean supported environment, actual signing/update behavior where applicable, long-running resource behavior and representative IPC load. A signature file's presence alone does not prove authenticity or downgrade resistance.

Use the project's normal commands through RTK, typically `rtk cargo test --locked` plus its frontend build. Verify current updater documentation and pinned source before prescribing configuration keys or vulnerability mitigations. Do not invent keys or require an updater for an offline product.

This skill works in the current agent. Use React performance guidance only when traces implicate the frontend. UI changes require the project design contract and the [QLoop protocol](../../_shared/qloop-ui-protocol.md), a declared pack dependency. If it is unavailable, leave UI acceptance pending and name the missing dependency. Reading this skill does not start an independent critic or other agent.

## Candidate helper for round 2

Capture startup milestones and process-tree memory receipts with runtime/build IDs. Any app-launch helper needs explicit executable/path controls and must not install, update or close unrelated applications.

## Sources

Source review: 2026-11-17. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental. Earlier drafts are preserved as project evidence, not active instructions.

- https://v2.tauri.app/concept/inter-process-communication/
- https://v2.tauri.app/security/capabilities/
- https://v2.tauri.app/plugin/updater/
- https://v2.tauri.app/distribute/windows-installer/

## Shipped measurement helper

Run `rtk proxy python scripts/compare_receipts.py before.json after.json` from this skill directory. It uses Python 3.10+ standard library only. It reads explicit JSON receipts and writes JSON to stdout; no network or command execution. Without a caller-provided `--max-regression-pct`, exit 0 means valid measurement, not acceptance. Exit 1 means over-budget or inconclusive; exit 2 means invalid input. See [receipt contract](references/measurement-receipts.md) before first use.
