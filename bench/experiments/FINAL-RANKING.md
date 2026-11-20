# Final ranking, 14 November 2026

Everything below comes from receipts produced on 13 and 14 November on one machine (Intel Core Ultra 7 270K Plus, 24 logical CPUs, 47.6 GiB RAM, Windows 11). Chart: [RANKING.html](RANKING.html), rebuilt by `ranking_chart.py` from the same receipts. Quality is nDCG@10 on each corpus's test split. Δ quality = Footnote ÷ competitor − 1 (positive: Footnote better). Latency ×: competitor median query time ÷ Footnote's (above 1: Footnote faster). Significance: paired bootstrap, 10,000 resamples, 95 % interval, per comparison without multiplicity correction.

## 1. Products, same documents and queries

Each product keeps its own models, chunking and ranking. Best variant per competitor and corpus (the toughest comparison for Footnote). mem0 runs without LLM extraction; Cognee on its chunk index, not its graph; ICM keyword-only (no embedding model). Footnote's latency includes its MCP round-trip, ICM's an HTTP round-trip, mem0 and Cognee are in-process.

| Corpus | Footnote | mem0 | Δ | × | Cognee | Δ | × | ICM | Δ | × |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Mixed FR+EN, 3,039 q | **0.489** | 0.343 | **+42.4 %** | 10.3 | failed | | | 0.342 | **+42.8 %** | 1.1 |
| Alloprof FR, 2,316 q | **0.478** | 0.360 | **+33.0 %** | 4.4 | 0.305 | **+56.8 %** | 29.1 | 0.333 | **+43.5 %** | 1.4 |
| Syntec FR, 100 q | **0.795** | 0.745 | **+6.8 %** | 0.5 | 0.700 | **+13.6 %** | 5.8 | 0.657 | **+21.0 %** | 1.0 |
| SciFact EN, 300 q | **0.747** | 0.729 | +2.5 % (tie) | 6.4 | 0.719 | **+3.8 %** | 25.4 | 0.664 | **+12.4 %** | 1.3 |
| NFCorpus EN, 323 q | **0.368** | 0.357 | +2.9 % (tie) | 5.1 | 0.338 | **+8.7 %** | 30.5 | 0.307 | **+19.7 %** | 1.0 |

Bold Δ: the paired interval excludes zero. **Footnote has the best quality on all five corpora; it is significantly better in 24 of 26 comparisons; the two others are ties with mem0 on the English sets.** On speed, Footnote is 4 to 10 times faster than mem0 (except tiny Syntec, where mem0's in-process call wins 6.9 against 12.8 ms) and 6 to 30 times faster than Cognee; ICM keyword-only is as fast (1.0 to 1.4×). Intervals: [HEADTOHEAD-INTERVALS.md](HEADTOHEAD-INTERVALS.md), [ICM-INTERVALS.md](ICM-INTERVALS.md).

Cognee multilingual on mix-fr-en crashed inside Cognee's graph-database worker after a 3 h 30 build (its one retry was cut off by a 2-hour tool limit while still building, so it is inconclusive; see the overnight log). Cognee lexical on NFCorpus is not reproducible (Cognee's tie order); its interval carries a warning.

**Where Footnote loses: index build time, now 2.4 to 2.5 times faster than in these head-to-head receipts.** Build seconds in the head-to-head runs, Footnote / mem0 (best) / ICM / Cognee: SciFact 676 / 162 / 47 / 3,387; Alloprof 637 / 87 / 24 / 1,608; mix-fr-en 1,757 / 336 / 98 / failed. On 14 November the embedding helper was found to run each model on 2 CPU threads; document encoding now uses half the logical CPUs, with identical vectors and rankings ([BUILD-SPEED.md](BUILD-SPEED.md)). Re-measured: **SciFact 280 s, mix-fr-en 712 s**, same nDCG@10 and query latency. Footnote is still 1.7 to 2.1 times slower than mem0 and 6 to 7 times slower than keyword-only ICM on those two corpora (Alloprof not re-measured). Its stores are small (39 MiB SciFact, 110 MiB mix-fr-en; Cognee 3.4 GB on SciFact). Rejected on measurement: fp32 `gte-small` (slower than int8) and encoding both models at once (no gain).

## 2. Agent memory: LongMemEval-S, 500 questions

[LONGMEM-ICM.md](LONGMEM-ICM.md). Shipped Footnote binary: any@5 **0.978**, all@5 **0.908**, nDCG@10 **0.939**. ICM keyword-only, same machine: 0.952 / 0.820 / 0.875, Footnote better on every metric with every interval above zero. ICM's own published hybrid run (vendor machine): 0.974 / 0.886 / 0.926; Footnote better on all@10 and nDCG@10, tied on any@5, all@5 and any@10. ICM's local hybrid run is still to do.

## 3. Storage layer, identical inputs

[LANCEDB-RESULTS.md](LANCEDB-RESULTS.md): same chunks, same frozen bekko-a8m vectors, encoding excluded. Quality: SQLite + Tantivy equals LanceDB with our fusion everywhere; LanceDB's native RRF loses 0.011 to 0.013 on Alloprof and mix-fr-en (significant), ties elsewhere. Speed (p50): SQLite + Tantivy 0.4 to 2.3 ms; LanceDB native 6 to 21 ms; LanceDB storage with Tantivy 2.4 to 5 ms. Disk: LanceDB 30 to 35 % smaller. **Keep SQLite + Tantivy.**

Lexical scale, synthetic text, speed only: at 1M documents Tantivy p50 3.0 ms / p95 7.6 ms / 304 MB; LanceDB FTS 6.3 / 21.3 ms / 752 MB; SQLite FTS5 420 / 1,659 ms / 1,715 MB. **Tantivy is the right lexical engine at every size tested.**

## 4. Our own model: candidates

[MODEL-CONFIRMATION.md](MODEL-CONFIRMATION.md), [MODEL-LATENCY.md](MODEL-LATENCY.md). Every candidate that qualified on dev also wins on heldout (French/mixed: EmbeddingGemma-300m in the hybrid +0.073 and +0.063; arctic-m-v2 +0.055 and +0.042; English: EmbeddingGemma +0.043 and +0.017). **None passes the cost limits written before the results** (model file at most 600 MB, query encoding p95 at most twice today's pair): EmbeddingGemma fp32 is 1.2 GB and about 3.6× slower; its int8 and q4 exports are smaller but even slower on this CPU. Adopting EmbeddingGemma is a product decision rather than a rule outcome: +0.04 to +0.07 nDCG@10 for +10 to +25 ms per query and a 1.2 GB download, still faster than every competitor except ICM.

## 5. Accelerators

[GPU-ENCODING.md](GPU-ENCODING.md). DirectML on the RX 6800 helps only batch indexing with fp32 bekko-a8m (3.2×); single queries are 10 to 34 times slower; int8 models change results. OpenVINO through ONNX Runtime: batch bekko 1.6×, everything else slower. **ONNX Runtime on the CPU stays the default;** the install-time GPU detection idea is easy to build but not worth shipping on these numbers.

## Limits

Single runs per configuration; per-comparison intervals; public benchmarks may overlap with model training data (EmbeddingGemma's +0.14 on SciFact dev is suspiciously large); competitor environments are version snapshots, not hash-locked wheels. Rust fmt, clippy and tests pass.
