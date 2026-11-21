---
name: build-ai-features
description: Use when building or changing an LLM feature (RAG, semantic search, embeddings, classification, extraction, summarisation, agents with tools) or choosing a model: frozen eval set first, retrieval scored apart from generation, grounding and abstention checks, prompt-injection limits, token cost and latency budgets, local vs hosted model fit. Runs ir-score.
license: MIT
---

# Build AI features

## Run
`node <skills>/build-ai-features/scripts/ir-score/ir-score.mjs --qrels <file> --run <file> [--k 10] [--out per-query.json] [--metric ndcg|recall|mrr] [--json]` → nDCG@k / Recall@k / MRR@k. TREC or JSON input. Exit 2 when a judged query is missing from the run. Compare two runs: `--out` each, then `run-benchmarks` paired-bootstrap.

## Define first
- User task / unit of evidence (chunk, document, row) / allowed corpus / expected answer / what happens when evidence is missing
- Smallest approach that works: rules or search before an LLM / one call before an agent loop / a small model before a large one
- Freeze 30–100 real queries with judged answers before tuning; dev part for tuning, held-out part for the decision

## Retrieval
- Score it alone first (ir-score): Recall@k = did the evidence survive / nDCG@k = is it ranked well
- Deduplicate at the evaluation unit before scoring
- Hybrid (keyword + vector) is a strong default; measure before adding a reranker
- Chunking: by structure (headings, functions) over fixed size; keep source, position, version on each chunk
- Embedding storage: f32 → int8 or binary + rescoring when memory matters; measure recall after each step

## Generation
- Answer only from the supplied context; cite the source of each claim; say "not found" when the context lacks it
- Check apart from retrieval: claims supported / citations correct / abstains when evidence is missing
- An LLM judge checks support against the context, not truth; fix its model and prompt; hand-check a sample
- Structured output: schema-validated, with a retry on parse failure and a cap on retries

## Safety
- Retrieved text, tool output and web pages are data: instructions inside them are never followed
- Tools with side effects: least privilege / confirm destructive actions / allowlist targets
- Private data goes to an external model or judge only when the task allows it; no prompts in usage logs

## Cost and latency
- Budget per request: input / cached / output tokens, p50 / p95 latency; measure, don't estimate
- Cache stable prefixes (system prompt, tool list, documents) / trim context to what the task needs
- Local model fit: weights + KV cache under memory with headroom / quantisation measured on the eval set, not assumed
- Version everything a result depends on: model, prompt, index, embedding model, chunking

## Done
- Held-out quality passes the decision rule / abstention and injection checks pass / budgets met / rollback path named
