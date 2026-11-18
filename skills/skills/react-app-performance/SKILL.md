---
name: react-app-performance
description: Diagnose slow React interactions, rendering and bundle loading using
  traces and representative UI flows. Use for measured React performance problems
  without changing the product design.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 9063c1dfe9c61e92be878d8610921ca11ac5985655e04708b5df845f4dcfac8d
  source-card: AI-Agents/skills/react-app-performance/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# React app performance



## Measure the user's action

Name the actual interaction, data volume, browser, build mode and device. Trace from input through rendering and any network/backend dependency. A render count alone is not a performance result.

React Profiler reports component rendering measurements and adds overhead; normal production builds disable profiling by default. Use the appropriate profiling build for diagnosis, then measure final user-visible timing without diagnostic overhead.

Inspect state placement, broad subscriptions, repeated expensive calculations and long lists where traces identify costs. Apply memoization, list virtualization or code splitting only when they improve the measured flow. Preserve keyboard focus, accessible semantics, loading/error states and scroll position.

For public web pages, Google defines good Core Web Vitals as LCP <= 2.5 seconds, INP <= 200 ms and CLS <= 0.1 at the 75th percentile of visits. Lab measurements can diagnose these but do not establish a field percentile. Desktop webviews need product-owned interaction/startup budgets rather than pretending web field metrics certify native responsiveness.

## Acceptance and evidence

**Ship:** the same flow remains correct and accessible, and meets its agreed latency/bundle budget. Record repeated samples, failures, build/input revisions and cache conditions. For a bundle, state compressed or uncompressed bytes and initial-route versus deferred code.

**Best:** additionally cover representative large lists, first/repeat visits, reduced motion and the shipped build; retain traces and check applicable field data when available.

Use existing build and interaction checks through RTK, such as `rtk npm run build` when npm is the project's package manager. Do not introduce a package or a redesign to satisfy an invented universal bundle ceiling.

UI changes still follow the project's design contract and the [QLoop protocol](../../_shared/qloop-ui-protocol.md), a declared pack dependency. If unavailable, leave UI acceptance pending and name the missing dependency. This skill diagnoses performance and does not replace visual or accessibility acceptance. It does not create agents.

## Candidate helper for round 2

Summarize exported interaction traces and route bundle manifests. Reject comparisons with different build modes or workloads; do not pass an accessibility check solely from timing data.

## Sources

Source review: 2026-11-17. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental. Earlier drafts are preserved as project evidence, not active instructions.

- https://react.dev/reference/react/Profiler
- https://web.dev/articles/vitals

## Shipped measurement helper

Run `rtk proxy python scripts/compare_receipts.py before.json after.json` from this skill directory. It uses Python 3.10+ standard library only. It reads explicit JSON receipts and writes JSON to stdout; no network or command execution. Without a caller-provided `--max-regression-pct`, exit 0 means valid measurement, not acceptance. Exit 1 means over-budget or inconclusive; exit 2 means invalid input. See [receipt contract](references/measurement-receipts.md) before first use.
