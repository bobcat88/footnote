# Footnote hybrid retrieval: how to improve the fusion stage

Research date: 2026-11-13. All web pages were read on that date. "Fetched" means I read the page text myself; "via search summary" means I only saw a search-tool summary and did not read the page.

Baseline: `score = 0.55*z(dense) + 0.45*z(bm25)`. Each list is z-scored over its own top-100, and a document missing from a list gets that list's pool minimum. Dense is the average of two 384-d local models. Measured by us: convex combination beats RRF(k=60); a per-query adaptive alpha from lexical peakedness hurts; pool 20 hurts and pool 200 is neutral; dense Rocchio PRF helps NFCorpus and hurts the French sets.

Evidence tags: [peer] peer-reviewed, [preprint] arXiv preprint (not peer-reviewed unless stated), [vendor] vendor documentation, [blog] blog or third party. Any number I could not verify is marked UNVERIFIED. Every "expected gain" below is my own estimate unless a source number is quoted next to it.

Noise warning (my estimate, UNVERIFIED): the paired standard deviation of per-query nDCG@10 differences between two close systems is typically around 0.1 to 0.2. With about 300 queries (SciFact has 300 test queries and NFCorpus 323, per arXiv 2609.22770), the standard error is then roughly 0.006 to 0.012. A single-dataset gain below about +0.01 cannot be resolved. Pool all datasets and use a paired bootstrap on per-query nDCG@10 for every experiment below.

---

## 1. Ranked experiments (max 10)

Ranking criterion: expected gain times plausibility, divided by cost. Experiments 1 and 2 are almost free. None of the expected gains exceeds about +0.01 to +0.02 nDCG@10, and the evidence says the fusion stage is already near its ceiling once a tuned convex combination is in place.

