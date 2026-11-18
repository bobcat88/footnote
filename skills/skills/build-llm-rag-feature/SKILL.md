---
name: build-llm-rag-feature
description: Deliver a grounded LLM or retrieval feature with held-out quality, latency
  and usage evidence. Use for memory search, document retrieval and grounded-answer
  workflows.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: 7c2c73d748511dc5d09d20a065759fe596054678e8e3ccce2e231b171e1f8335
  source-card: AI-Agents/skills/build-llm-rag-feature/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# Build an LLM RAG feature



## Route only the relevant work

This job procedure works in the current agent without a governor. It never creates subagents or installs dependencies implicitly. The following capability skills are candidates, not an instruction to load every one:

- [llm-rag-quality](../llm-rag-quality/SKILL.md)
- [python-performance](../python-performance/SKILL.md)
- [rust-performance](../rust-performance/SKILL.md)

If a required capability is unavailable, name the gap and use available project guidance; do not silently claim that its checks ran. Read the minimum sufficient context and preserve applicable repository rules.

## Delivery procedure

Define the feature's user task, evidence unit, permitted corpus, expected answer behavior and abstention/error states. Use the implementation language capability that actually applies; do not load both by default.

Freeze a small representative evaluation before tuning. Establish the existing baseline and evaluate retrieval independently of generation. Use approved corpora and endpoints; record changed retrieval/index/prompt/model versions.

**Ship:** correctness and evidence-grounding requirements pass on held-out queries, applicable abstention/failure and data-access-boundary checks pass, and quality/latency/resource budgets are met. A check may be non-applicable only with a reason showing why the change cannot affect that behavior. Tuning fixtures alone cannot establish quality acceptance. **Best:** additionally verify representative scale, reproducible index/model configuration and a monitoring/rollback plan.

Keep usage observations, price-based cost estimates and subscription headroom distinct. Do not claim a quality win from synthetic fixture recall or compare unequal retrieval modes as competitive proof.

Round-2 opportunity: one receipt linking query-level retrieval scores, answer-review evidence, latency and usage without retaining private prompts unnecessarily.

## Return evidence

State what changed, why, checks performed, measurements, artifact paths and remaining limits. Record pass/fail/not-applicable for the task's agreed criteria. Unrun checks remain pending. Framework-specific state updates and memory admission belong to orchestration.

## Sources

Composed 2026-11-17 from the linked capabilities and the user's project requirements. Primary technical sources remain with each capability. Independent composition review is recorded in the project source-review ledger. Behavioral benefit remains experimental.

