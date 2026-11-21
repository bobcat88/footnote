---
skill_hash: b79b271a7aa72e70
reviewed: 2026-11-21
---
# build-ai-features: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| nDCG, Recall, MRR definitions; queries without relevant docs excluded | [ir-measures: measures](https://ir-measur.es/en/latest/measures.html) · [trec_eval](https://github.com/usnistgov/trec_eval) |
| Faithfulness judges check support against context | [Ragas: faithfulness](https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/faithfulness/) |
| Prompt injection via retrieved content | [OWASP Top 10 for LLM Applications: LLM01](https://genai.owasp.org/llmrisk/llm01-prompt-injection/) |
| Prompt caching of stable prefixes | [Anthropic prompt caching](https://docs.claude.com/en/docs/build-with-claude/prompt-caching) |
| Binary and int8 embedding quantisation with rescoring | [Hugging Face: embedding quantization](https://huggingface.co/blog/embedding-quantization) |

## Field evidence
- This repository's retrieval benchmarks: keyword-only vs hybrid pilots could not support competitive claims; a candidate model better on held-out data was still rejected on query cost.