### E1. Exact score completion instead of "pool minimum" flooring
- **Change:** For every document in the union of the BM25 and dense top-100 pools, compute the true score from the other retriever. For BM25-only candidates, take the cosine from the stored vectors (about 384 multiply-adds per model). For dense-only candidates, get the true BM25 score from Tantivy (a scorer seek or explain on the doc). Then apply z-scoring with the means and standard deviations still taken from each retriever's own top-100, but use the true values rather than the floor. Optionally clip z to [-3, +4].
- **Expected gain:** My estimate is +0.000 to +0.010 nDCG@10. UNVERIFIED: I found no paper that A/B-tests "complete the missing score" against "floor to pool minimum".
  - Bruch et al. say ranks and scores outside a retriever's top-k are unknown, and they approximate them inside the union [peer, ACM TOIS 42(1) 2023; https://arxiv.org/abs/2210.11934, read via full-text passages].
  - The Fusion Tax study finds fusion hurts most on queries that only one retriever covers [preprint, https://arxiv.org/abs/2610.30612 (abstract), 2026-10-22].
  - Another paper shows truncated fusion is not equivalent to complete-list fusion [preprint, https://arxiv.org/abs/2608.07152 (abstract), 2026-08-07]. This is also a plausible reason why our pool-20 test hurt and pool-200 was neutral.
  - Our own pool 20/200 result suggests the effect is real but small.
- **Labels:** 0 new labels. Reuse existing qrels.
- **Cost:** Dense side is trivial. The BM25 side needs a per-doc Tantivy scoring path (moderate code, roughly a few ms for about 150 candidates; UNVERIFIED). No new model.
- **FR/EN:** Language-neutral.
- **Test recipe:** Add `--fill=exact|floor` to the fusion harness and run SciFact, NFCorpus, Alloprof and Syntec. Report the paired-bootstrap delta nDCG@10 vs the baseline. A sub-arm sets pool depth to 50, 100 and 200; if exact fill works, the pool-20 penalty should shrink.

### E2. Plateau and leave-one-corpus-out check of alpha (0.55) in z-space
- **Change:** No formula change. Sweep alpha from 0.30 to 0.80 in steps of 0.05 on each dataset. Then (a) choose alpha on all datasets except one and score the held-out one (leave-one-corpus-out, LOCO). (b) 5-fold CV inside each dataset. (c) Report the width of the plateau (the alpha range within 0.003 nDCG@10 of the maximum).
- **Expected gain:** Gain is not the point. It is risk control.
  - Fusion Tax (7 BEIR corpora, BGE-small, min-max top-100): LOCO alpha costs at most 0.014 nDCG@10 versus the in-sample optimum, except TREC-COVID at 0.031. Within-corpus 5-fold CV lands within 0.010 of in-sample. A fixed alpha of 0.6 to 0.7 (dense weight) is "a defensible transferable default" [preprint, https://arxiv.org/abs/2610.30612, Table IV; via full-text passages].
  - Bruch et al.: the convex-combination weight "converges with less than 5% of the training data, regardless of the magnitude of domain shift". The absolute number of queries was not stated in the passages I read, so UNVERIFIED. They report 0.6 to 0.8 (semantic weight, min-max/TM2C2) working consistently [peer, https://arxiv.org/abs/2210.11934].
  - Your alpha is in z-space, so these numbers are not directly comparable. Your 0.55 looks a little BM25-heavy relative to those min-max values.
  - A single global alpha is the better default. The Fusion Tax authors recommend "a per-corpus static policy" over per-query routing.
- **Labels:** 100 to 200 queries per corpus for CV is a safe working figure. The only source for 50 to 200 is an author's rule of thumb, "not from the research" [blog, https://bigdataboutique.com/blog/hybrid-search-fusion-methods-compared, 2026-10-19, fetched]. We already have BEIR and MTEB-Fr labels.
- **Cost:** Scripting only.
- **FR/EN:** Run per language. If the French sets want a different alpha than the English ones, that is a language-level global rule (two values), not a per-dataset one. Note that the Fusion Tax rule is "dense-led by at least 0.04 nDCG@10 gives alpha about 0.6 to 0.7, balanced gives about 0.5" (their words), which suggests the optimum moves with the measured BM25-vs-dense gap.
- **Test recipe:** `for a in 0.30..0.80: eval each dataset`, then LOCO and 5-fold on the 4 datasets and the union. Accept a per-language alpha only if LOCO-per-language beats the global LOCO by more than the bootstrap CI.

### E3. Separate weights for the two embedding models (3-way fusion) and a "drop the weaker model" control
- **Change:** Replace `z(dense)` = mean of `z(A)` and `z(B)` with `wA*z(A) + wB*z(B) + wL*z(bm25)`, with wA+wB+wL=1. Grid in steps of 0.05 (about 200 points) with CV. Include the controls wB=0 and wA=0.
- **Expected gain:** My estimate is 0 to +0.01. This is the weakest-sourced experiment.
  - I found no paper that cleanly measures averaging two same-dimension bi-encoders against the better single one. UNVERIFIED.
  - Bruch et al. explicitly leave "fusing three or more models" to future work [peer].
  - The Infinity/"weakest link" study shows a weak path can drag the fused result below the best single path: Touché BM25 0.650, dense 0.390, RRF 0.604 nDCG@10 [preprint, https://arxiv.org/abs/2508.01405, v2, fetched passages]. Their 3-path (FTS+SVS+DVS) is a balanced "sweet spot" for accuracy per cost.
  - In French legal retrieval, fusing several domain-general models helped zero-shot. But once models were trained in-domain, about 70% of fusions were worse than the best single system unless the weights were carefully tuned. Equal weighting "consistently" reduced performance there [preprint, https://arxiv.org/abs/2409.01357, Louis et al., 2024-09-02].
  - Together these say equal weights are a risk and weights should be measured.
- **Labels:** 150+ queries per language for a 2-D weight grid. Use CV and a coarse grid to avoid overfitting.
- **Cost:** Needs per-model scores kept separate until fusion (probably already available). Latency about zero.
- **FR/EN:** Likely to differ by language. If one model is clearly weaker in French, expect wB toward 0 there.
- **Test recipe:** Single run of the weight grid per dataset with 5-fold CV. Compare to the current equal blend with the paired bootstrap, per language.

### E4. Query-independent squashing of BM25 instead of per-query z-score
- **Change:** Replace `z(bm25)` with a fixed-scale saturating transform such as `2*atan(bm25/8)/pi` (Vespa's tutorial form), or `tanh(bm25/c)`, or `bm25 / sum_t(idf_t*(k1+1))` (divide by the query's BM25 upper bound). Keep z for dense (or test raw cosine). Tune only the scale c on CV.
- **Expected gain:** My estimate is -0.005 to +0.01. UNVERIFIED for z-score vs atan. What the sources do show:
  - Vespa's hybrid tutorial (NFCorpus, nDCG@10, fetched via Firecrawl 2026-11-13, https://docs.vespa.ai/en/learn/tutorials/hybrid-search.html) lists bm25 0.3210, semantic 0.3077, hybrid-rrf 0.3233, hybrid-linear-normalize 0.3423, and the atan-normalized BM25 + cosine profile 0.3410. So atan is about equal to linear normalization and about +0.018 over RRF there. The page text says atan "performs the best" while the table shows linear-normalize slightly higher; I could not resolve which, so UNVERIFIED.
  - Per-query top-100 z-scoring throws away absolute lexical confidence (a query with one very strong exact match and a query with no good match both get z-scores of similar spread).
  - Bruch et al. find the theoretical-minimum min-max (TM2C2), which uses a fixed lower bound, is "more robust to the data distribution", but they also prove that most linear normalizations are rank-equivalent per query, so the choice is "a rather small detail" [peer]. OpenSearch added a `lower_bounds` option for min_max in 3.0 (via search summary of docs.opensearch.org normalization-processor page).
  - The Qdrant docs warn that DBSF statistics taken from a small prefetch can be skewed by one outlier [vendor, https://qdrant.tech/documentation/concepts/hybrid-queries/, fetched].
- **Labels:** One scale parameter, about 100 queries.
- **Cost:** One-line change. It also removes dependence of BM25 scores on the pool.
- **FR/EN:** The BM25 scale differs per corpus size, language and analyzer. The scale c probably needs per-corpus calibration, for example from the corpus's median top-1 BM25. This is a risk for a product with user-supplied corpora.
- **Test recipe:** `--bm25-norm=z|atan:c|ub`, with c in {4, 8, 16, 32}. Hold the sweep on a train split, then evaluate on test, with the same alpha re-tuned.

### E5. Gate dense Rocchio PRF with fusion agreement and draw feedback from the fused list
- **Change:** Run PRF only if the top-3 (or top-5) overlap between the BM25 list and the dense list is at least a threshold (strong feedback signal). Build the Rocchio vector from the fused top-k, not the dense-only top-k. Fuse again after PRF (interpolate before and after).
- **Expected gain:** My estimate is a recovery of the French loss to about 0 and keeping NFCorpus gains. UNVERIFIED for this exact gate.
  - Controlled-signal study: strong PRF signals help everything and weak ones hurt, with losses above 60% for BM25+Rocchio and above 50% on some dense models. Dense Rocchio is less fragile than bag-of-words but still unstable [preprint/SIGIR'22, https://arxiv.org/abs/2205.05888, read].
  - Interpolating sparse and dense both before and after PRF gave the best effectiveness across most datasets [preprint, https://arxiv.org/abs/2205.00235 (abstract)].
  - Applying a strong re-ranker before PRF improved PRF by 5% to 8% by raising feedback precision [preprint, https://arxiv.org/abs/2306.17082 (abstract)]. We have no cross-encoder, but the fused list is a higher-precision feedback source than dense alone (hypothesis).
  - Selective PRF is itself a known direction [preprint, https://arxiv.org/abs/2401.11198 (abstract)].
  - This fits our result: first-stage precision on the French sets is probably lower (UNVERIFIED; no French PRF source found).
- **Labels:** 100 to 200 queries to set the overlap threshold. Beware: per-query routing features transfer badly across corpora (see section 2).
- **Cost:** One extra dense search over the candidate set or the flat index (cheap locally), plus a second fusion pass. No second query encode is needed.
- **FR/EN:** Intended to fix French. Evaluate per language.
- **Test recipe:** `prf=off|always|gated(t)` with t in {1,2,3} of top-5 overlap. Pass criterion: gated >= off on both French sets and >= always on NFCorpus within CI.

### E6. Agreement bonus (CombMNZ-style) for balanced corpora
- **Change:** `final = convex_z + beta * 1[doc in both top-k lists]`, or multiply by the number of retrievers that returned the doc. Sweep beta in {0, 0.1, 0.2, 0.3} in z units.
- **Expected gain:** My estimate is 0 to +0.005 on balanced sets, negative or zero on dense-led sets. UNVERIFIED in a z-score convex setting.
  - Fusion Tax Table V (BGE-small, nDCG@10, min-max top-100): NFCorpus RRF 0.3570, CombMNZ 0.3574, interpolation at 0.7 0.3547. TREC-COVID RRF 0.8010, CombMNZ 0.8045, interpolation at 0.7 0.7166. But on dense-led sets interpolation at 0.7 wins (SciFact 0.7307 vs CombMNZ 0.7230; FiQA 0.4087 vs 0.3872) [preprint, https://arxiv.org/abs/2610.30612, read].
  - "Rank fusion or agreement-based when neither component should dominate" is their conclusion.
  - Our z-score combination may already capture agreement because a doc present in both lists gets two positive z contributions.
- **Labels:** About 100 queries.
- **Cost:** One line.
- **FR/EN:** Language-neutral.
- **Test recipe:** `--agree-bonus=beta`. Report separately for datasets where BM25 and dense are within 0.01 nDCG@10 of each other (balanced) and for the rest.

### E7. LexBoost-style neighbour smoothing of the BM25 score (offline dense kNN graph)
- **Change:** At index time, store for each chunk its n nearest neighbours (n about 8 to 16) from the dense model. At query time, replace each candidate's lexical score s(d) by `lambda*s(d) + (1-lambda)*mean_{d' in N(d)} s(d')`, with lambda about 0.7. Then fuse as usual.
- **Expected gain:** Evidence is for BM25 alone, not for the hybrid. My estimate for the hybrid is 0 to +0.01. UNVERIFIED.
  - LexBoost on TREC DL 2019, BM25 nDCG@10 0.4989, LexBoost(16) with lambda 0.7: 0.526 (+0.027; the dagger for significance is on nDCG@100 and nDCG@1000, not clearly on nDCG@10). nDCG@1000 0.6023 to 0.6332 (significant) [preprint, https://arxiv.org/abs/2409.05882, Kulkarni et al., 2024-08-25, read]. The weight lambda was transferred from DL19 to DL20, and the fusion is itself a convex combination.
  - Risk: our dense signal already enters the fusion, so the gain may largely be redundant.
  - English MS MARCO evidence only. Language-agnostic in principle since the graph uses dense similarity.
- **Labels:** 50 to 100 queries to tune lambda.
- **Cost:** Offline O(n^2) kNN (or ANN) at ingest, storage n ids per chunk, and incremental updates when notes change. This is the most expensive item here in code and index size.
- **FR/EN:** Likely both.
- **Test recipe:** Build the 16-NN graph with model A on SciFact and NFCorpus. Run `bm25 -> smooth(lambda in 0.6..0.9) -> fusion`, compare to baseline with the paired bootstrap.

### E8. Small supervised fusion (logistic regression or GBM) trained on official train splits
- **Change:** Train a model over features `[z_dense_A, z_dense_B, z_bm25, in_bm25_pool, in_dense_pool, ranks, log doc length]`, optimising pairwise or listwise loss. Train on BEIR NFCorpus train (2,590 queries as used in 2609.22770) and SciFact train (809). Evaluate on held-out corpora (LOCO).
- **Expected gain:** My estimate is about 0, and it may be negative on transfer. Direct evidence is negative at this scale:
  - Logistic regression on candidate features scored 0.774 train and 0.745 test, below the tuned fusion at 0.753 test, on SciFact [preprint, https://arxiv.org/abs/2609.22770, read].
  - Positive evidence needs far more features and data: a forest model blending 253 lexical features with dense features gave up to +11% nDCG@10 on MS MARCO with a 4.3% latency increase [preprint, https://arxiv.org/abs/2510.16393 (abstract), 2025-10-18]. That is a different regime (big training set, many features, English).
  - A feature-based router's signals change sign across corpora (see section 2).
- **Labels:** Thousands of query-document judgments (UNVERIFIED threshold). Only English BEIR train splits are known to exist; French train data for Alloprof/Syntec is UNVERIFIED.
- **Cost:** Training script plus a small model at inference. LightGBM adds a dependency; logistic regression can be a dot product.
- **FR/EN:** The training data is English, so French transfer is the risk.
- **Test recipe:** Train on NFCorpus-train + SciFact-train, test on Alloprof, Syntec and FiQA. Stop if it does not beat the global alpha on the French sets.

### E9. Narrow rule gate for rare or exact tokens (identifiers, quoted strings, digits, code)
- **Change:** A hard rule, not a learned model. If the query contains a quoted phrase, an identifier-like token (camelCase, snake_case, digits plus letters), or a token with idf above a threshold that matches few documents, shift alpha by a fixed amount (for example from 0.55 to 0.40). Otherwise leave it alone.
- **Expected gain:** About 0 on BEIR and MTEB. UNVERIFIED for any dataset. Its value is in product queries that those benchmarks do not contain.
  - Pinecone's legacy docs suggest alpha 0.75 for natural-language queries, 0.5 for mixed and 0.25 for SKUs/IDs/named entities, but this is a vendor heuristic with no measurements (via search summary of docs.pinecone.io, not directly verified).
  - Adaptive IDF weighting in a local-first SQLite system gave +0.1% (SciFact) to +3.4% (FiQA) nDCG@10 under RRF with fixed 0.6/0.4. The +21.4% on ArguAna was "primarily from the adaptive distance cutoff", not from weighting [preprint, https://arxiv.org/abs/2604.15484, vstash, read]. So this is not strong evidence either way.
  - Our own adaptive-alpha result and the literature below point the same way, which is why this is last and narrow.
- **Labels:** Needs a custom eval set of 30 to 100 identifier-style queries with judgments. There is no public benchmark in our list for this.
- **Cost:** A regex plus one branch. Latency about 0.
- **FR/EN:** Language-neutral rule.
- **Test recipe:** Build an identifier-query set from your own notes or code snippets. Compare gated vs not on that set only, and check the gate does not fire on SciFact, NFCorpus, Alloprof or Syntec (it must be a no-op there).

---

## 2. What the evidence says will not help (and consistency with our own failures)

| Method | Verdict | Why (source) |
|---|---|---|
| Replacing convex combination with RRF, weighted RRF, or tuned k | Will not help. Possible exception: balanced corpora. | Bruch et al.: RRF is sensitive to parameters, CC beats RRF in and out of domain [peer, https://arxiv.org/abs/2210.11934]. OpenSearch docs report RRF averaging 3.86% lower NDCG@10 than score-based hybrid on six BEIR datasets (via search summary of docs.opensearch.org RRF page and launch blog; per-dataset numbers UNVERIFIED). Weaviate switched its default from rank fusion to relative-score fusion in 1.24 after an internal ~6% recall gain on FIQA (no absolute values) [vendor, https://weaviate.io/blog/hybrid-search-fusion-algorithms, fetched]. Fusion Tax: weighted RRF with 0.7/0.3 weights closes 25% to 73% of the gap to interpolation on dense-led corpora, so weighting explains about half the RRF disadvantage and the score information the rest [preprint, https://arxiv.org/abs/2610.30612]. But RRF/CombMNZ win on balanced sets (TREC-COVID 0.801 vs 0.717). |
| Tuning RRF k | Little value. | Bruch: separate k per retriever helps one dataset and hurts another (RRF(10,4): MS MARCO 0.451 vs 0.425 for RRF(60,60); HotpotQA 0.621 vs 0.675). Fully parameterised RRF stays below TM2C2 [peer]. Qdrant default k=2 (zero-based), settable from v1.16.0 [vendor, fetched]. |
| Adding a rank-kernel (RRF) term inside the convex combination | No gain. | Bruch Fig. 12, "RRF-CC": no significant impact [peer]. A rank-score mix with lambda 0.75 and kappa 20 reached 0.779 on SciFact train, the same as min-max alpha 0.8 and z-linear alpha 0.8 [preprint, https://arxiv.org/abs/2609.22770]. |
| Other normalizers (min-max, z-score, percentile, L2, DBSF, tanh) | Expect +-0.003 or less. | Bruch: any linear normalizer gives rank-equivalent fusion up to retuning alpha [peer]. Percentile normalization "does not surpass traditional methods" in French legal retrieval; tuned z-score NSF was best zero-shot [preprint, https://arxiv.org/abs/2409.01357, read]. Engines offer them anyway: OpenSearch min_max (default), l2, z_score (3.0, 2025-05-06) [via search summary]; Elastic minmax, l2_norm (9.1) [vendor, fetched]; Qdrant DBSF (mu +- 3 sigma, v1.11.0) [vendor, fetched]. Only a query-independent squash (E4) is structurally different. |
| Per-query adaptive alpha from cheap features (score gap, peakedness, query length, mean idf, language) | Consistent with our failure. Expect 0 or negative. | Fusion Tax: failure signal exists within a corpus but "its carriers and their directions change across corpora"; the held-out router "does not pay"; they recommend a per-corpus static policy. The per-query oracle gains 0.02 to 0.05 nDCG@10 over static fusion, but heuristics do not capture it [preprint, https://arxiv.org/abs/2610.30612]. Financial retrieval: oracle headroom 21.8% but none of three routers (score-confidence, random forest on query features, ridge on query embeddings) beat the fixed blend under grouped CV [preprint, https://arxiv.org/abs/2608.00183]. On SciFact a list-geometry-conditioned alpha added +0.0006 nDCG@10 and a sparse-off router lost nDCG [preprint, https://arxiv.org/abs/2609.22770]. Thomson Reuters: the per-query optimal weight is an interval, not a point; a ridge model on TF-IDF query features performed like a single dataset-mean weight; the 74/128 significant wins are against equal-weight RRF, not against a tuned static weight, and the best methods need a GPU encoder or an LLM [preprint, https://arxiv.org/abs/2610.24341, read]. |
| LLM-based per-query weighting (DAT and similar) | Out of scope (no LLM). | DAT uses an LLM to score each top-1 [preprint, https://arxiv.org/abs/2503.23013 (abstract)]; about 205 ms per query in the Thomson Reuters measurement [preprint, https://arxiv.org/abs/2610.24341]. |
| Candidate pool larger than about 100 to 200 | Neutral, consistent with our pool-200 result. | Top-200 to 400 gave "no empirical gain" [preprint, https://arxiv.org/abs/2610.24341]. A larger candidate list "fails to resolve" the weakest-link problem [preprint, https://arxiv.org/abs/2508.01405]. |
| Candidate pool 20 or less | Hurts, consistent with ours. | Truncated fusion is not equivalent to complete-list fusion [preprint, https://arxiv.org/abs/2608.07152]. Elastic's own docs say a higher `rank_window_size` improves relevance at a performance cost; its default is 10 [vendor, fetched]. |
| Intersection of the two pools | Will hurt (my reasoning, no direct source). | The union is what every engine uses. The Fusion Tax authors find fusion preserves recall in aggregate; the loss is concentrated on queries only one retriever covers, so an intersection would delete exactly the documents where fusion adds recall. |
| MMR or diversity reranking for nDCG | Will hurt nDCG. Use only for near-duplicate collapse. | MMR cost 20.4% to 25.4% nDCG@10 for +24% intra-list diversity on TREC-COVID [preprint, https://arxiv.org/abs/2604.13728]. On clean pools, every classical diversifier lost relevance: MMR lambda 0.9 lost 0.7 S-Recall points, lambda 0.3 lost 30.6; on ArguAna MMR collapses NDCG@10 to 0.2 [preprint, https://arxiv.org/abs/2610.09412]. It helps only when the candidate pool has heavy near-duplicates (about 0.003+ near-duplicate pair fraction). BEIR nDCG will not reward it. |
| Off-the-shelf cross-encoder on the fused head | Not helpful here. | MS MARCO MiniLM cross-encoder on the fused top-50 gave 0.694 nDCG@10 on SciFact vs 0.753 for the fused list [preprint, https://arxiv.org/abs/2609.22770]. -0.3% to -3.1% and +560 to 2,100 ms in vstash [preprint, https://arxiv.org/abs/2604.15484]. About 1.6 s per query for 30 candidates on an Apple M5 (integrated GPU) in the Fusion Tax measurement [preprint]. English-trained models are an extra French risk (UNVERIFIED). |
| Dense PRF applied to every query | Consistent with the French loss. | See E5: weak feedback signals hurt [preprint, https://arxiv.org/abs/2205.05888]. |

### Are our own results consistent with the literature?
- **Convex beats RRF (lab ablation):** Yes. Bruch et al. 2023, OpenSearch, Weaviate and the Fusion Tax all agree on dense-led corpora. Caveat from Fusion Tax: on balanced corpora (NFCorpus, TREC-COVID in their runs) RRF and CombMNZ tie or win, so a per-dataset difference is plausible.
- **Alpha 0.55 from a sweep:** Plausible. Published dense weights are 0.6 to 0.8 (Bruch), 0.6 to 0.7 (Fusion Tax LOCO) and 0.7 to 0.9 (2609.22770), all for min-max-style scales with one dense model. Your z-score scale and two-model blend are different, so do not compare directly. E2 resolves this.
- **Adaptive alpha from lexical peakedness hurts:** Consistent. Cheap query-level signals have repeatedly failed to beat a well-tuned static weight (Fusion Tax, financial retrieval, SciFact list-conditioned alpha).
- **Pool 20 hurts, 200 neutral:** Consistent (Thomson Reuters, Infinity, truncated-fusion work).
- **Dense Rocchio helps NFCorpus and hurts French:** Qualitatively consistent with PRF's dependence on first-stage precision (Li et al. 2022). There is no direct French PRF evidence in what I found, so the French cause is UNVERIFIED.

---

## 3. Other questions in the brief, with what I found

**Alpha tuning with few labels / global vs per-dataset**
- Bruch et al.: alpha converges with under 5% of a training split; the absolute count was not given in the passages I read.
- Fusion Tax: 5-fold CV mode equals the in-sample optimum on all 7 corpora; held-out nDCG within 0.010.
- Overfit exists: CQA unix lost 0.012 after a frozen fit [preprint, https://arxiv.org/abs/2609.22770].
- Qdrant's docs advise tuning weights on an evaluation set with a train/validation split and leaving defaults when you have none [vendor, fetched].
- Recommendation: global alpha per language, validated by LOCO (E2).

**Embedding ensembles**
- No strong direct evidence found. Procrustes work shows two independently trained models can be aligned with an orthogonal map learned from about 10,000 texts, giving access to a stronger query model with fixed document vectors; this is not an ensemble gain [preprint, https://arxiv.org/abs/2510.13406, read]. 
- For a two-model average, evidence is limited to the French legal study (fusion helps zero-shot, weights matter) and the weakest-link study (a weak path can hurt).

**Learned fusion and calibration**
- Small LR was slightly worse than a tuned mix (E8 evidence).
- Score-distribution calibration models need a per-run fit and were discussed as costly in the Fusion Tax related work; their own recommendation is interpolation when one component is reliably stronger and rank fusion when neither should dominate.

**Candidate pool: depth, union vs intersection**
- Depth 100 is the standard in the sources I read (Fusion Tax top-100; Qdrant says a larger prefetch stabilises DBSF; Azure keeps text recall at 1,000 by default and returns 50).
- Union is universal. No source tests intersection (see table).

**Dedup and diversity**
- Not an nDCG lever (table). Near-duplicate collapse (cosine above a high threshold) is a product-quality lever, not a BEIR lever.

---

## 4. How the engines do it today (accessed 2026-11-13)

| Engine | What it does | Recommendation / notes | Source |
|---|---|---|---|
| Elasticsearch | RRF retriever: `rank_constant` default 60, `rank_window_size` default 10, per-retriever `weight` (default 1.0) GA since 9.2. Linear retriever: weighted sum with normalizer `none` / `minmax` / `l2_norm` (l2_norm since 9.1); a top-level default normalizer since 9.2, earlier default none. | Docs say avoid `none` because it "may bias the result set towards lexical matches". Higher `rank_window_size` "will improve result relevance at the cost of performance". Docs give no specific tuning values. | https://www.elastic.co/docs/reference/elasticsearch/rest-apis/retrievers/rrf-retriever and .../linear-retriever [vendor, fetched] |
| OpenSearch | Normalization processor: `min_max` (default), `l2`, `z_score` (added in 3.0, 2025-05-06); combination `arithmetic_mean` (default), `geometric_mean`, `harmonic_mean`, with `weights`; z_score only with arithmetic mean. RRF added in 2.19. | Docs report RRF ~3.86% lower NDCG@10 than score-based on six BEIR sets with latency gains under 2% (secondary). 3.7 (GA 2026-06-10) adds a Hybrid Optimizer in Search Relevance Workbench sweeping 82 variants per query (min_max/l2 x 3 means x 11 weights, z-score x 11, RRF constants 1, 5, 10, 20, 60). It names no winner and says to rerun on your own data. | https://docs.opensearch.org/3.1/search-plugins/search-pipelines/normalization-processor/ (via search summary); https://www.instaclustr.com/blog/opensearch-3-7-faster-vectors-and-smarter-hybrid-search/ [blog, fetched, 2026-06-19] |
| Weaviate | `relativeScoreFusion` (min-max per search, then weighted sum; introduced 1.20, default from 1.24) vs `rankedFusion` (1/(rank+60)). | Authors say relative score keeps more information; internal FIQA benchmark showed ~6% recall improvement, no absolute numbers. | https://weaviate.io/blog/hybrid-search-fusion-algorithms [vendor, fetched] |
| Qdrant | RRF (v1.10.0, zero-based ranks, default k=2, k settable v1.16.0, weights v1.17.0); DBSF (v1.11.0): per-query mean +- 3 sigma min-max, summed; formula queries for post-fusion boosts (v1.14.0). | With an eval set use weighted RRF; trust-the-magnitudes without an eval set: DBSF; neither: RRF. DBSF can be skewed by outliers in a small prefetch; raise the prefetch limit. | https://qdrant.tech/documentation/concepts/hybrid-queries/ [vendor, fetched; last ~13k characters unread] |
| Azure AI Search | RRF only for multi-query (hybrid, multiple vector queries); `1/(rank+k)`, doc says k=60 performs best; per-vector-query weight multiplier; semantic ranker (0-4 score) runs after RRF. Default top 50 returned; `maxTextRecallSize` default 1,000. | Weights tuned via debug subscores. Doc page updated 2026-09-17. | https://learn.microsoft.com/en-us/azure/search/hybrid-search-ranking [vendor, fetched] |
| Vespa | Global-phase functions: `normalize_linear` (min-max over the global-phase hits), `reciprocal_rank`, `reciprocal_rank_fusion`; global phase reranks the top 100 hits by default. | NFCorpus tutorial: see E4 for the numbers. | https://docs.vespa.ai/en/phased-ranking.html (via search summary); https://docs.vespa.ai/en/learn/tutorials/hybrid-search.html [vendor, via Firecrawl query] |
| Pinecone | Current guide: filter-then-rank in one request, or client-side RRF over separate searches. Says "RRF is a fusion method, not a synonym for hybrid search". Legacy sparse-dense index: client-side convex alpha scaling; suggested starting alphas 0.75 / 0.5 / 0.25 by query type. | Alpha guidance is a vendor heuristic and came only via a search summary of legacy pages. The current guide did not mention alpha. | https://docs.pinecone.io/guides/search/hybrid-search [vendor, fetched]; legacy: https://docs.pinecone.io/guides/data/encode-sparse-vectors (search summary) |
| Survey view | A 2026 survey finds RRF is the recommended or sole fusion in Elasticsearch, Azure, LangChain, and the safe default in Qdrant and Pinecone's cross-index case; OpenSearch (min-max default) and Weaviate (relative score default since 1.24) are the exceptions. | Time-sensitive documentation survey, not an effectiveness comparison. | https://arxiv.org/abs/2610.30612 §II-B [preprint, read] |

---

## 5. Sources

Papers (all read through the Firecrawl paper-reading tool unless noted "abstract only"):
1. Bruch, Gai, Ingber. An Analysis of Fusion Functions for Hybrid Retrieval. ACM TOIS 42(1), 2023. https://arxiv.org/abs/2210.11934 (v of 2023-05-26). Read.
2. Chauhan, Agarwal. The Fusion Tax: A Diagnostic Analysis of When Hybrid Retrieval Hurts. 2026-10-22. https://arxiv.org/abs/2610.30612. Read.
3. Ma et al. (Thomson Reuters). Optimal Fusion Weights Form Intervals, Not Points. 2026-10-17. https://arxiv.org/abs/2610.24341. Read.
4. Pati, Patil. Parameterized Dense-Sparse Fusion ... Rank-Score Mix on BEIR SciFact with Qdrant. 2026-09-19. https://arxiv.org/abs/2609.22770. Read. Single-company preprint, no paired significance test (stated in its limitations).
5. Wu, Lin. Evidence-Unit Fairness and the Limits of Query-Adaptive Sparse-Dense Fusion in Financial Document Retrieval. 2026-07-31. https://arxiv.org/abs/2608.00183. Abstract only.
6. Wang et al. Balancing the Blend (Infinity "weakest link"). 2025-08-02, v2 2025-11-04. https://arxiv.org/abs/2508.01405. Read.
7. Louis, van Dijck, Spanakis. Know When to Fuse: Non-English Hybrid Retrieval in the Legal Domain (French). 2024-09-02. https://arxiv.org/abs/2409.01357. Read.
8. Steffens. vstash: Local-First Hybrid Retrieval with Adaptive Fusion. 2026-04-16. https://arxiv.org/abs/2604.15484. Read. Single-author preprint.
9. Kulkarni et al. LexBoost: Improving Lexical Document Retrieval with Nearest Neighbors. 2024-08-25. https://arxiv.org/abs/2409.05882. Read.
10. Li et al. How does Feedback Signal Quality Impact Effectiveness of PRF. SIGIR 2022. https://arxiv.org/abs/2205.05888. Read.
11. Li et al. To Interpolate or not to Interpolate: PRF, Dense and Sparse Retrievers. 2022-04-30. https://arxiv.org/abs/2205.00235. Abstract only.
12. Mackie et al. Adaptive Latent Entity Expansion for Document Retrieval. 2023. https://arxiv.org/abs/2306.17082. Abstract only.
13. Datta et al. A Deep Learning Approach for Selective Relevance Feedback. 2024. https://arxiv.org/abs/2401.11198. Abstract only.
14. Hsu, Wang, Tzeng. DAT: Dynamic Alpha Tuning. https://arxiv.org/abs/2503.23013. Abstract only (LLM-based, excluded).
15. Santra et al. Beyond Correlations: A Downstream Evaluation Framework for QPP. 2026-01-24. https://arxiv.org/abs/2601.17339. Abstract only; reports >4.5% gains for QPP-weighted fusion over unweighted CombSUM and RRF. Not compared with a tuned static weight, so I did not use it as support.
16. Zhang. Exact Adaptive Hybrid Retrieval Without Fixed Top-L Cutoffs. 2026-08-07. https://arxiv.org/abs/2608.07152. Abstract only.
17. Prajapati. Hybrid Retrieval for COVID-19 Literature (rank vs projection fusion, MMR). 2026-04-15. https://arxiv.org/abs/2604.13728. Abstract only.
18. Brouillette et al. Finding the Right Balance: Relevance and Diversity in LLM Retrieval. 2026-10-07. https://arxiv.org/abs/2610.09412. Read.
19. Nardini et al. Blending Learning to Rank and Dense Representations for Efficient and Effective Cascades. 2025-10-18. https://arxiv.org/abs/2510.16393. Abstract only.
20. Maystre et al. When Embedding Models Meet: Procrustes Bounds and Applications. 2025-10-15. https://arxiv.org/abs/2510.13406. Read.

Engine and vendor documentation (accessed 2026-11-13 unless a date is given):
21. Elastic RRF retriever: https://www.elastic.co/docs/reference/elasticsearch/rest-apis/retrievers/rrf-retriever (fetched)
22. Elastic linear retriever: https://www.elastic.co/docs/reference/elasticsearch/rest-apis/retrievers/linear-retriever (fetched)
23. OpenSearch normalization processor: https://docs.opensearch.org/3.1/search-plugins/search-pipelines/normalization-processor/ (search summary only)
24. OpenSearch RRF docs / launch blog: https://docs.opensearch.org/latest/vector-search/ai-search/hybrid-search/rrf/ and https://opensearch.org/blog/introducing-reciprocal-rank-fusion-hybrid-search/ (search summary only; 3.86% figure not independently verified)
25. OpenSearch 3.7 Hybrid Optimizer (Instaclustr, 2026-06-19): https://www.instaclustr.com/blog/opensearch-3-7-faster-vectors-and-smarter-hybrid-search/ (fetched)
26. Weaviate: https://weaviate.io/blog/hybrid-search-fusion-algorithms (fetched; post dated 2023-08-29 but describes 1.24)
27. Qdrant hybrid queries: https://qdrant.tech/documentation/concepts/hybrid-queries/ (fetched, first 100k of 113k characters)
28. Azure AI Search hybrid ranking (page updated 2026-09-17): https://learn.microsoft.com/en-us/azure/search/hybrid-search-ranking (fetched)
29. Vespa hybrid tutorial: https://docs.vespa.ai/en/learn/tutorials/hybrid-search.html (via Firecrawl query extraction; table and rank-profile code obtained, narrative text not fully read). Vespa phased ranking: https://docs.vespa.ai/en/phased-ranking.html (search summary only).
30. Pinecone hybrid guide: https://docs.pinecone.io/guides/search/hybrid-search (fetched); legacy alpha pages via search summary only.
31. BigData Boutique, "Hybrid search fusion methods compared", 2026-10-19: https://bigdataboutique.com/blog/hybrid-search-fusion-methods-compared (fetched; third-party; its 50 to 200 query figure is explicitly the author's rule of thumb).

---

## 6. Gaps and caveats
- I could not find a paper that measures averaging two same-dimension embedding models against the better single model in a hybrid with BM25. E3 rests on indirect evidence.
- I found no French-specific PRF evidence, and no French sparse-dense fusion study on Alloprof or Syntec. The only French fusion source is a legal-domain study (BSARD-style LLeQA) with trained retrievers.
- I did not read the engine pages in full where noted ("via search summary"). Vespa page: the extracted table and the extracted sentence disagreed on whether atan or linear-normalize was best (0.3410 vs 0.3423); I report both numbers.
- Many 2026 preprints here are single-lab and unreviewed. Treat the Fusion Tax (the page lists an IEEE Access DOI, review status UNVERIFIED), vstash, and the SciFact/Qdrant paper as hypotheses generators, not settled results. Bruch et al. is the only peer-reviewed anchor I read in full-text passages.
- Absolute number of labelled queries needed to tune alpha is not established by any source I read (Bruch gives "under 5% of a training split"). My working figure of 100 to 200 queries per corpus is a judgment.
