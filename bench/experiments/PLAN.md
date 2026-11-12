# Optimization experiments (pre-registered 2026-11-12)

Goal: find layers that improve the Footnote retrieval pipeline beyond the frozen default (bekko-a8m + BM25 en_stem + convex combination,
z-score, alpha 0.55, whole documents). At most 20 experiments. Each row of `sweeps/experiments.jsonl` varies ONE thing from that base.
Reference for scale is 100k to 1M chunks (see `dbbake/results.jsonl`): an optimization that only helps at 10k chunks counts for little.

## Rules (written before the results)

- Tune on `dev`. A candidate must beat the base by at least 0.01 nDCG@10 on **both** datasets of its language group on dev
  (English group: SciFact, NFCorpus. Mixed/French group: Alloprof, mix-fr-en), or be neutral (within 0.005) and strictly cheaper.
- Candidates are then confirmed on `heldout` with the paired bootstrap (`compare.py`); only an interval that excludes zero counts as a gain.
- Cost is stated next to every gain: extra latency per query, extra memory per 1000 chunks, extra build time. A gain that costs more
  than 2 ms per query at 100k chunks, or more than 20 percent memory, needs a gain above 0.02 to ship.
- Winners are combined in a final stack (E15) and re-tested together, because gains rarely add up.

## Experiments

| Id | Layer | Change | Why it might help | Datasets |
|---|---|---|---|---|
| E01 | lexical | BM25 k1/b (Tantivy fixes k1 1.2, b 0.75) | the Rust port cannot tune them: how much do we lose? | all |
| E02 | lexical | drop query words with low idf | OR matching adds noise from question words | all |
| E03 | lexical | pseudo-relevance feedback (RM3-like) | vocabulary mismatch | all |
| E04 | lexical | title boost x2, x3 | titles carry the topic | scifact, nfcorpus, mix |
| E05 | dense | Rocchio query expansion toward the top documents | short questions | all |
| E06 | fusion | per-query alpha from lexical peakedness | queries differ in which signal is reliable | all |
| E07 | fusion | candidate pool 20/50/200 | latency and quality at scale | all |
| E08 | fusion | dense only over the lexical top-N (two-stage) | avoids a full dense scan at 1M chunks | all |
| E09 | vectors | f16 storage | half the memory | all |
| E10 | vectors | int8 storage | quarter of the memory, faster scan | all |
| E11 | vectors | truncate to 256/192/128 dimensions | smaller and faster scan | all |
| E12 | rerank | cross-encoder only where the dense gap is small | pay the reranker only on hard queries | alloprof, mix |
| E13 | scale | quality with 100k real-text distractors | does precision hold at 100k chunks | scifact, alloprof |
| E14 | scale | warm latency of the full stack at 100k..1M chunks | the headline number | synthetic |
| E15 | stack | all winners together | gains may not add up | all |

E13 to E15 have their own scripts in this folder once the E01 to E12 sweep is read.
