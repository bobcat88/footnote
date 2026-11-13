# Footnote lexical stage: what could beat plain Tantivy BM25

Research date: 2026-11-13. Every number below carries its source in the source list (section 4). Anything I could not confirm in this session is marked UNVERIFIED. No code was run; no number comes from Footnote's own harness except those quoted in the brief.

Baseline reminders from the brief: Tantivy 0.27, BM25 with fixed k1=1.2 and b=0.75, SimpleTokenizer + RemoveLong(40) + LowerCaser + AsciiFolding + English stemmer, OR query, chunks of about 2,000 characters, mixed FR/EN. Already measured by Footnote: k1/b moves nDCG@10 by under 0.005; RM3-like PRF and idf query pruning hurt; title boost no help; English stemmer beats French and combined stemming.

## 0. Facts about Tantivy that shape every experiment

| Topic | Verified state (2026-11-13) | Consequence |
|---|---|---|
| Current version | 0.27.0 is the latest release (docs.rs shows tantivy-0.27.0; the 0.27 version-bump commit #3197 is dated Oct 17, 2026 on the repo page). `main` has an "Upcoming Release" section (MSRV 1.91, `onpair` crate, new `DynamicColumn` variants) with nothing ranking-related. | You are on the newest release. Nothing newer to upgrade to for ranking. |
| BM25 parameters | `const K1: Score = 1.2; const B: Score = 0.75;` in `src/query/bm25.rs` on `main`. `Bm25Weight::new` is `pub(crate)`. Public hook is only `Bm25StatisticsProvider` (total_num_tokens, total_num_docs, doc_freq), added in PR #1855 (Feb 2023). | k1/b cannot be set through the public API. Your k1/b sweep must have used a patched fork or a custom Weight. Statistics can be overridden (e.g. compute idf over a larger or cleaner corpus), but not the formula. |
| BM25 formula | Lucene-style: idf = ln(1 + (N-n+0.5)/(n+0.5)), tf part freq/(freq + k1(1-b+b dl/avgdl)), times (k1+1). Field length is quantised to a 256-entry fieldnorm table. | Same family as Lucene "default" BM25. Kamphuis 2020 shows this family is equivalent to the other variants in effectiveness (see section 2). |
| Per-field scoring | Per-field boost only (`QueryParser::set_field_boost`, `BoostQuery`). No BM25F (no cross-field tf merge). `DisjunctionMaxQuery` exists; 0.27 fixes it being scored as a sum under `TopDocs` (#3022). | "BM25F" has to be approximated with DisMax or Boolean SHOULD across fields. Your title-boost null result is consistent with Pyserini (section 2). |
| Speed (block-max WAND) | 0.27 adds block-max pruning for top-K on conjunctions and Block-Max WAND for single-scorer boolean queries (#2865, #2897, #2912, #2915, #2981, #3003). In `boolean_weight.rs`, WAND runs only through `SpecializedScorer::TermUnion` / `TermIntersection`; any other SHOULD scorer falls into `SpecializedScorer::Other` and uses plain `for_each_pruning_scorer`. | Adding a non-term SHOULD clause (phrase, fuzzy, n-gram wrapper) probably disables block-WAND for the whole union. I read the dispatch code but did not benchmark: treat the latency hit as likely, size UNVERIFIED. |
| Phrase / slop | `PhraseQuery` with `set_slop`; slop supports transpositions since an earlier fix (#2031/#2020). `Weight::scorer_danger` (#3070) builds phrase scorers lazily inside intersections (helps only when phrase is MUST). Needs `IndexRecordOption::WithFreqsAndPositions`. `RegexPhraseQuery` exists (#2516). | Bigram boosting is doable without new index data if positions are already indexed (default for `TEXT`). |
| Fuzzy | `FuzzyTermQuery` (distance, transposition_cost_one, prefix variant). 0.27 adds `FuzzyTermQuery::automaton()` and public `DfaWrapper` (#3136). | Typo tolerance is available; none of your corpora has typos, so nDCG will not show a gain. |
| Tokenizers in 0.27 | `AlphaNumOnlyFilter, AsciiFoldingFilter, FacetTokenizer, LowerCaser, NgramTokenizer, RawTokenizer, RegexTokenizer, RemoveLongFilter, SimpleTokenizer, SplitCompoundWords, Stemmer, StopWordFilter, WhitespaceTokenizer` plus `Language`. No shingle / bigram filter. `StopWordFilter::new(Language) -> Option<StopWordFilter>` and `StopWordFilter::remove(words)`. | Index-time word bigrams need a custom `TokenFilter` (small). Check `StopWordFilter::new(Language::French).is_some()` for the French list (I believe it exists, not verified). |
| Stemmer backend | 0.27 replaced the unmaintained `rust-stemmers` with `frostem` (#3028; docs.rs dependency `frostem ^1.20260804.0`), fixing a Greek panic. | If your stemming baselines were measured on 0.26 or earlier, re-baseline on 0.27 before trusting small deltas. |
| `tantivy-stemmers` crate | v0.3.0 on crates.io. Snowball algorithms for about 30 languages including `french`, plus variants (`english_lovins`, `english_porter`, `english_porter_2`, `hindi_lightweight`, `czech_dolamic_light`...). Usage: `StemmerTokenizer::new(algorithms::french)` wrapped after `LowerCaser`. Release date and tantivy-version compatibility are not stated on the page: UNVERIFIED. | Tantivy itself already has Snowball French, so the crate adds little for FR. It only matters if you want alternative algorithms (Lovins is more aggressive than Porter2). |

## 1. Ranked experiments (at most 10)

Conventions: gains are nDCG@10 points (0.01 = one point) unless stated. "Harness recipe" uses placeholders: replace `<variant>` and the runner command with your own bench entry point. Always report per corpus (SciFact, NFCorpus, Alloprof, Syntec, union, LongMemEval-S) with paired bootstrap against current BM25, because your null results (k1/b under 0.005) show the noise floor is small.

### E1. Inference-free learned sparse channel, fused with BM25
- What to change: add a second first-stage retriever. Doc side: encode each chunk with `opensearch-project/opensearch-neural-sparse-encoding-multilingual-v1` (FR+EN, 160M params, Apache-2.0); keep the non-zero vocabulary weights (about 138 per doc on average, or 75 with `prune_ratio` 0.1). Query side needs no model: tokenise with the model tokenizer, weight each token by the shipped `idf.json`, dot product with doc weights. English control: `opensearch-neural-sparse-encoding-doc-v3-gte` (133M, also inference-free, supported by fastembed-rs 7.7.0 as `SparseModel::OpenSearchNeuralSparseDocV3Gte`). Store postings outside Tantivy (a `HashMap<token_id, Vec<(chunk_id, f32)>>` or a redb table; 100k chunks x 138 = about 14M postings) because Tantivy cannot score with arbitrary stored weights (K1/B are constants, no custom similarity). Fuse with the BM25 list by weighted sum of normalised scores, not RRF.
- Expected gain and evidence:
  - English standalone: doc-v3-gte NFCorpus 0.360 and SciFact 0.725 (model card) versus Lucene-flat BM25 0.322 and 0.679 (Pyserini BEIR): about +3.8 and +4.6. SPLADE-v3 (not shippable, see licence) gives 0.357/0.710 in its paper and 0.363/0.714 in Pyserini, consistent.
  - French/multilingual: OpenSearch multilingual-v1 MIRACL fr 0.558 versus BM25 0.115; average 0.629 versus 0.305. Caution: that BM25 is the weak XLM-R-subword BM25. BGE-M3's own appendix shows a proper Lucene-analyzer BM25 scores 38.5 on MIRACL versus 31.9 for the subword BM25 and 53.9 for BGE-M3 sparse, so the fair gap is about +15 average, not +32.
  - Counter-evidence for your exact corpora: Sparseup-fr-v0 (150M, blog dated 2026-10-12, licence not stated) on French NanoBEIR gets NFCorpus-fr 28.53 versus BM25 30.73 (loses) and SciFact-fr 67.47 versus 65.57 (+1.9), while the average is +7.9 driven by FEVER, FiQA, Quora, NQ. SciFact and NFCorpus are exactly the datasets where learned sparse gains are smallest or negative in French.
  - Fusion: Elastic reports a tuned weighted sum of BM25 and ELSER gives +24% relative average nDCG@10 over BM25 alone on a BEIR subset (English, Elastic's own data); transfer to your corpora UNVERIFIED.
  - Realistic expectation: +0.02 to +0.04 on SciFact/NFCorpus, unknown on Alloprof/Syntec/LongMemEval (no published number for the multilingual-v1 model on them).
- Cost: query latency about zero extra model time (tokenise + idf lookup + posting accumulation, low milliseconds for hundreds of postings per term, UNVERIFIED). Index time is the problem: 160M-parameter encoder over 512-token chunks on CPU; order of 0.1 to 1 s per chunk single-thread (UNVERIFIED estimate, measure). The model emits one score per vocabulary entry per token (vocabulary 105,879), so keep batches small (fastembed-rs README gives the same warning). Storage: small. Code: medium (ONNX export or ort + tokenizers, max-pool + log(1+relu), prune, postings, fusion). `ort` is already what fastembed-rs uses.
- FR/EN: both (multilingual-v1); doc-v3-gte is English only.
- Harness recipe: `bench run --retriever sparse_os_mlv1 --corpora all` and `bench run --retriever bm25+sparse --alpha {0.3,0.5,0.7}`; compare to BM25; also report union corpus and Recall@100 (sparse expansion should lift recall more than nDCG).

### E2. CPU cross-encoder rerank of the BM25 top-K
- What to change: rerank BM25 top 20 to 50 with a small cross-encoder, truncating each chunk to 128 to 256 tokens (the best-matching window if you can find it). English candidate: `mixedbread-ai/mxbai-rerank-xsmall-v1` (70.8M params, Apache-2.0). Multilingual candidates: `cross-encoder/mmarco-mMiniLMv2-L12-H384-v1` (multilingual MiniLM, trained on machine-translated MS MARCO; licence and French numbers UNVERIFIED); fastembed-rs `TextRerank` also lists `bge-reranker-base`, `bge-reranker-v2-m3` (568M, likely too slow), `jina-reranker-v1-turbo-en`, `jina-reranker-v2-base-multilingual` (licence UNVERIFIED, I recall non-commercial).
- Expected gain: mxbai card, 11 BEIR datasets via Pyserini: Lucene BM25 38.0 to 43.9 nDCG@10 (+5.9) for xsmall; base-v1 46.9. Reranking pool size is not stated on the card: UNVERIFIED. Expect +0.03 to +0.06 on SciFact/NFCorpus in English. For French: no verified number for any small reranker; the English-only model must not be used on French chunks.
- Cost: this is the budget breaker. SBERT's published docs/sec (MiniLM-L6 1,800/s, TinyBERT-L2 9,000/s) are GPU (V100) numbers from the docs page, not CPU. On a laptop CPU, 50 pairs x 256 tokens through a 6-layer model is plausibly 100 to 300 ms (UNVERIFIED estimate). "Tens of milliseconds" is realistic only for top 10 to 20 with 128-token windows and an L2 to L4 model. 568M models are out. Code: low to medium (fastembed-rs or ort).
- FR/EN: English only for the xsmall; the multilingual MiniLM covers French but is the weakest quality option.
- Harness recipe: `bench run --retriever bm25 --rerank mxbai-xsmall --pool {20,50} --max-tokens {128,256}`; log p50/p95 latency next to nDCG@10; run on the English corpora first, then `--rerank mmarco-mminilm` on Alloprof/Syntec.

### E3. Chunk granularity and chunk-to-document aggregation
- What to change: build indexes at 500, 1000 and 2000 characters (and, for chat data, one chunk per turn or user/assistant pair) and aggregate chunk scores to the session/document with max or max + 0.1 x sum of the next best before computing nDCG at the unit you actually return.
- Expected gain: LongMemEval paper (Sec. 5.2) finds the round is a better value granularity than the session for retrieval and QA; the paper also shows BM25 is clearly weaker than dense retrievers on this benchmark (Table 9 discussion). No BM25-only chunk-size numbers verified: magnitude UNVERIFIED. SciFact and NFCorpus documents are short abstracts, so expect no change there.
- Cost: zero model cost; index rebuild; slightly larger index at smaller chunks. Code: low.
- FR/EN: both.
- Harness recipe: `bench run --chunk-chars {500,1000,2000} --agg {none,max,max+sum2} --corpora lme,alloprof,syntec`.

### E4. Per-chunk language routing plus a light French stemmer (start with an oracle)
- What to change: first a diagnostic. Run BM25 with the French stemmer versus the English stemmer on each French-only corpus alone (Alloprof, Syntec), not only on the mixed union. If French stemming wins on pure-French data, the union loss is a routing problem, which this experiment fixes. Step 1 (oracle): create fields `text_en` and `text_fr` (analyzers: English stemmer; French stemmer or a light French stemmer) and fill exactly one per chunk using the corpus label. Query both fields (Boolean SHOULD, each field analysed with its own analyzer). Step 2: replace the oracle with `lingua` restricted to {French, English} per chunk. Step 3 variants: (a) light French stemmer written as a custom `TokenFilter` (strip plural s/x and simple feminine e, as Lucene's French minimal stemmer does, from memory UNVERIFIED), (b) drop `AsciiFoldingFilter` on the French field, (c) French elision/stop words via `StopWordFilter::new(Language::French)`.
- Expected gain: UNVERIFIED. No published effect size for per-chunk routing in a mixed index was found. Plausible range: 0 to +0.01 on French corpora, 0 on English. The reason your English stemmer won is probably that Snowball French over-stems short chunks; a lighter stemmer is the cheapest thing that could beat both. Cross-field score comparability is a real risk (idf and avgdl differ per field).
- Language detection facts: `lingua` is the most accurate on short text and lets you restrict languages; its README example reports 99.7% on sentences, 94.1% on word pairs and 73.9% on single words (German case). `whatlang` is lighter but less accurate on short text (its author says so); `whichlang` covers 16 languages and is fastest. A 2,000-character chunk is easy for any of them; queries are short, so do not route queries, run both analyzers on the query.
- Cost: index about 1x (each chunk in one field), query two analyzers. Code: low (oracle) to medium (lingua + custom filter).
- FR/EN: French gain only.
- Harness recipe: `bench run --analyzer {en,fr,fr_light,route_oracle,route_lingua} --corpora alloprof,syntec,union`.

### E5. Parallel character 4-gram field (language-agnostic)
- What to change: add a second text field tokenised with `NgramTokenizer::new(4, 4, false)` after lowercasing and ASCII folding (positions off to save space) and score it as a SHOULD clause with weight 0.3 to 1.0 next to the word field, or fuse the two ranked lists.
- Expected gain: McNamee and Mayfield (2004, CLEF 2002, European languages including French): n=4 is a good choice, accuracy rivals the best language-specific approaches and rivals or exceeds unnormalised words, at higher storage and time cost. It does not claim to beat stemmed words, so on English the gain over a Porter2-stemmed field is probably small. Effect size on your data UNVERIFIED; plausible +0.005 to +0.02 on French sets, mainly from morphology and compounds, around 0 on SciFact/NFCorpus.
- Cost: index roughly 2 to 3 times larger (UNVERIFIED), query clause count grows with query length, and the n-gram clause is a non-term-union risk for block-WAND speed only if you wrap it; two plain term unions fused by score sum can stay WAND-friendly. Code: low.
- FR/EN: both; most useful on French and on mixed-language chunks.
- Harness recipe: `bench run --extra-field ngram4 --ngram-weight {0.3,0.6,1.0}`.

### E6. Adjacent-pair phrase SHOULD clauses (sequential-dependence style)
- What to change: after the existing OR of terms, add, for each adjacent pair of analysed query terms, `BoostQuery(PhraseQuery(pair).set_slop(1 or 2), w)` as SHOULD with w in {0.1, 0.2, 0.4}. Skip pairs made of two very frequent terms. Positions must be indexed.
- Expected gain: the sequential dependence / MRF model (Metzler and Croft, SIGIR 2005) is the classical justification; I could only retrieve bibliographic data in this session, not its effect sizes: UNVERIFIED. I found no BEIR-style number for BM25 plus proximity. Short abstracts (SciFact, NFCorpus) and bag-of-words natural-language questions (LongMemEval) suggest small gains, 0 to +0.01; best chance is on Alloprof/Syntec where queries are phrases. Treat as unproven.
- Cost: query latency up, probably much more than the clause count suggests because a non-term SHOULD clause moves the union off the block-WAND path (see section 0); size UNVERIFIED. Index: none if positions already stored. Code: low.
- FR/EN: both.
- Harness recipe: `bench run --prox-bigram-boost {0,0.1,0.2,0.4} --slop {0,1,2}` and record p95 latency.

### E7. BGE-M3 sparse weights (only if BGE-M3 already produces your dense vectors)
- What to change: take the sparse "lexical weights" (ReLU of a linear head on token states, max per token id) from the same forward pass used for dense, store them as sparse postings as in E1. fastembed-rs 7.7.0 exposes `Bgem3Embedding` (dense + sparse + ColBERT in one pass, quantised variant `BGEM3Q`, max length up to 8192).
- Expected gain: BGE-M3 paper, MIRACL nDCG@10: sparse 53.9 average versus BM25 31.9 (XLM-R tokenizer) or 38.5 (Lucene analyzers); French column 35.3 versus 11.5 (subword BM25), so the fair gap is smaller, see E1. On long documents (MLDR, up to 8192 tokens) sparse 62.2 is below Lucene-analyzer BM25 64.1: the advantage shrinks with long text; your chunks (about 500 tokens) sit between. No BEIR/SciFact/NFCorpus number verified for BGE-M3 sparse.
- Cost: 568M-parameter model, heaviest option; if the dense stage already runs it, the sparse side is almost free at index time; otherwise too heavy for this purpose. Licence: I believe MIT, not re-verified in this session. Code: medium.
- FR/EN: strong multilingual coverage (100+ languages claimed).
- Harness recipe: `bench run --retriever sparse_bgem3` against E1 and BM25 on the same chunks.

### E8. Offline chunk enrichment (contextual prefix or extracted facts)
- What to change: before indexing, prepend to each chunk generated context (document title, section path, a one or two sentence situating blurb, or extracted user facts for chat sessions) produced offline by a local small LLM. Query side unchanged. No LLM at query time.
- Expected gain: Anthropic Contextual Retrieval: contextual embeddings plus contextual BM25 cut the top-20 failure rate by 49% (5.7% to 2.9%), 67% with reranking (1.9%); the post does not isolate the BM25-only contribution: UNVERIFIED. LongMemEval: fact-augmented keys gave an average +9.4% recall@k and +5.4% final accuracy over flat indexing (extraction with Llama 3.1 8B Instruct; averaged across retrievers, not BM25-only). Your own title-boost null result suggests the cheap non-LLM part (titles) will not help on SciFact/NFCorpus.
- Cost: the highest of the list: one LLM pass per chunk at index time, and index growth by the prefix length. Code: medium, plus model ops.
- FR/EN: both if the model is bilingual.
- Harness recipe: `bench run --chunk-prefix {none,title,llm_ctx}` on LongMemEval-S and the two French sets; hold the chunker constant.

### E9. Query-side hygiene: dedup, cap repeats, language-aware stop words
- What to change: (a) collapse repeated analysed query terms into one clause with weight 1 (or log(1+tf)); (b) optionally drop stop words only on the query side for natural-language questions (LongMemEval) using `StopWordFilter` per language; (c) `minimum_number_should_match` for long queries (0.27 exposes `BooleanWeight::with_minimum_number_should_match`; public query builder name should be checked).
- Expected gain: small and uncertain. Anserini counts repeated query terms as separate instances, which materially changed BM25 on BRIGHT's long queries (Anserini/BRIGHT reproducibility paper, effect between the two query generators described as significant; exact figure not extracted: UNVERIFIED). BM25S reports stop words have minimal average impact on BEIR in English. Because idf-pruning already hurt for you, expect within +-0.005 and run it mainly as a sanity check.
- Cost: none. Code: low.
- FR/EN: both; stop-word list is per language.
- Harness recipe: `bench run --query-dedup {off,on} --query-stop {off,en,fr,en+fr}`.

### E10. Fuzzy fallback for out-of-dictionary query terms
- What to change: for query terms absent from the term dictionary, add `FuzzyTermQuery::new(term, 1, true)` (prefix length 2) as SHOULD with a low boost.
- Expected gain: none on your six corpora (no typos in them), so nDCG@10 cannot justify it. Value is product robustness for real user typos. Do not judge it on this harness.
- Cost: small, automaton over the term dictionary only for rare OOV terms, but it removes the block-WAND path for those queries. Code: low.
- FR/EN: both.
- Harness recipe: inject a synthetic typo rate (1 edit in 10% of query terms) into all queries and compare `--fuzzy off/on`; report only the typo-injected run.

## 2. What the literature says will NOT help in your setting (skip)

1. BM25 variants (BM25+, BM25L, Robertson, ATIRE, BM25-adpt, TF-IDF). Kamphuis et al. (ECIR 2020), large reproducibility study on Robust04, Core17, Core18: Lucene default AP .2531 on Robust04 versus BM25+ .2526, BM25L .2542, Lucene accurate .2533; authors' conclusion: "does it matter? ... no". BM25S (2024) Table 3: most implementations average 39.7 to 40 across BEIR, differences attributed to tokenisation. Your fieldnorm quantisation is the same lossy trick as Lucene's and Lucene "accurate" differs by .0002 AP. Do not implement BM25+/L.
2. k1/b tuning, including BEIR's k1=0.9,b=0.4. You measured under 0.005; BM25S shows the same flatness across (1.2,0.75), (1.5,0.75), (0.9,0.4). Also requires a patched Tantivy.
3. BM25F or title-field boosting. Pyserini BEIR: multifield BM25 SciFact 0.665 versus flat 0.679, NFCorpus 0.325 versus 0.322, BEIR macro 0.429 versus 0.424. This matches your null result.
4. RM3 / lexical pseudo-relevance feedback. The BEIR authors report trying Anserini with RM3 and finding plain Anserini BM25 best. A 2026 preprint (arXiv 2609.37911) reports RM3 helping NFCorpus (0.3231 to 0.3465) but not TREC-COVID/SCIDOCS, so the literature is mixed; your measurement says skip, and I would not retry it.
5. Stop-word removal and stemmer fine-tuning for English. BM25S: stemming matters a little, stop words "minimal impact" on average. Gains there are inside your noise floor.
6. Doc2Query / doc2query-- document expansion. Doc2Query-- is evaluated on MS MARCO passage dev (RR@10 0.279 to 0.323 when keeping 30% of 80 expansions, index -33% at the best setting, but about 214 GPU-hours total at n=40 with filter). The generator is trained on English MS MARCO; I found no verified multilingual model or BEIR result for Doc2Query--. Poor fit for a local CPU product with a French half. Skip.
7. SPLADE-v3 as a shipped model. English only, and its model card carries a CC BY-NC-SA 4.0 notice (non-commercial). Use it only as an upper bound for English sparse quality (51.7 BEIR-13 average in its paper). Granite-embedding-30m-sparse (Apache-2.0, 30M, BEIR-13 average 50.8 per its card) is a cheaper English alternative to doc-v3-gte; English only.
8. Elastic ELSER. Runs as an Elasticsearch inference endpoint (`.elser_model_2`); not an embeddable local model. English-only and licence status UNVERIFIED. Skip.
9. Large rerankers on CPU (bge-reranker-v2-m3, 568M, or any LLM reranker). Not compatible with a tens-of-milliseconds budget on 2,000-character chunks.
10. Compound-word splitting (`SplitCompoundWords`). Targets German-style compounds; French/English do not benefit.
11. Per-query language detection to choose an analyzer. Short text is where detectors fail (single words 73.9%, word pairs 94.1% in the lingua README example). Analyse the query with both analyzers instead.
12. BM25S itself and Tantivy's block-max WAND. They change speed, not effectiveness.
13. Facet and filter queries. Irrelevant to relevance on these corpora.
14. Judging fuzzy matching on SciFact/NFCorpus/Alloprof/Syntec/LongMemEval. Those sets have no typo noise.

## 3. Suggested order of work

1. E4 diagnostic only (French vs English stemmer on pure-French corpora, 1 hour). It tells you whether the routing idea has any headroom.
2. E3 (chunk size and aggregation, no models).
3. E1 on English control first (doc-v3-gte via fastembed-rs), then multilingual-v1; this decides whether a sparse channel is worth shipping.
4. E2 with a hard latency budget.
5. Everything else only if E1/E2 leave a gap.

## 4. Source list (accessed 2026-11-13)

Tantivy and ecosystem
- Tantivy changelog (0.27.0, Upcoming Release, BM25/BMW/phrase/fuzzy/stemmer entries): https://raw.githubusercontent.com/quickwit-oss/tantivy/main/CHANGELOG.md
- Tantivy repo page (0.27 bump commit #3197, Oct 17, 2026): https://github.com/quickwit-oss/tantivy
- `src/query/bm25.rs` (K1=1.2, B=0.75, `Bm25StatisticsProvider`): https://raw.githubusercontent.com/quickwit-oss/tantivy/main/src/query/bm25.rs
- PR #1855 "Make BM25 scoring more flexible": https://github.com/quickwit-oss/tantivy/pull/1855
- `boolean_weight.rs` (TermUnion / Other dispatch, `for_each_pruning`): https://raw.githubusercontent.com/quickwit-oss/tantivy/main/src/query/boolean_query/boolean_weight.rs
- docs.rs tokenizer module (0.27.0): https://docs.rs/tantivy/0.27.0/tantivy/tokenizer/index.html ; StopWordFilter: https://docs.rs/tantivy/0.27.0/tantivy/tokenizer/struct.StopWordFilter.html ; Stemmer (frostem dependency): https://docs.rs/tantivy/latest/tantivy/tokenizer/struct.Stemmer.html
- tantivy-stemmers: https://crates.io/crates/tantivy-stemmers
- lingua-rs: https://github.com/pemistahl/lingua-rs ; whatlang vs lingua: https://www.greyblake.com/blog/whatlang-strikes-back/ ; whichlang announcement: https://www.reddit.com/r/rust/comments/13lptf6/opensourcing_whichlang_a_fast_language_detection/
- fastembed-rs (sparse, BGE-M3, inference-free, reranker list, v7.7.0): https://github.com/anush008/fastembed-rs

Lexical baselines and variants
- Kamphuis et al., "Which BM25 Do You Mean?" (ECIR 2020): https://pmc.ncbi.nlm.nih.gov/articles/PMC7148026/
- BM25S (arXiv 2407.03618): https://arxiv.org/abs/2407.03618 ; HTML with Table 3: https://arxiv.org/html/2407.03618v1
- Pyserini BEIR two-click reproductions (BM25 flat vs multifield, SPLADE-v3): https://castorini.github.io/pyserini/2cr/beir.html
- BEIR (reviewer thread on RM3 and Pyserini BM25): https://openreview.net/forum?id=wCu6T5xFjeJ
- Anserini/Pyserini BRIGHT baselines (repeated query terms): https://arxiv.org/html/2509.02558v1
- McNamee and Mayfield, character n-grams (Information Retrieval 7, 2004): https://link.springer.com/article/10.1023/B:INRT.0000009441.78971.be
- Metzler and Croft, MRF term dependencies (SIGIR 2005; effect sizes not read): https://dl.acm.org/doi/10.1145/1076034.1076115

Learned sparse
- SPLADE-v3 report (arXiv 2403.06789): https://arxiv.org/html/2403.06789v1 ; model card: https://huggingface.co/naver/splade-v3
- OpenSearch multilingual-v1: https://huggingface.co/opensearch-project/opensearch-neural-sparse-encoding-multilingual-v1
- OpenSearch doc-v3-gte: https://huggingface.co/opensearch-project/opensearch-neural-sparse-encoding-doc-v3-gte
- Granite embedding 30m sparse: https://huggingface.co/ibm-granite/granite-embedding-30m-sparse
- BGE-M3 paper (arXiv 2402.03216; Table 1 and Table 11): https://arxiv.org/html/2402.03216 ; model: https://huggingface.co/BAAI/bge-m3
- Sparseup-fr (French sparse, NanoBEIR-fr, 2026-10-12): https://huggingface.co/blog/alphaedge-ai/french-sparseup
- Generated query expansion with SPLADE-v3 (RM3 and BM25 NFCorpus numbers): https://arxiv.org/html/2609.37911v1
- Elastic hybrid retrieval (BM25 + ELSER weighted sum vs RRF): https://www.elastic.co/search-labs/blog/improving-information-retrieval-elastic-stack-hybrid
- ELSER docs: https://www.elastic.co/docs/explore-analyze/machine-learning/nlp/ml-nlp-elser

Rerankers
- mxbai-rerank-xsmall-v1: https://huggingface.co/mixedbread-ai/mxbai-rerank-xsmall-v1
- jina-reranker-v1-tiny-en: https://huggingface.co/jinaai/jina-reranker-v1-tiny-en
- cross-encoder mmarco-mMiniLMv2-L12-H384-v1: https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1
- Sentence-Transformers cross-encoder table (GPU docs/sec): https://www.sbert.net/docs/cross_encoder/pretrained_models.html

Document and chunk enrichment
- Doc2Query-- (arXiv 2301.03266): https://arxiv.org/html/2301.03266v3
- Anthropic Contextual Retrieval: https://www.anthropic.com/research/contextual-retrieval
- LongMemEval (arXiv 2410.10813): https://arxiv.org/html/2410.10813

## 5. Open uncertainty, in one place

- No latency figure for any model on your CPU was found; all CPU estimates above are UNVERIFIED.
- No published result of any learned sparse or reranker model on Alloprof, Syntec or LongMemEval-S was found.
- The block-WAND consequence of adding non-term SHOULD clauses is inferred from reading `boolean_weight.rs`, not measured.
- Effect sizes for SDM/proximity, per-chunk language routing, light French stemming, ASCII folding on French, and character 4-grams on your data are UNVERIFIED; the cited papers support the direction, not the size.
- Licences for BGE-M3, doc-v3-gte, mmarco-mMiniLM and jina-reranker-v2 were not confirmed on the model pages in this session (OpenSearch multilingual-v1, mxbai-xsmall and granite-30m-sparse were confirmed as Apache-2.0; SPLADE-v3 shows a CC BY-NC-SA notice).
- tantivy-stemmers 0.3.0 compatibility with tantivy 0.27 is not stated on its crates.io page.
