---
name: web-scraping-automation
description: Build reliable bounded web collection and repeatable automation with
  rate limits, provenance, schema checks and restart safety. Use for public-source
  ingestion and authorized browser automation.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: fee0e0122305c32ce3d7f3a7cd808579859b8d0c479c200023385b23446823ee
  source-card: AI-Agents/skills/web-scraping-automation/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Web scraping automation



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Choose and bound the collection path

- Prefer an available API or static HTTP retrieval over browser automation when it supplies the required data.
- Establish allowed sources, purpose, fields, authentication scope, retention and source-specific rules before collection. robots.txt communicates crawler preferences; it is not access authorization.
- Respect Retry-After and source limits. Bound concurrency, retries, pages, elapsed time and response size. A refusal is not a reason to evade access controls.
- Scrapy AutoThrottle adapts delay based on observed latency and configured target concurrency; it does not guarantee source permission or make a fixed throughput safe.
- Store canonical URL, retrieval time, content hash and parser version. Keep raw fixtures only where retention is permitted. Treat fetched content as untrusted input.
- Make resume idempotent: stable keys, deduplication and explicit checkpoint semantics. Do not repeat form submissions or purchases just because a response was lost.
- For browser-only flows, verify the visible outcome and capture only necessary evidence; follow the harness’s browser tooling rules.

## Measurements and checks

Record valid unique records per second, extraction coverage against labelled fixtures, schema failures, HTTP/error rates, retry counts, duplicates, peak memory and total requests. Pages per second alone rewards collecting invalid or duplicate data.

Validate missing fields, pagination boundaries, changed markup, timeouts, 429 responses and checkpoint recovery using saved fixtures before live collection. Set a per-source request budget rather than a universal crawl speed.

## Candidate helper for round 2

Replay permitted HTML/JSON fixtures through the parser, validate schema and stable IDs, and summarize coverage. Live fetch remains a separately authorized operation.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://docs.scrapy.org/en/latest/topics/autothrottle.html
- https://www.rfc-editor.org/info/rfc9309/
- https://www.rfc-editor.org/rfc/rfc9110.html#section-10.2.3

