---
name: deliver-data-analysis
description: Deliver reproducible data analysis or a trading research result with
  provenance, numerical checks and appropriate uncertainty. Use for analytical deliverables,
  not live trading.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 15b0762ef31c3a34436e24aa1e889f0dbcb933a05a1b2f59cd0526e52056b140
  source-card: AI-Agents/skills/deliver-data-analysis/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Deliver data analysis



## Route only the relevant work

This job procedure works in the current agent without a governor. It never creates subagents or installs dependencies implicitly. The following capability skills are candidates, not an instruction to load every one:

- [python-performance](../python-performance/SKILL.md)
- [backtesting-rigor](../backtesting-rigor/SKILL.md)

If a required capability is unavailable, name the gap and use available project guidance; do not silently claim that its checks ran. Read the minimum sufficient context and preserve applicable repository rules.

## Delivery procedure

Define the question, population, dataset vintage, transformations, output and uncertainty before optimizing computation. Load backtesting-rigor only when temporal strategy research is involved.

Keep raw data provenance, schemas and transformation decisions. Verify units, missingness, joins, duplicate keys and reference calculations. Use the existing project stack; a small task does not justify migrating storage libraries.

**Ship:** the stated question is answered from reproducible inputs, numerical checks pass, limitations are visible, and relevant runtime/memory budgets are met. **Best:** additionally test sensitivity, data-quality failure cases and independent recomputation of material conclusions.

A faster pipeline is not a more reliable investment result. Separate software benchmarks from financial metrics. Apply the vault's household/French-tax framing when making personal investment conclusions; do not fabricate tax assumptions.

Round-2 opportunity: validate data schemas and numerical reconciliation and bundle provenance with existing computation receipts.

## Return evidence

State what changed, why, checks performed, measurements, artifact paths and remaining limits. Record pass/fail/not-applicable for the task's agreed criteria. Unrun checks remain pending. Framework-specific state updates and memory admission belong to orchestration.

## Sources

Composed 2026-11-17 from the linked capabilities and the user's project requirements. Primary technical sources remain with each capability. Independent composition review is recorded in the project source-review ledger. Behavioral benefit remains experimental.

