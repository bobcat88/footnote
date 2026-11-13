# Embedding boost: candidates to replace or add to bekko-a8m + gte-small

Research date: 2026-11-13. All facts below were fetched online on that date unless a revision/date is given. Nothing was downloaded (only model-card text, HF API metadata, and JSON score files).

## How to read the numbers

- "MTEB-repo" scores come from the `embeddings-benchmark/results` GitHub repo (`main` on 2026-11-13), per-task JSON, `test` split (else `dev`), `main_score` x100 = nDCG@10. The short hash after a model is the results revision used (the one with the most tasks). This repo mixes runs by the MTEB team and submissions by model authors, so treat every number as **self-reported unless proven otherwise**. MTEB task versions differ per run (the repo does not carry a single leaderboard version). The Alloprof/Syntec values will not match Footnote's own harness (a8m: MTEB-repo Alloprof 45.0 / Syntec 80.3 vs your blend 0.478 / 0.795), so compare **deltas between models**, not absolute values.
- "HF-card" = self-reported `model-index` on the Hugging Face card.
- "AP" = active (non-embedding-table) parameters. This is what drives CPU cost, not total parameters. Taken from the bekko paper Table (arXiv 2607.25180) when available; "derived" means I computed it from config.json (hidden size, layers, vocab).
- UNVERIFIED = no source found, or an extrapolation by me.
- Baseline rows for your current pair (MTEB-repo): a8m `a8cedb6b`: SciFact 70.0, NFCorpus 33.2, Alloprof 45.0, Syntec 80.3, BSARD 58.6, XPQA fra-fra 58.2. gte-small (`thenlper/gte-small`): SciFact 72.7 and NFCorpus 34.8 (HF-card), Alloprof 36.4, Syntec 59.1, BSARD 42.8 (MTEB-repo `17e1f347`).

## Key finding before the shortlist: the latency budget decides everything

Query encoding must stay in single-digit ms per model. CPU cost scales with AP, and published CPU throughput exists for only some models (bekko card, Ryzen 9 7950X, OpenVINO, batch 64, NQ docs up to 512 tokens, docs/s):

| Model | AP | docs/s | Relative cost vs a25m |
|---|---|---|---|
| bekko-a8m | 7.7M | 364 | 0.37x |
| multilingual-e5-small | 21.6M | 226 | 0.6x |
| bekko-a25m | 24.9M | 134 | 1x |
| granite-97m-multilingual-r2 | 28.3M | 125 | 1.07x |
| granite-311m-multilingual-r2 | 110.3M | 38 | 3.5x |
| multilingual-e5-large | 303.9M | 21 | 6.4x |

(bekko-a8m/a25m card and arXiv 2607.25180 section 7.2. Harrier-270m, EmbeddingGemma-300M, arctic-m-v2 and gte-multilingual-base were **not** measured in that protocol.)

Extrapolation (UNVERIFIED, mine): Harrier-270m (AP 100.3M), EmbeddingGemma-300M (106.3M), arctic-embed-m-v2.0 (113M) and gte-multilingual-base (113.3M) sit within about 10% of granite-311m-r2's AP, so expect roughly 3 to 4x the cost of a25m, and about 10x a8m. For a 16-token query that is plausibly 10 to 30 ms on a laptop with ORT, i.e. borderline or over the single-digit target. A vendor anchor: LiquidAI reports p50 7.3 ms / p95 9.6 ms for 32-token queries on a MacBook Pro M4 Max with llama.cpp fp16 for a 354M model (AP about 290M, derived), and does not say CPU or GPU (LFM2.5 card, fetched 2026-11-13). **The first test for every 100M-AP candidate is a latency measurement on your laptop, before any quality run.**

Indexing cost scales the same way: 1M chunks of about 500 tokens with a 100M-AP model costs about 3 to 4x a 25M-AP model's index time and about 10x a8m's (UNVERIFIED, proportional to AP). Plan for a one-off re-embed per model per corpus.

Runtime note: the bekko card claims OpenVINO is about 6.1x faster than ONNX Runtime on the Ryzen 9 and 1.9x on a Pi 5 for its models. If ORT is not a hard constraint, the OpenVINO backend is worth one measurement.

## 1. Ranked shortlist (max 6)

Order = expected quality gain over the current pair, adjusted for whether it can meet the latency and ops constraints. Items 1, 2 and 4 are the "heavy" class and may fail the latency gate; items 3 and 6 are cheap drop-ins.

### 1. google/embeddinggemma-300m (EmbeddingGemma 1)

