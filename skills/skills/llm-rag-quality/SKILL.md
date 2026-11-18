---
name: llm-rag-quality
description: Evaluate retrieval and grounded generation with held-out queries, relevance
  judgments, latency and usage accounting. Use for memory search and RAG changes,
  not generic chatbot styling.
metadata:
  version: 0.1.0
  review-status: source-reviewed; independent coverage and parent gap closure recorded
  behavioral-status: experimental; no general quality or efficiency claim
  distribution-status: experimental; not installed; no measured efficiency claim
  source-sha256: ba0e86573fcff17096b1a919aac932d892dbc5a10058b74b4225e56caddfea25
  source-card: AI-Agents/skills/llm-rag-quality/SKILL.md
  source-normalization: UTF-8 without BOM; CRLF converted to LF before source hashing
---
# LLM RAG quality



## Use and evidence contract

Run this skill directly in the current agent. It does not require a governor or start subagents. Load another skill only when its expertise is needed; framework reporting is optional.

Before changing code, name the workload, correctness oracle, input revision, runtime/tool versions, machine, cache conditions and project-owned performance budget. Record a baseline and repeat the same workload after the change. Use the repository's existing benchmark runner; if none exists, a small repeatable command is sufficient. Report raw samples, the summary statistic, failures and uncertainty. Do not claim a p95 from a handful of runs.

On Windows, prefix supported external commands with RTK. Commands below show the wrapped form. Do not install profilers, change production settings or run costly workloads merely because this skill mentions them.

**Ship:** applicable correctness checks pass, measurements are reproducible, the project budget is met, and regressions are explained. **Best:** additionally cover representative scale and failure cases, retain benchmark receipts, and verify the deployed/release configuration. These are local delivery policies, not external standards. Record non-applicable checks with reasons; never invent a missing measurement.

## Freeze the evaluation unit

- Define whether results are chunks, documents, memories or sessions. Deduplicate at the declared unit before scoring.
- Freeze corpus snapshot, relevance judgments, queries, train/dev/test split and retrieval depth. Fit thresholds on dev data; keep held-out queries out of prompt tuning.
- Report Recall@k for relevant-item survival and nDCG@k for graded ranking; specify relevance thresholds and treatment of unjudged items. Include per-query results and paired differences.
- Match input corpus, returned units, search mode, hardware and warm/cold state across systems. Unequal keyword-only and hybrid pilots cannot establish competitive superiority.
- Evaluate generated answers separately: supported claims, correct citations, answer correctness, abstention on missing evidence and resistance to instructions embedded in retrieved text.
- A faithfulness judge checks support against supplied context; it is not proof that the context is true. Human-check a sample of judge decisions and record judge model/version.

## Measurements and checks

Record retrieval and end-to-end p50/p95 latency, indexing time, disk/peak-memory footprint, answer quality, input/output/cache tokens, failure counts and actual versus estimated cost. Do not silently drop failures from denominators. A project chooses quality and latency floors before comparison.

Do not ingest private corpora or send them to external judges without task authorization. Use existing approved fixtures first. Keep payloads out of usage-only telemetry.

## Candidate helper for round 2

Score frozen qrels and run files with documented metrics, query-level receipts and missing-query detection. Keep deterministic retrieval scoring separate from paid or nondeterministic answer judges.

## Sources

Source review: 2026-11-17. Sources support the methods above, not universal performance thresholds. Independent coverage and parent gap closure are recorded in the project source-review ledger. Behavioral benefit remains experimental.

- https://ir-measur.es/en/latest/measures.html
- https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/faithfulness/

