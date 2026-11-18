---
name: website-launch-checklist
description: Review public website launch or client handover readiness across performance,
  accessibility, security, France/EU applicability, search visibility and operations.
  Use for launch decisions; this does not authorize publishing.
metadata:
  version: 2.1.0-draft
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 6931a47979d1f0551dbcc1f150bf03c05540106fab39f1e084639f734c0c0196
  source-card: AI-Agents/skills/website-launch-checklist/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Website launch checklist



## Establish the actual release contract

Record publisher/jurisdiction, audience, site type, payments/accounts, personal-data flows, supported browsers, target devices and deployment scope. Read [launch checks](references/launch-checks.md) for the applicable areas, not as unconditional legal requirements for every site.

Use source-backed standards where applicable and project-owned budgets elsewhere. Do not derive a universal page-weight, JavaScript, conversion or Lighthouse-score requirement from a population average. Ship/Best are local delivery policies; no client is forced to accept an unrelated benchmark target.

**Ship:** primary flows, relevant security/data boundaries, applicable legal information and accessibility checks have evidence; agreed performance budgets are met in a declared test environment. Missing checks stay pending. **Best:** additionally verify representative deployment conditions, operational recovery and the agreed post-launch field-measurement plan. A clean automated accessibility scan alone cannot establish accessibility.

## Measure and improve

Freeze representative pages and interactions, build revision, device/network conditions and cache state. Keep raw repeated lab samples. Google defines good field Core Web Vitals as LCP <= 2.5 seconds, INP <= 200 ms and CLS <= 0.1 at the 75th percentile of visits. A Lighthouse score or TBT measurement is not field INP acceptance; a new site may lack field evidence.

Trace the actual bottleneck. For LCP, separate server response, resource discovery, transfer and render delay; expose the LCP resource early and avoid lazy-loading the primary LCP image. For interactions, reduce unnecessary JavaScript and split work causing long main-thread tasks. For layout shifts, reserve media/dynamic-content space and inspect font loading. Measure the same flow after each change.

Review third-party purpose, consent, loading and ownership. Optimize caching by data sensitivity and asset identity; do not remove privacy protections merely to improve cache eligibility. Browser-specific back/forward-cache behavior changes: test the supported browser rather than assuming no-store always prevents it.

## Verify before handover

Test staging for safe iteration and the actual deployment for headers, redirects, compression, error statuses and configuration. Use existing approved tools and RTK-wrapped commands on Windows. Do not install scanners, change DNS, submit forms, create external accounts, send email or publish as an implicit consequence of this checklist.

UI acceptance uses the project's design contract and the [QLoop protocol](../../_shared/qloop-ui-protocol.md). This is a declared pack dependency: if unavailable, leave its acceptance pending. The skill does not start agents. Return pass/fail/not-applicable per applicable check, evidence paths and unresolved ownership decisions.

## Sources

Primary-source revision: 2026-11-17. The detailed reference links official legal, accessibility, security and performance guidance. Prior statistics, fines and unsupported universal gates were removed; the previous card is retained in project evidence. Independent source coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://web.dev/articles/vitals
- https://web.dev/articles/optimize-lcp
- https://web.dev/articles/optimize-long-tasks
- https://developer.chrome.com/docs/web-platform/bfcache-ccns
- [Conditional launch checks and sources](references/launch-checks.md)