- Why: the best verified single-model numbers among everything small enough to consider: SciFact 78.7, NFCorpus 39.2, Alloprof 59.1 (MTEB-repo `64614b0b`) versus a8m 70.0 / 33.2 / 45.0. Syntec, BSARD, Mintaka and XPQA fr: not in the repo, UNVERIFIED. Card (2026-11-13): MMTEB v2 task-mean 61.15, English v2 69.67. Matryoshka-trained at 768/512/256/128.
- Repo and file: `onnx-community/embeddinggemma-300m-ONNX` (ungated mirror, lastModified 2025-09-04). Use `onnx/model.onnx` + `onnx/model.onnx_data` (0.5 MB + 1234.5 MB, fp32). Both files must sit in the same folder. Smaller option: `onnx/model_quantized.onnx` + `_data` (0.6 MB + 308.9 MB, dynamic int8), see failure mode. Source repo `google/embeddinggemma-300m` is gated (manual approval).
- Prefixes: query `task: search result | query: {text}`; document `title: none | text: {text}` (use the real title if you have one; the card says it helps).
- Pooling: mean pooling plus two Dense layers plus L2 normalisation, built into the ONNX graph; read the `sentence_embedding` output. The ONNX card's Python example unpacks two outputs and uses the second, so check the output order in your runtime.
- Dimension to store: 256 (MRL-trained). Card, task-mean: multilingual 61.15 / 60.71 / 59.68 / 58.23 at 768/512/256/128; English 69.67 / 69.18 / 68.37 / 66.66. That is -2.4% and -1.9% at 256.
- Max length 2048 tokens. License: Gemma Terms of Use (not OSI; flow-down use restrictions, check them against Footnote's distribution model; UNVERIFIED legal read).
- Most likely failure: it does not meet single-digit ms on a query (AP 106M), or it is made slower by int8. Activations do not support fp16 (NaN or garbage); a community dynamic-int8 export (a different repo, `cookieshake/embeddinggemma-300m-onnx`) reports cosine drift about 0.016 vs fp32 and a MatMulNBits int8 export that can be slower than fp32 per query (AmarettoLabs card). Keep fp32 weights and, if memory matters, quantise only the embedding table (see section 3).

### 2. microsoft/harrier-oss-v1-270m

- Why: released 2026-03-30, MIT. SciFact 75.4, NFCorpus 36.1, Alloprof 52.2, Syntec 83.5, BSARD 61.7, XPQA fra-fra 64.8 (MTEB-repo `31de22b6`): +5.4 / +2.9 / +7.2 / +3.2 vs a8m. MMTEB v2 mean 66.5 (card), MMTEB retrieval 66.4 (bekko paper), the highest of any model below 300M in that table. Distilled from larger embedders.
- Repo and file: `onnx-community/harrier-oss-v1-270m-ONNX` (created 2026-03-30, modified 2026-04-10). `onnx/model_quantized.onnx` + `_data` (0.2 MB + 343.7 MB) or fp32 `onnx/model.onnx` + `_data` (0.2 MB + 1105.9 MB). Output used by the card: `sentence_embedding`.
- Prefixes: query `Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: ` (the card's `web_search_query`; a custom one-sentence task instruction is allowed, so test "Given a search query over technical notes, retrieve relevant note passages"); documents get no prefix. Dropping the query instruction degrades quality (card FAQ).
- Pooling: last non-padding token plus L2. If you pool yourself, handle padding side: the reference code assumes right padding unless the last mask column is all ones.
- Dimension to store: 640 native. **No Matryoshka training is mentioned**, so truncating to 256/128 is untested; budget 640 B/vector at int8 or test truncation yourself.
- Max 32,768 tokens, 94 languages (card tag), license MIT.
- Most likely failure: the ONNX int8 file shifts the last-token embedding more than a mean-pooled encoder would, and cost is 3 to 4x a25m (AP 100.3M, extrapolated), so it may miss the latency budget. Also no MRL, so it cannot go below about 640 dims without a quality hit you would have to measure.

### 3. ibm-granite/granite-embedding-small-english-r2 (replace the gte-small slot)

- Why: same cost class as mE5-small/a25m (47.7M total, 12 layers, hidden 384, same transformer shape as granite-97m-r2, so AP is about 28M, derived) but clearly better English retrieval than gte-small: SciFact 75.5 vs 72.7, NFCorpus 37.1 vs 34.8 (MTEB-repo `54a8d261` vs HF-card). Native 384 dims, so no code change. Apache-2.0, 8192 tokens, no prefixes.
- Repo and file: `onnx-community/granite-embedding-small-english-r2-ONNX` (created 2025-09-26): `onnx/model.onnx` + `_data` (0.2 MB + 194.8 MB fp32); also fp16 97.4 MB, q8 51.9 MB. Official `ibm-granite/granite-embedding-small-english-r2` has no ONNX files.
- Prefixes: none. Pooling: CLS (official `1_Pooling/config.json`), then L2. The ONNX example reads `sentence_embedding` and normalises it.
- Dimension to store: 384 (no Matryoshka). English only.
- Most likely failure: it is English-only, so French notes and French queries get nothing from this slot (a8m must carry French), and ModernBERT-family models lose quality under ORT dynamic int8 (see section 3); use fp32 or at most the fp16 file.

### 4. Snowflake/snowflake-arctic-embed-m-v2.0

- Why: multilingual, Apache-2.0, trained for compression. SciFact 72.3, NFCorpus 35.9, Alloprof 55.2 (MTEB-repo `f2a7d59d`); its larger sibling l-v2.0 gets Syntec 88.5, BSARD 63.1, XPQA fra-fra 77.3 (`edc2df7b`), which suggests m-v2.0 is strong on French, but its own Syntec number is UNVERIFIED. Card: BEIR(15) 55.4, MIRACL 55.2, CLEF 53.9 (self-reported).
- Repo and file: `Snowflake/snowflake-arctic-embed-m-v2.0` (modified 2025-04-24), in-repo `onnx/model_int8.onnx` (310.9 MB) or `onnx/model.onnx` (1226.1 MB fp32). Also `onnx/model_fp16.onnx` 613.3 MB. Note the bnb4 and q4 files are 837 to 844 MB, so they are not smaller on disk.
- Prefixes: query `query: `; documents none. Pooling: CLS, then L2.
- Dimension to store: 256. Card: MRL 256 loses -1.81% BEIR, -2.17% MIRACL, -2.13% CLEF-focused, -3.06% CLEF-full; "MRL for this model is 256 dimensions" and 128-byte vectors need 4-bit quantisation (FAISS pq256x4fs).
- AP 113M (card), max 8192 tokens (built on gte-multilingual-base).
- Most likely failure: latency (AP 113M, the heaviest of the four) and the in-repo int8 file degrading as other dynamic-int8 exports do.

### 5. jinaai/jina-embeddings-v5-text-nano (retrieval adapter), only if the licence allows

- Why: SciFact 75.8, NFCorpus 38.7 (MTEB-repo `4c1f9846`); paper Table 4 nDCG@10: BEIR 56.06, MTEB-multilingual retrieval 63.26, MTEB-English 58.80. EuroBERT-210M base (French is one of its languages), distilled from Qwen3-Embedding-4B. Matryoshka dims 32/64/128/256/512/768. French-specific scores (Alloprof, Syntec): UNVERIFIED (none in the repo).
- Repo and file: `jinaai/jina-embeddings-v5-text-nano-retrieval` (the multi-task repo is not the one for ONNX use). `onnx/model_quantized.onnx` + `_data` (0.1 MB + 247.0 MB) or fp32 `onnx/model.onnx` + `_data` (849.2 MB). Community int8 export `electroglyph/jina-embeddings-v5-text-nano-retrieval-onnx` (218 MB) exists but has 51 downloads and no licence field; avoid.
- Prefixes: query `Query: `, document `Document: ` (from `config_sentence_transformers.json`). Pooling: last token (`1_Pooling/config.json`), then L2. Store 256.
- **License: CC-BY-NC-4.0**; commercial use requires contacting Jina. Decide this first. AP is about 85M (derived).
- Most likely failure: the licence, then latency; also `trust_remote_code` and a custom `jina_embeddings_v5` architecture make ONNX parity checks necessary.

### 6. ibm-granite/granite-embedding-97m-multilingual-r2 (cheap multilingual add-on)

- Why: same cost as a25m (AP 28.3M, 125 docs/s), 384 native dims, no prefixes, Apache-2.0, 32k context. Best MMTEB retrieval at its size (60.3 vs a8m 56.2, a25m 57.5, mE5-small 50.9; bekko paper Table 2). But per-task numbers are weaker than a8m on the English sets: SciFact 68.7, NFCorpus 29.4, Alloprof 47.3 (MTEB-repo `c61e626a`). Expect it to help French, long documents and diversity, not English BEIR. Cross-lingual: Belebele 52.9, MLQA 60.5 (IBM blog, 2026-05-14).
- Repo and file: `onnx-community/granite-embedding-97m-multilingual-r2-ONNX` (2026-06-21): `onnx/model.onnx` 390.0 MB (fp32). The int8 files (97.9 MB) are the likely loser: you already measured granite int8 as degraded.
- Prefixes none, pooling CLS + L2, store 384, no MRL.
- Most likely failure: it is no better than a8m on your English sets and only changes the blend marginally, as a25m did (+0.008, n.s.).

Cheap sanity option, not ranked: swap `bekko-embedding-v1-a8m` for `a25m` only if you need long-document gains (already measured as not significant).

## 2. All candidates considered

Columns: AP; native dim; MRL (dims trained); max seq; languages; license; ONNX; MTEB-repo scores (SciFact / NFCorpus / Alloprof / Syntec / BSARD / XPQA fra-fra), UNVERIFIED where blank; prefixes; pooling.
"n/a" = no ONNX found by me. "-" = not reported anywhere I could find (UNVERIFIED).

| Model (HF repo) | Params total / AP | Dim; MRL dims | Max seq | Langs | License | ONNX (repo; sizes) | SciFact / NFC / Alloprof / Syntec / BSARD / XPQA-fr | Prefixes | Pooling |
|---|---|---|---|---|---|---|---|---|---|
| bekko-embedding-v1-a8m (hotchpotch) [current] | 106M / 7.7M | 384; 384/256/128/64 | 8192 | multi | MIT | in-repo: model.onnx 130 MB | 70.0 / 33.2 / 45.0 / 80.3 / 58.6 / 58.2 | none | mean |
| gte-small (thenlper; Xenova ONNX) [current] | 33M / ~21M (derived) | 384; none | 512 | EN | MIT (UNVERIFIED) | Xenova/gte-small int8 33.8 MB | 72.7 / 34.8 (HF-card) / 36.4 / 59.1 / 42.8 / 55.5 | none | mean |
| bekko-embedding-v1-a25m | 123M / 24.9M | 384; 384/256/128/64 | 8192 | multi | MIT | in-repo model.onnx 199 MB | 72.3 / 34.3 / 45.4 / 81.6 / 61.7 / 59.9 | none | mean |
| bekko-static-embedding-v1 | 205M / 0 | 1024; 1024/512/256/128/64/32 | 32k tokenizer cap | 40 incl. FR | MIT | in-repo int8 per dim: 1024: 205.6 MB, 256: 52.0 MB, 128: 26.4 MB | 66.9 / 32.5 / 37.0 / 75.7 / 50.9 / 60.5 | none | mean of token vectors (needs the bundled `bekko_static_backend`) |
| **EmbeddingGemma-300M** (google) | 308M / 106.3M | 768; 768/512/256/128 | 2048 | 100+ | Gemma Terms | onnx-community: fp32 1234.5 MB, q8 308.9 MB, q4 196.7 MB, fp16 617 MB (fp16 invalid) | 78.7 / 39.2 / 59.1 / - / - / - | `task: search result \| query: ` / `title: none \| text: ` | mean + 2 Dense |
| EmbeddingGemma 2 (google/embeddinggemma-2, 2026-09-14) | 270M text backbone (740M with vision+audio) / ~130M | 768; 768/512/256/128 | 8192 | 100+ | Apache-2.0 | onnx-community/embeddinggemma-2-ONNX: text model.onnx 1084 MB, q8 313.7 MB, ungated | no SciFact/NFC/Alloprof in repo; on 20 common retrieval tasks mean 62.87 vs EG1 63.05 | same as EG1 | mean + proj |
| **harrier-oss-v1-270m** (microsoft, 2026-03-30) | 268M / 100.3M | 640; none stated | 32768 | 94 | MIT | onnx-community: q8 343.7 MB, fp32 1105.9 MB | 75.4 / 36.1 / 52.2 / 83.5 / 61.7 / 64.8 | query `Instruct: ...\nQuery: `; doc none | last token |
| harrier-oss-v1-0.6b | 596M / ~440M (derived) | 1024; none | 32768 | 94 | MIT | n/a (community) | 76.3 / 37.2 / 58.6 / 85.0 / 66.2 / 67.7 | same | last token |
| bitnet-embedding-270m (microsoft, 2026-07-15) | 270M / ~100M | 640 | 32768 | multi | MIT | none (GGUF + bitnet.cpp only) | MTEB v2 mean 66.26 vs harrier 66.5 | query instruction | last token |
| Qwen3-Embedding-0.6B (Alibaba) | 596M / ~440M (derived) | 1024; 32..1024 user-defined (no per-dim numbers on card) | 32768 | 100+ | Apache-2.0 | onnx-community/Qwen3-Embedding-0.6B-ONNX: int8 613.5 MB, fp16 1.2 GB | 69.7 / 36.7 / 58.7 / 85.1 / 57.7 / 69.2 | query `Instruct: {task}\nQuery:{q}`; doc none | last token |
| jina-embeddings-v5-text-nano (2026-02-18) | 212M (239M quoted) / ~85M | 768; 32/64/128/256/512/768 | 8192 | multi (EuroBERT 15 langs) | CC-BY-NC-4.0 | jinaai/...-nano-retrieval: q8 247.0 MB, fp32 849.2 MB | 75.8 / 38.7 / - / - / - / - | `Query: ` / `Document: ` | last token |
| jina-embeddings-v5-text-small | 596M (677M quoted) / ~440M | 1024 | 32768 | multi | CC-BY-NC-4.0 | n/a (GGUF/MLX; ONNX not checked) | 76.5 / 39.8 / - / - / - / - | same | last token |
| jina-embeddings-v3 | 572M / ~300M (UNVERIFIED) | 1024; MRL (UNVERIFIED dims) | 8192 | 89 | CC-BY-NC-4.0 (UNVERIFIED) | not checked | 72.5 / 36.6 / 54.4 / 83.9 (HF-card) / 64.4 (HF-card) / - | task adapters | mean |
| jina-embeddings-v4 | ~3.8B (UNVERIFIED) | 2048 | 32768 | multi | not checked | not checked | 76.1 / 34.4 / 61.0 / - / - / - | task | - |
| nomic-embed-text-v1.5 | 137M / ~110M | 768; 768..64 | 8192 | EN | Apache-2.0 | yes (not size-checked) | 70.3 / 34.7 (HF-card) / 28.5 / 40.6 / 21.2 / 52.4 | `search_query: ` / `search_document: ` | mean |
| nomic-embed-text-v2-moe | 475M / 305M active | 768; 768..256 | 512 | ~100 | Apache-2.0 | community only (`netrias/...`, `keisuke-miyako/...-int8`, tiny download counts) | - / - / - / - / - / - (AILA only 28.8 / 28.6); BEIR 52.86, MIRACL 65.80 (card) | `search_query: ` / `search_document: ` | mean |
| arctic-embed-m-v2.0 (Snowflake) | 305M / 113M | 768; 256 only | 8192 | 74 | Apache-2.0 | in-repo: int8 310.9 MB, fp32 1226 MB, fp16 613 MB | 72.3 / 35.9 / 55.2 / - / - / - | `query: ` / none | CLS |
| arctic-embed-l-v2.0 | 568M / 303M | 1024 | 8192 | 74 | Apache-2.0 | in-repo: int8 569.7 MB | 70.9 / 35.1 / 54.8 / 88.5 / 63.1 / 77.3 | `query: ` | CLS |
| arctic-embed-m-v1.5 (EN) | 109M / 86M | 768; 256 | 512 | EN | Apache-2.0 | not checked | 71.8 / 36.2 / 32.0 / 63.1 / 38.7 / 55.2 | `Represent this sentence...` (UNVERIFIED exact) | CLS |
| multilingual-e5-small | 118M / 21.6M | 384 | 512 | 94 | MIT | many community | 67.6 / 31.1 / 27.4 / - / - / - | `query: ` / `passage: ` | mean |
| multilingual-e5-base | 278M / 86M | 768 | 512 | 94 | MIT | many | 69.7 / 32.5 / 34.4 / 82.9 / 55.4 / 59.6 | same | mean |
| multilingual-e5-large(-instruct) | 560M / 304M | 1024 | 512 | 94 | MIT | many | instruct: 71.6 / 36.3 / 52.1 / 87.8 / 66.2 / 72.7 | `Instruct: ..\nQuery: ` for instruct | mean |
| bge-m3 (BAAI) | 568M / 312M | 1024 | 8192 | 100+ | MIT | not checked | 64.4 / 31.4 / 49.0 / - / - / - | none | CLS |
| bge-small-en-v1.5 | 33M / ~21M | 384 | 512 | EN | MIT | yes | 71.3 / 34.3 / 30.8 / 60.7 / 40.5 / 51.8 | query instruction (optional) | CLS |
| bge-base-en-v1.5 | 109M | 768 | 512 | EN | MIT | yes | 74.3 / 37.4 / 31.3 / 63.7 / 36.0 / 51.8 | same | CLS |
| gte-multilingual-base | 305M / 113.3M | 768; (elastic, UNVERIFIED) | 8192 | 70+ | Apache-2.0 | onnx-community: int8 340.3 MB | 73.4 / 36.4 / 53.6 (HF-card) / 83.0 (HF-card) / 62.6 / 67.3 | none | CLS |
| gte-modernbert-base | 149M / ~85M (derived) | 768 | 8192 | EN | Apache-2.0 | community only (`keisuke-miyako/...`) | 76.4 / 34.3 / - / - / - / - | none | CLS (UNVERIFIED) |
| granite-embedding-small-english-r2 | 47.7M / ~28M | 384; none | 8192 | EN | Apache-2.0 | onnx-community: fp32 194.8 MB, q8 51.9 MB | 75.5 / 37.1 / - / - / - / - | none | CLS |
| granite-embedding-english-r2 | 149M / ~85M | 768 | 8192 | EN | Apache-2.0 | onnx-community | 75.8 / 37.5 | none | CLS |
| granite-embedding-97m-multilingual-r2 | 97M / 28.3M | 384; none | 32768 | 200+ pretrain, 52 retrieval-tuned incl. FR | Apache-2.0 | onnx-community: fp32 390 MB, int8 97.9 MB | 68.7 / 29.4 / 47.3 / - / - / - | none | CLS |
| granite-embedding-311m-multilingual-r2 | 312M / 110.3M | 768; 768/512/384/256/128 | 32768 | same | Apache-2.0 | onnx-community: fp32 1247 MB, int8 312.6 MB | 70.8 / 30.8 / 52.1 / - / - / - | none | CLS |
| granite-embedding-278m / 107m multilingual (R1) | 278M / 107M | 768 / 384 | 512 | 12 | Apache-2.0 | in-repo (278m: 1112 MB) | 65.8 / 28.9 / 44.7 / 81.1 / 56.8 / 58.2 (278m); 63.3 / 27.0 / 38.9 / 79.7 / 53.6 / 53.0 (107m) | none | CLS |
| mxbai-embed-large-v1 | 335M | 1024; 2d/MRL | 512 | EN | Apache-2.0 | yes | 73.9 / 38.7 / 36.6 / 64.0 / 41.0 / 52.0 | `Represent this sentence for searching relevant passages: ` | CLS |
| mxbai-embed-xsmall-v1 | 22.7M | 384 | 512 | EN | Apache-2.0 | yes | 65.8 / 32.0 (HF-card) | same prefix | CLS |
| modernbert-embed-base (nomic) | 149M | 768; MRL | 8192 | EN | Apache-2.0 | not checked | 69.6 / 33.4 | `search_query: ` / `search_document: ` | mean |
| voyage-4-nano (voyageai, 2026-01-06, modified 2026-10-22) | 346M / 180M | 2048; 2048/1024/512/256 | 32000 | multi | Apache-2.0 | none (GGUF only) | no SciFact/NFC/Alloprof in repo; MIRACL/MKQA/Belebele only | `Represent the query for retrieving supporting documents: ` / `Represent the document for retrieval: ` | mean |
| LFM2.5-Embedding-350M (LiquidAI, 2026-05-05) | 354M / ~290M | 1024; none | 512 | 11 incl. FR | LFM Open License v1.0 | none (GGUF only) | no SciFact/Alloprof; NanoBEIR-multilingual-ext avg 0.577 (card) | `query:` / `document:` | CLS |
| F2LLM-v2-80M / 160M / 330M (codefuse-ai) | 80M / 159M / 334M | 320 / 640 / 896 | 40960 | multi | Apache-2.0 | none found | 45.6 / 29.0 / 39.5 / 75.7 / 42.8 / 64.7 (80M); 48.8 / 30.3 / 43.2 / 76.2 / 51.4 / 68.6 (160M); 51.4 / 32.6 / 51.4 / 81.5 / 55.9 / 73.0 (330M) | UNVERIFIED | last token (UNVERIFIED) |
| F2LLM-v2-0.6B | 0.6B | - | 40960 | multi | Apache-2.0 | none found | 54.3 / 34.3 / 55.5 / 85.2 / 60.4 / 74.7 | UNVERIFIED | - |
| pplx-embed-v1-0.6b (perplexity) | 0.6B | - | - | multi | not checked | not checked | only AILAStatutes 59.8 in repo | UNVERIFIED | - |
| Octen-Embedding-0.6B | 0.6B (Qwen3-based, UNVERIFIED) | - | - | multi | not checked | not checked | only AILA 58.9 / 86.6 in repo | UNVERIFIED | - |
| KaLM-embedding-multilingual-mini-instruct-v2.5 | ~0.5B (UNVERIFIED) | - | - | multi | not checked | not checked | 74.4 / 37.1 | instruct | - |
| Lajavaness/bilingual-embedding-small / base | small ~118M (UNVERIFIED) | 384 / 768 | - | FR+EN | not checked | not checked | 64.0 / 28.9 / 39.2 / 78.2 / 50.0 / - (small); 68.5 / 32.6 / 41.3 / 79.7 / 51.4 (base) | - | - |
| OrdalieTech/Solon-embeddings-mini-beta-1.1 | - | - | - | FR | not checked | not checked | - / - / 45.3 / 78.7 / 59.5 / 65.7 | - | - |
| paraphrase-multilingual-MiniLM-L12-v2 | 118M | 384 | 128 | 50+ | Apache-2.0 | yes | 48.5 / 23.4 / 26.6 / 65.5 / 37.8 / 42.5 | none | mean |
| sentence-transformers/static-similarity-mrl-multilingual-v1 | static | 1024 MRL | - | 50+ | Apache-2.0 (UNVERIFIED) | not checked | 49.6 / 22.8 / 26.3 / 65.9 / 34.7 / 51.6 | none | mean |
| minishlab/potion-multilingual-128M | static | 256 | - | 101 | MIT (UNVERIFIED) | not checked | 45.1 / 17.3 / 22.3 / 61.1 / 32.4 / - | none | mean |
| amaretto-embed-148m (AmarettoLabs, vocab-pruned EmbeddingGemma) | 148M / ~106M | 768; 512/256/128 | 2048 | EN, ES, PT, FR, DE, IT, NL, PL | Gemma Terms | in-repo: fp32 612 MB, int8 297 MB (MatMulNBits, ORT >= 1.20) | none reported; card claims about 99.3% of EG1 on retrieval/STS (vendor claim) | as EG1 | mean + Dense |

No newer BGE small/base exists as far as I could find (HF search of the `BAAI` account on 2026-11-13: the newest embedding-like repos are `bge-reasoner-embed-qwen3-8b-0923` and `llm-embedder`). Newer small multilingual models found: Harrier (2026-03), granite multilingual R2 (2026-04), jina v5 (2026-01/02), EmbeddingGemma 2 (2026-09), bekko (2026-07), bitnet-embedding (2026-07), LFM2.5-Embedding (2026-05), F2LLM-v2 (2026-03), voyage-4-nano (2026-01).

## 3. Dimension, quantisation and storage strategy

Current assumption in code: 384 dims. Models whose native dim is 384: bekko a8m/a25m, granite-97m-r2, granite-small-english-r2, gte-small, mE5-small. Everything in the heavy class (768/640) needs a configurable `dim`. A "384" slice of a 768-d model is only valid if 384 is a trained MRL dim: for EmbeddingGemma it is **not** (512/256/128), for granite-311m-r2 it is (768/512/384/256/128), for arctic-m-v2 only 256, for jina-v5-nano 256 and 512.

### Evidence collected

- Matryoshka truncation (relative loss vs full dim):
  - bekko a25m (HAKARI-Bench overall, card): 384 → 0.570; 256: -1.35%; 128: -6.17%; 64: -14.96%. a8m (paper, section 7.4): 256: -1.8%; 128: -7.0%; 64: -17.5%.
  - EmbeddingGemma-300M (card, task-mean): multilingual 61.15 → 59.68 at 256 (-2.4%), 58.23 at 128 (-4.8%); English 69.67 → 68.37 (-1.9%), 66.66 (-4.3%); code 68.76 → 66.74 / 62.96 (-2.9% / -8.4%).
  - arctic-embed-m-v2.0 (card): 256: -1.81% BEIR, -2.17% MIRACL, -2.13% / -3.06% CLEF.
  - jina-v5 paper: only a figure; text says quality drops noticeably below 256.
  - Harrier, granite-97m-r2, granite-small-en-r2: no MRL, no evidence.
- Scalar int8 and binary (vector quantisation, not weight quantisation):
  - bekko paper section 7.4, 384 dims: int8 without rescoring -5.5% (a8m), -2.4% (a25m); with fp32 rescoring of the top 100: -0.04% / -0.03%. Binary without rescoring -12.9% / -12.6%; with rescoring -0.44% / -0.38%. Binary at 64 dims without rescoring is unusable (-53% to -56%). The authors' recommended operating point: **256 dims + int8 + rescore**, noting the full-precision vectors must be kept for rescoring.
  - HF embedding-quantization blog (mxbai-embed-large-v1, MTEB retrieval 15 tasks): int8 97% (no rescore) to about 99% (rescore x4 to x5); binary 92.5% without, 96.45% with rescore x4. Model dependence is strong: binary+rescore retains 93.8% on all-MiniLM-L6-v2 (384d), 87.7% on nomic-embed-text-v1.5, 74.8% on e5-base-v2; int8 without rescore is 90.8% on MiniLM-384d and 94.7% on e5-base-v2. Small dims are more fragile, and calibration data matters for int8.
  - jina-v5 paper (ablation, not the release): binary costs -1.9 to -3.1 points on MTEB with a quantisation-friendly loss vs -3.1 to -3.9 without. voyage-4-nano advertises quantisation-aware int8/binary outputs (card; no numbers).
- Weight quantisation pitfalls (model files, not vectors):
  - bekko paper section 7.3: vocabulary-only row-wise int8 changes mean nDCG@10 by -0.0001 (100% retention), but quantising the Transformer weights costs -4.7% (OpenVINO qint8) and -6.2% (ORT qint8 AVX512), and long-document retrieval falls by about 24%; a25m's OpenVINO qint8 failed their consistency gate and a8m's produced NaN on ARM. This matches your own finding that granite and bge-m3 int8 files degrade.
  - EmbeddingGemma: activations cannot be fp16; dynamic int8 export drift about 0.016 cosine, and a model_int8 file can be slower than fp32 on some CPUs (community cards, not official).

### Recommendations

1. For the models you already run, keep fp32 transformer weights and shrink only the vocabulary table if size matters. That gives about -60% file size at no measured loss (bekko evidence; transferring this to Gemma/Harrier/jina is plausible because their tables are the bulk of the file, but UNVERIFIED).
2. Storage per vector (arithmetic): fp32 384d = 1536 B; int8 384d = 384 B; int8 256d = 256 B; int8 128d = 128 B; binary 384d = 48 B; int8 768d = 768 B; int8 640d = 640 B. Per 1M chunks and per model: 384 MB (int8 384), 256 MB (int8 256), 128 MB (int8 128), 48 MB (binary 384), 768 MB (int8 768). Two models at int8 256d = 512 MB per 1M chunks.
3. Default plan to test: **256d (or native 384d) int8 with a float query against int8 docs (asymmetric), on both models**, and compare with symmetric int8. Test whether asymmetric float-query/int8-doc scoring recovers most of the "int8 + rescore" gain without storing fp32 docs; the HF blog and bekko used rescoring against higher-precision vectors, so this is UNVERIFIED for your sets.
4. Do not go to 128d unless storage forces it. Measured losses at 128d are 4.3 to 8.4% (EmbeddingGemma) and 6 to 7% (bekko), which is bigger than any gain you are chasing. Never use 64d.
5. Binary is a first-stage filter only: use binary Hamming to take the top 100 to 400, then rescore with int8. With 384-d models binary alone loses 12 to 13% (bekko). At 48 B/vector per model it is attractive only when the corpus approaches 1M chunks and brute-force int8 scans are too slow; at 1M x 256d int8, a SIMD brute-force dot product per query is on the order of a few hundred MB read, so measure before adding a second stage.
6. For blends of different dims, each model keeps its own column and score; do not concatenate before normalising (UNVERIFIED but standard practice).

## 4. Rejected models and why

- Qwen3-Embedding-0.6B: AP about 440M (derived: 596M total minus a 155.6M vocab table), 4x the heavy class, so single-digit ms is out of reach. SciFact 69.7 is below EmbeddingGemma 78.7 and Harrier 75.4 despite 4x the compute; its strengths are French (Alloprof 58.7, Syntec 85.1, XPQA fra-fra 69.2). Official card has no ONNX; the onnx-community export exists (int8 613.5 MB). No smaller Qwen3-Embedding size exists.
- Harrier-oss-v1-0.6b, jina-v5-text-small, pplx-embed-v1-0.6b, Octen-0.6B, F2LLM-v2-0.6B, KaLM mini v2.5, LFM2.5-350M, voyage-4-nano: all AP above about 180M; same latency problem. In addition: jina v5 small is CC-BY-NC; LFM2.5 has a custom licence and no ONNX; voyage-4-nano has no ONNX and no SciFact/Alloprof/Syntec evidence (watch it for its quantisation-aware int8/binary training).
- Harrier bitnet-embedding-270m: needs bitnet.cpp/GGUF, not ORT; card says research use, not for production (research claim: MTEB v2 66.26 vs 66.5 for Harrier).
- EmbeddingGemma 2: on 20 retrieval tasks that appear in both results folders the mean is 62.87 vs 63.05 for EG1, so the text side gains nothing (code gains: CodeSearchNetCC 96.6 vs 73.7). Only reason to prefer it: Apache-2.0 and ungated, 8192 tokens. Test it only if the Gemma Terms are a blocker; no SciFact, NFCorpus or French retrieval scores exist for it.
- amaretto-embed-148m: vocab-pruned EG1 (compute unchanged), Gemma Terms, no published retrieval numbers, vendor-only claims.
- multilingual-e5-small/base/large and instruct: you measured the blend as worse; MTEB-repo confirms mE5-small (SciFact 67.6, Alloprof 27.4) and mE5-base (69.7 / 34.4) are below a8m. e5-large-instruct has very good French (Syntec 87.8, XPQA 72.7) but AP 304M.
- bge-m3: SciFact 64.4, NFCorpus 31.4, AP 312M, and you saw int8 degradation.
- bge-small/base-en-v1.5, mxbai-embed-large/xsmall, nomic-embed-text-v1.5, modernbert-embed-base: English only; French is weak (Alloprof 28.5 to 36.6, Syntec 40.6 to 64.0); xsmall SciFact 65.8, nomic 70.3, and gte-small (72.7) already matches or beats them at the same size. No newer BGE small exists.
- gte-modernbert-base: SciFact 76.4 but NFCorpus 34.3, English only, AP about 85M; granite-small-english-r2 is better on both at a third of the compute.
- nomic-embed-text-v2-moe: 475M total / 305M active (MoE), 512-token cap (your chunks are about 500 tokens), `trust_remote_code`, no official ONNX, no SciFact/NFC/French retrieval scores in the repo.
- jina-embeddings-v3 (CC-BY-NC, 570M) and v4 (about 3.8B): too big.
- arctic-embed-l-v2.0, gte-multilingual-base, granite-311m-r2: good (l-v2.0 Syntec 88.5), but AP 110 to 312M and no better than EmbeddingGemma/Harrier on your English sets; gte-multilingual-base is the only one worth a spot-check as an Apache alternative to arctic-m-v2 (SciFact 73.4 / NFC 36.4).
- granite-278m/107m R1 and paraphrase-multilingual-MiniLM: dominated (SciFact 48 to 66).
- F2LLM-v2-80M/160M/330M: strong French XPQA but very weak English retrieval (SciFact 45.6 to 51.4); no ONNX found.
- Lajavaness bilingual-embedding, Solon mini beta: French numbers at or below a8m, English retrieval unproven or lower.
- Static models (bekko-static-embedding-v1, static-similarity-mrl-multilingual-v1, potion): standalone retrieval is well below a8m (bekko-static SciFact 66.9, Alloprof 37.0; others lower still). Optional later: bekko-static as a near-free third signal (queries are a table lookup, ONNX int8 at 256d is 52 MB), but I have no evidence it adds anything to the blend.

## 5. Sources (all fetched 2026-11-13)

- MTEB per-task results repo: https://github.com/embeddings-benchmark/results (tree `results/<org>__<model>/<revision>/<Task>.json`)
- Bekko paper: https://arxiv.org/abs/2607.25180 (HTML https://arxiv.org/html/2607.25180), submitted 2026-07-28
- Bekko cards: https://huggingface.co/hotchpotch/bekko-embedding-v1-a25m , https://huggingface.co/hotchpotch/bekko-static-embedding-v1
- EmbeddingGemma: https://huggingface.co/google/embeddinggemma-300m , https://huggingface.co/onnx-community/embeddinggemma-300m-ONNX , paper https://arxiv.org/abs/2509.20354 (2025-09-24)
- EmbeddingGemma 2: https://huggingface.co/google/embeddinggemma-2 , https://huggingface.co/onnx-community/embeddinggemma-2-ONNX
- EmbeddingGemma community ONNX/int8 notes: https://huggingface.co/cookieshake/embeddinggemma-300m-onnx , https://huggingface.co/AmarettoLabs/amaretto-embed-148m-ONNX
- Harrier: https://huggingface.co/microsoft/harrier-oss-v1-270m , https://huggingface.co/onnx-community/harrier-oss-v1-270m-ONNX , https://huggingface.co/microsoft/bitnet-embedding-270m ; announcement https://blogs.bing.com/search/April-2026/Microsoft-Open-Sources-Industry-Leading-Embedding-Model
- Qwen3-Embedding: https://huggingface.co/Qwen/Qwen3-Embedding-0.6B , https://huggingface.co/onnx-community/Qwen3-Embedding-0.6B-ONNX
- Jina v5: https://huggingface.co/jinaai/jina-embeddings-v5-text-nano , https://huggingface.co/jinaai/jina-embeddings-v5-text-nano-retrieval , paper https://arxiv.org/abs/2602.15547
- Granite multilingual R2 blog (2026-05-14): https://huggingface.co/blog/ibm-granite/granite-embedding-multilingual-r2 ; cards https://huggingface.co/ibm-granite/granite-embedding-97m-multilingual-r2 , https://huggingface.co/onnx-community/granite-embedding-small-english-r2-ONNX , https://huggingface.co/onnx-community/granite-embedding-97m-multilingual-r2-ONNX
- Snowflake: https://huggingface.co/Snowflake/snowflake-arctic-embed-m-v2.0 , paper https://arxiv.org/abs/2412.04506
- nomic v2-moe: https://huggingface.co/nomic-ai/nomic-embed-text-v2-moe
- Voyage / Liquid: https://huggingface.co/voyageai/voyage-4-nano , https://huggingface.co/LiquidAI/LFM2.5-Embedding-350M
- Quantisation reference: https://huggingface.co/blog/embedding-quantization
- HF API metadata (file sizes, licences, dates, gating): `https://huggingface.co/api/models/<repo>?blobs=true`, `config.json`, `1_Pooling/config.json`, `config_sentence_transformers.json` of each repo above.

## Not verified (open items for the test plan)

- Per-query CPU latency of every heavy-class model on your laptop: no published number for EmbeddingGemma, Harrier, arctic-m-v2 or jina-v5-nano with ORT.
- Syntec/BSARD/Mintaka/XPQA for EmbeddingGemma-300M, jina-v5-nano and arctic-m-v2.0; Alloprof/Syntec for granite-small-english-r2 (English only, expect poor).
- Whether MTEB-repo entries for each model were produced by the MTEB team or by the authors.
- Legal fit of the Gemma Terms and of CC-BY-NC for Footnote's distribution.
- Whether truncating Harrier (no MRL) to 256 or 384 dimensions is usable.
