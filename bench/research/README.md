# Research, 2026-11-13

Four desk studies done by separate research agents (web and paper search, nothing installed), each written before any of its ideas was tested.
They are hypotheses with sources, not results. Results land in `../experiments/RESULTS.md` and `../receipts/`, through the same harness and the same
rules as everything else (dev to choose, heldout to confirm, paired bootstrap, cost next to every gain).

| Report | Question | Leading ideas | Status |
|---|---|---|---|
| [competitors.md](competitors.md) | Which two popular products to run head-to-head, besides ICM | **mem0** 2.2.1 (68.9k stars; raw add, no LLM, Qdrant local, dense candidates re-scored by BM25) and **Cognee** 1.6.8 (33.3k stars; SQLite + LanceDB + graph, chunk index) | mem0 adapter built (`../competitors/mem0_engine.py`, `run.py ... ext:mem0`); Cognee environment installing |
| [lexical-boost.md](lexical-boost.md) | How to improve the lexical stage around Tantivy | multilingual learned sparse (OpenSearch neural sparse, query side model-free) fused with BM25; small CPU reranker on the BM25 top 20; per-chunk language routing; character 4-gram field | queued |
| [fusion-boost.md](fusion-boost.md) | How to improve the fusion of lexical and dense | exact score completion; alpha plateau and leave-one-corpus-out check; separate weights for the two models; query-independent squash | running (`../sweeps/fusion2.jsonl`) |
| [embedding-boost.md](embedding-boost.md) | Which current small models to test | EmbeddingGemma-300M, Harrier-270m, granite-small-english-r2, arctic-embed-m-v2.0 (latency is the constraint: 3 to 4 times the cost of the current models) | models registered and downloaded with hash locks; MTEB-reproduction gate running |

## What the reports agree with in our own earlier results

- RRF loses to convex combination; per-query adaptive weights and pseudo-relevance feedback do not help; BM25 parameter tuning and BM25 variants do not matter
  (Tantivy hard-codes k1 and b, our sweep was done in the Python lab).
- int8 vectors cost nothing; Matryoshka truncation to 256 costs 1 to 2 percent, 128 costs 4 to 8 percent (our E11 found the same order).

## Comparison rules for the head-to-head

- Same datasets and splits through `fetch.load`, same metrics (`metrics.py`), same receipt format (`run.py`), queries clipped to 500 bytes for everyone.
- Each product runs as it ships locally, in its own pinned environment, with no LLM call, and the receipt states what was NOT exercised
  (mem0's LLM extraction, Cognee's graph).
- Storage is reported by part (store, derived index, model files), latency with its scope (in-process call versus resident MCP session).
- Vendor-published numbers are quoted with their provenance and never mixed with ours.
