# Optimization experiments: results

Pre-registered in [PLAN.md](PLAN.md) on 2026-11-12, before any result. Base: the frozen default (bekko-a8m, BM25 with English stemmer,
convex combination z-score alpha 0.55, whole documents). Rows come from `../sweeps/experiments.jsonl` and `../sweeps/ensemble.jsonl`;
the tables in [TABLES.md](TABLES.md) are generated from them by `report_exp.py`. Dev split for tuning, heldout for confirmation.

## What the rules said

A change ships if it gains 0.01 nDCG@10 on **every** dataset of its language group on dev and then holds on heldout with a paired bootstrap interval
that excludes zero, or if it is neutral (within 0.005) and strictly cheaper.

## Outcomes

| Id | Change | Verdict | Why |
|---|---|---|---|
| E01 | BM25 k1/b (Tantivy fixes k1 1.2, b 0.75) | no loss | within 0.005 on all datasets except Alloprof at -0.0052; the Rust port can use Tantivy's fixed values |
| E02 | drop low-idf query words | rejected | negative or flat; idf 3 costs up to 0.014 |
| E03 | lexical pseudo-relevance feedback | rejected | negative everywhere (-0.002 to -0.030) |
| E04 | title boost | rejected | +0.004 on SciFact, nothing elsewhere |
| E05 | dense Rocchio feedback | rejected | helps NFCorpus by 0.007 to 0.009, hurts the French and mixed sets by up to 0.022 |
| E06 | per-query fusion weight | rejected | negative on the French and mixed sets |
| E07 | candidate pool 20 / 50 / 200 | keep 100 | pool 20 loses up to 0.020; 200 gains at most 0.002 |
| E08 | dense scoring only over the lexical top-N | **ships at N = 1000** | -0.001 to -0.0025 on dev, and neutral to positive on heldout; N = 100 loses up to 0.028. Removes the full dense scan |
| E09 | f16 vectors | ships (free) | identical ranking, half the memory |
| E10 | int8 vectors | **ships** | identical ranking (within 0.0004), a quarter of the memory |
| E11 | fewer dimensions (256, 192, 128) | rejected | 128 dimensions costs 0.049 on Alloprof |
| E16 | blend a second dense model | **ships** | `gte-small` at weight 0.5 gains +0.012 / +0.018 / +0.024 / +0.015 on dev; `e5-small-multi` and `minilm-l12-multi` do not pass the rule |

## Heldout confirmation of the final stack

Stack = ensemble (bekko-a8m + gte-small, weight 0.5) + int8 vectors + dense scoring over the lexical top-1000. Receipts: `../receipts/heldout/`.
Paired bootstrap, 10,000 resamples, 95 percent interval on the nDCG@10 difference against the frozen default (`compare.py`).

| Dataset (heldout) | Default | Final stack | Difference | 95% interval | Verdict |
|---|---|---|---|---|---|
| SciFact (300 q) | 0.7335 | 0.7480 | +0.0144 | [+0.0022, +0.0277] | better |
| NFCorpus (323 q) | 0.3514 | 0.3706 | +0.0192 | [+0.0081, +0.0316] | better |
| Alloprof (1,624 q) | 0.4785 | 0.4914 | +0.0129 | [+0.0050, +0.0209] | better |
| mix-fr-en (2,320 q) | 0.4865 | 0.4979 | +0.0115 | [+0.0063, +0.0166] | better |

The ensemble without the two-stage shortlist scores 0.7462 / 0.3727 / 0.4912 / 0.5009 (all four intervals also exclude zero), so the
shortlist and the int8 vectors cost nothing measurable. Int8 vectors with the shortlist and a single model are neutral to slightly
positive (+0.003 to +0.007, intervals mostly touching zero).

## Costs

- The ensemble runs a second encoder: about twice the query encoding time and twice the vector storage (two int8 vectors of 384 dimensions per chunk, 772 bytes).
## E12: a cross-encoder only where the dense ranking is unsure

`bge-reranker-v2-m3`, pool 10, 200 Alloprof dev queries, base = the frozen default (0.5083). The gate reranks a query only when the cosine gap
between the two best documents is below the margin. Rows: `../sweeps/gated_rerank.jsonl`.

| Gate (cosine gap below) | nDCG@10 | Gain | Queries reranked |
|---|---:|---:|---:|
| never | 0.5083 | - | 0% |
| 0.01 | 0.5208 | +0.0125 | 29.5% |
| 0.03 | 0.5416 | +0.0333 | 60.5% |
| 0.06 | 0.5343 | +0.0260 | 84.0% |
| always | 0.5468 | +0.0385 | 100% |

The gain tracks the share of queries reranked, with no sharp knee: the gate at 0.03 keeps 87% of the gain for 60% of the reranker calls. The reranker costs
1.4 to 2.8 s per query on a CPU, so it stays an optional high-precision mode, off by default (rule: a gain above 0.02 is needed to justify more than 2 ms).
The same sweep on the mixed French and English set was started and interrupted to free the machine for the product receipts; no mixed-language E12 result exists.

## E14: latency of the shipped binary at scale

`scale_product.py`: real document text (150-word spans cut from real corpora), the real indexer, the real Tantivy index, one resident `mcp` session,
300 real queries each encoded by both models, document vectors **random** int8 (speed only). A scratch state folder, never the user's audit log.
Rows: `../sweeps/scale_product.jsonl`. The SciFact row is a development timing against the index built for its receipt, not a receipt.

| Stored chunks | Strategy | p50 | p95 | Index build | Disk (SQLite + Tantivy) |
|---:|---|---:|---:|---:|---:|
| 5,183 | full dense scan | 20 ms | 23 ms | - | 37 MB |
| 40,018 | full dense scan | 44.7 ms | 67.0 ms | 13.9 s | 180 + 15 MB |
| 100,044 | BM25 shortlist of 1,000 | 52.2 ms | 79.3 ms | 28.8 s | 450 + 33 MB |

Two earlier versions of the same path, for the record: re-reading every vector from SQLite on each query took 31 to 38 ms at 5,183 chunks, and validating all 1,000
shortlisted candidates in scan mode was the second largest cost. Both were measured, fixed and merged (PRs 7 and 8); the ranking did not change.
Remaining cost at 5,000 chunks is mostly the two model encodes (about 3 to 5 ms each) and SQLite lookups.

## E13: precision with 100,000 distractors

Not run. The script exists (`scale_quality.py`) and embedding 100,000 distractor documents is a long CPU job; no claim about precision at 100,000 chunks or more is made anywhere.
