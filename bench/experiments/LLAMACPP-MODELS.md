# Test 5: stronger embedding models through llama.cpp (Footnote Library only)

> **Tokenizer qualification, 15 November:** subsequent tokenizer probing found
> that b11901 adds the terminal token from the Qwen3-Embedding-4B GGUF itself,
> while the lab's Qwen3 configuration also explicitly appends `<|endoftext|>`.
> The Qwen3-4B rows below therefore measure that double-token recipe; they do not
> establish the result of the model's standard single-token input. The server's
> token IDs matched the original tokenizer on three short fixtures without the
> extra suffix. Other Qwen3 configurations share the lab suffix and also need
> checking. Re-run with an isolated cache and validate more thoroughly before
> drawing a conclusion about the model itself. Existing receipts are preserved.
> The single-token dev recheck is recorded in
> [QWEN4-TERMINAL-TOKEN-RESULTS-2026-11-16.md](QWEN4-TERMINAL-TOKEN-RESULTS-2026-11-16.md).

Written 14 November 2026, before any run. Agent memory (Footnote Memory) is out of scope: it keeps today's pair.

## Why

EmbeddingGemma-300m already beat the shipped pair on held-out data (+0.04 to +0.07 nDCG@10, `MODEL-CONFIRMATION.md`), but failed the cost
limits on ONNX Runtime: 1.2 GB of files and single-query encoding up to 3.6 times the pair's. Its int8 and q4 ONNX exports were slower
still. llama.cpp runs GGUF quantized models with CPU kernels built for them, and offers a Vulkan backend for the AMD GPU that DirectML served
poorly (`GPU-ENCODING.md`). The question: does llama.cpp make a stronger model affordable?

## Setup, frozen

- **Runtime:** llama.cpp upstream build 11901 (commit 5320c87ff, Windows x64, Vulkan), run as `llama-server --embedding`, read-only from
  `<local-llamacpp-runtime>`. Queries on the CPU (`-ngl 0`). Documents once on the CPU and once on the GPU (`-ngl 99`, Radeon RX 6800).
- **Candidates**, each alone in the shipped hybrid (the M2 configuration: BM25 shortlist, z-score fusion, alpha 0.55, whole documents):
  - **C1** EmbeddingGemma-300m, `ggml-org/embeddinggemma-300M-GGUF`, Q8_0; prefixes `task: search result | query: ` and `title: none | text: `.
  - **C2** Qwen3-Embedding-0.6B, `Qwen/Qwen3-Embedding-0.6B-GGUF`, Q8_0; query prefix
    `Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: `, none for documents.
  - **C3** bge-m3, `ggml-org/bge-m3-Q8_0-GGUF`, Q8_0, no prefixes.
  - **C4** EmbeddingGemma 2 (September 2026, Apache 2.0, text part only: 270M), `ggml-org/embeddinggemma-2-GGUF`, Q8_0 (310 MB); prefixes
    `task: search result | query: ` and `title: none | text: `. Added the same day, before any result, after review pointed to it. Its card
    warns that float16 gives NaN or degraded vectors: GPU vectors are checked for that under condition 5.
  - **C5** EmbeddingGemma 2 in ONNX (`onnx-community/embeddinggemma-2-ONNX`, text model), on ONNX Runtime like the shipped pair, so it
    would need no llama.cpp: int8 (`model_quantized`, 314 MB) and fp32 (`model`, 1.08 GB). Same prefixes as C4.
  - **C6** Granite Embedding 311M Multilingual R2 (IBM, April 2026, Apache 2.0, claims 65.2 on MTEB multilingual retrieval), ONNX int8
    (`model_quint8_avx2`, 313 MB), on ONNX Runtime; no prefixes.
  Files are pinned by SHA-256 in the lab lock before the first run. Pooling and normalization as each model's GGUF metadata declares.
- **Baseline:** the shipped pair, existing receipts (`receipts/heldout/*-lab_models2_baseline-heldout` and the dev runs of `MODEL-CANDIDATES.md`).

## Hugging Face scan (14 November, before any run)

Embedding models created in 2026, sorted by downloads (feature-extraction and sentence-similarity). Kept: permissive license, French,
at most about 600M parameters for the text part, an ONNX or GGUF file from the authors or the ggml/onnx communities. Left out:
Jina v5 text nano and small (CC BY-NC: no commercial use), LFM2.5-Embedding and Nemotron-3-Embed (custom licenses; Nemotron is 1.1B),
Qwen3-VL-Embedding (2B and up), and the other 0.6B models (pplx-embed-v1, F2LLM-v2, Octen, harrier-0.6b, voyage-4-nano), for which
Qwen3-Embedding-0.6B stands in: same size class, the slowest kind of candidate. They can follow if a 0.6B model passes.

## Screening, to keep the machine free (small corpora first)

- **Stage A:** every candidate on the dev split of SciFact (English) and Alloprof (French), documents and queries on the CPU. A candidate
  goes on only if it gains at least +0.01 nDCG@10 over the shipped pair on at least one of the two.
- **Stage B:** survivors on the other dev corpus of their language group (NFCorpus or mix-fr-en), then conditions 1 to 5 below.

## Amendment (14 November, while stage A runs, before any result was read)

Scope note: for now Footnote is for personal use and the embedding model is a measure of ingestion speed and quality; for paying clients a
free model or a commercial licence from the best model's vendor are both possible, and the real size limit is what the GPU can run.
So:
- **Licence** is reported, no longer a filter. Added: **C7** Jina v5 text nano retrieval and **C8** Jina v5 text small retrieval
  (CC BY-NC 4.0, official `jinaai/*-retrieval-GGUF`), and **C9** Qwen3-Embedding-4B and **C10** Qwen3-Embedding-8B (Apache 2.0, official
  `Qwen/*-GGUF`, Q8_0: 4.3 and 8.0 GB). Stage A as above, with one difference: C9 and C10 encode documents and queries on the GPU
  (too slow for the CPU); Jina v5 uses its `Query: ` and `Document: ` prompts and last-token pooling.
- **Condition 4 (file size) is replaced** by two classes. **CPU class:** conditions 1 to 3 as written. **GPU class:** conditions 1 and 2,
  the model runs fully in at most 12 GB of VRAM (16 GB card, 4 GB kept free), and single-query encoding p95 **on the GPU** is at most 50 ms
  (Vulkan, `-ngl 99`, same 100 dev queries). Download size, resident RAM or VRAM, vector dimensions and index size are reported.

## Adopt a candidate for an opt-in "performance" profile only if every condition holds

1. **Quality, dev:** at least +0.01 nDCG@10 over the shipped pair on both dev corpora of a language group (English: SciFact, NFCorpus;
   French/mixed: Alloprof, mix-fr-en). Then frozen.
2. **Quality, held-out:** paired bootstrap 95 % interval (10,000 resamples) of the nDCG@10 difference above zero on each corpus of that group.
3. **Query cost:** single-query encoding p95 on the CPU at most twice the shipped pair's (sum of both models' p95 in `MODEL-LATENCY.md`:
   SciFact 8.0 ms, NFCorpus 4.1 ms, Alloprof 17.2 ms, mix-fr-en 16.9 ms), same 100 dev queries, measured through the resident server.
4. **Download:** model file at most 1.2 GB. (Raised from 600 MB for an opt-in profile that users choose knowing its cost; decided before results.)
5. **GPU indexing**, judged apart: an index built on the GPU may be queried on the CPU only if every document's GPU vector has cosine at least
   0.9999 with its CPU vector and the held-out nDCG@10 of the GPU-built index is within 0.002 of the CPU-built one. Otherwise GPU indexing is
   not offered for that model.

Measured and reported, not pass/fail: document throughput on CPU and GPU, resident RAM and VRAM of the server, vector dimensions (storage).

The shipped default does not change whatever the result: today's pair on ONNX Runtime stays the standard profile. A passing candidate becomes
a documented opt-in, wired into Rust (`llama-cpp-2`), then confirmed on the large corpus before any release. Every database using it is
re-embedded once.

## Stage A results (14 November)

Dev split, each candidate alone in the shipped hybrid, CPU only. Raw rows: `bench/sweeps/llamacpp.jsonl`. Seconds are the whole run
(encoding dominates); the shipped pair's vectors were cached, so it has no comparable time here (the shipped binary embeds the SciFact
corpus in 241 s, `BUILD-SPEED.md` test 4).

| Candidate | Runtime | SciFact nDCG@10 | Δ | Alloprof nDCG@10 | Δ | SciFact s | Alloprof s |
|---|---|---|---|---|---|---|---|
| Shipped pair (bekko-a8m + gte-small) | ONNX | 0.7624 | | 0.5036 | | | |
| C1 EmbeddingGemma 1 Q8_0 | llama.cpp | 0.8519 | +0.090 * | 0.5629 | **+0.059** | 255 | 179 |
| C2 Qwen3-Embedding-0.6B Q8_0 | llama.cpp | 0.7664 | +0.004 | 0.5727 | **+0.069** | 768 | 458 |
| C3 bge-m3 Q8_0 | llama.cpp | 0.7600 | −0.002 | 0.5265 | +0.023 | 336 | 200 |
| C4 EmbeddingGemma 2 Q8_0 | llama.cpp | 0.9536 | +0.191 * | 0.5314 | +0.028 | 319 | 238 |
| C5 EmbeddingGemma 2 int8 | ONNX | 0.9545 | +0.192 * | 0.5311 | +0.028 | 742 | 446 |
| C5 EmbeddingGemma 2 fp32 | ONNX | 0.9543 | +0.192 * | 0.5314 | +0.028 | 648 | 413 |
| C6 Granite 311M multilingual R2 int8 | ONNX | 0.7624 | 0.000 | 0.5262 | +0.023 | 355 | 232 |

\* SciFact's dev split is SciFact's training set (809 queries). EmbeddingGemma 2 finds a relevant document in the top 10 for every query
(recall@10 1.0): almost certainly trained on it. SciFact dev is not evidence for these models; the held-out test split decides.

Setup note: the first C5 run failed because the ONNX graph is the full multimodal model (image, video and audio inputs required); the
lab now feeds zero-length tensors for those, and C5 then matches C4 (cosine 0.9997). Rerun before any result was used.

**Gate:** every candidate gains at least +0.01 on one of the two corpora, so all pass stage A. Same model, two runtimes: llama.cpp
encoded EmbeddingGemma 2 about twice as fast as ONNX Runtime on this CPU (319 s against 648–742 s), with the same scores.
Decision: run stage A2 (the amendment's candidates) first, then stage B on the best of both.

## Stage A2 results (14 November)

Same setup; C9 and C10 on the GPU (documents and queries). Raw rows: `bench/sweeps/llamacpp2.jsonl`. The first C10 run was stopped
before it produced a row: the lab had left C9's server running, holding 6.4 GB of VRAM, so most of C10 ran on the CPU. The lab now stops
a model's llama.cpp servers before loading the next one; C10 was rerun alone.

| Candidate | Licence | Hardware | SciFact nDCG@10 | Δ | Alloprof nDCG@10 | Δ | SciFact s | Alloprof s |
|---|---|---|---|---|---|---|---|---|
| Shipped pair | Apache 2.0 / MIT | CPU | 0.7624 | | 0.5036 | | | |
| C7 Jina v5 nano retrieval Q8_0 | CC BY-NC 4.0 | CPU | 0.7890 | +0.027 | 0.5757 | **+0.072** | 159 | 106 |
| C8 Jina v5 small retrieval Q8_0 | CC BY-NC 4.0 | CPU | 0.7938 | +0.032 | 0.5886 | **+0.085** | 736 | 449 |
| C9 Qwen3-Embedding-4B Q8_0 | Apache 2.0 | GPU | 0.7901 | +0.028 | 0.6180 | **+0.114** | 1,451 | 908 |
| C10 Qwen3-Embedding-8B Q8_0 | Apache 2.0 | GPU | 0.7894 | +0.027 | 0.6080 | +0.104 | 2,194 | 1,383 |

All four pass stage A. C10 is worse and slower than C9 on both corpora.

## Stage B shortlist (written 14 November, after stages A and A2, before any stage B run)

A candidate is dropped when another candidate of the same class (CPU or GPU) is both better and faster on the stage A corpus of that
language group. Licence is not a filter (amendment), so the dominance rule is applied within one licence group too, to keep the best
permissive option of each class in the matrix.

- **French/mixed group** (stage B corpus: mix-fr-en dev): C9 Qwen3-4B (GPU), C8 Jina small, C7 Jina nano, C2 Qwen3-0.6B (best permissive
  on the CPU in French), C1 EmbeddingGemma 1 (fastest permissive). Dropped: C10 (dominated by C9), C3 bge-m3, C4/C5 EmbeddingGemma 2 and
  C6 Granite (each dominated by C1).
- **English group** (stage B corpus: NFCorpus dev): C1, C4 (GGUF only; C5 is the same model and slower), C7, C8, C9. Dropped: C10
  (dominated by C9), C2, C3 and C6 (below +0.01 on SciFact). SciFact dev is contaminated for C1 and C4, so for them NFCorpus and the
  held-out split decide.

Stage B then applies conditions 1 to 5 to the survivors of each group: held-out confirmation, query p95 (CPU class or GPU class), GPU
indexing agreement, RAM and VRAM.

## Stage B, dev part: condition 1 (14 November)

Raw rows: `bench/sweeps/llamacpp.jsonl` and `llamacpp2.jsonl`, tag `research:llamacpp-b`. The first batch hit the 2-hour background
limit during C8 on mix-fr-en; C8 and C9 were rerun alone. C8's mix-fr-en time reuses document vectors cached by the stopped run, so it
is not comparable.

French/mixed group (pair: Alloprof 0.5036, mix-fr-en 0.5796):

| Candidate | Class | Alloprof Δ | mix-fr-en nDCG@10 | Δ | mix-fr-en s | Condition 1 |
|---|---|---|---|---|---|---|
| C9 Qwen3-Embedding-4B | GPU | +0.114 | 0.6390 | +0.059 | 3,438 | pass |
| C1 EmbeddingGemma 1 | CPU | +0.059 | 0.6459 | +0.066 | 633 | pass |
| C8 Jina v5 small | CPU | +0.085 | 0.6291 | +0.050 | (cached) | pass |
| C7 Jina v5 nano | CPU | +0.072 | 0.6226 | +0.043 | 374 | pass |
| C2 Qwen3-Embedding-0.6B | CPU | +0.069 | 0.6088 | +0.029 | 1,772 | pass |

English group (pair: SciFact 0.7624, NFCorpus 0.3539):

| Candidate | SciFact Δ | NFCorpus nDCG@10 | Δ | NFCorpus s | Condition 1 |
|---|---|---|---|---|---|
| C9 Qwen3-Embedding-4B (GPU) | +0.028 | 0.3705 | +0.017 | 1,054 | pass |
| C1 EmbeddingGemma 1 | +0.090 * | 0.3644 | +0.011 | 191 | pass |
| C8 Jina v5 small | +0.032 | 0.3635 | +0.010 (0.0096) | 516 | fail |
| C7 Jina v5 nano | +0.027 | 0.3585 | +0.005 | 111 | fail |
| C4 EmbeddingGemma 2 | +0.191 * | 0.3569 | +0.003 | 249 | fail |

\* contaminated SciFact dev. mix-fr-en is a union that includes SciFact documents and queries (test split), so C1's mix-fr-en lead
needs the held-out confirmation like every other number here.

Survivors for conditions 2 to 5: French/mixed C9, C1, C8, C7, C2; English C9, C1.

**Scope cut (15 November, before any held-out result):** C9 (Qwen3-Embedding-4B, GPU) leaves stage B: its dev lead
over C1 does not justify about 5 times the indexing time for now. It stays on record as the quality option for a company corpus built
once (see `docs/research/EMBEDDING-LEADERBOARDS.md`). C2 (Qwen3-Embedding-0.6B) leaves too: on mix-fr-en it gains less than half of C1
(+0.029 against +0.066) and indexes about 3 times slower. Conditions 2 to 5 continue on French/mixed C1, C8, C7 and English C1.

## Stage B, condition 2: held-out (15 November)

Paired bootstrap, 10,000 resamples, against the shipped pair's held-out receipts of 13 November (same configuration). Full output and
receipts: `LLAMACPP-CONFIRMATION.md` (runner `confirm_llamacpp.py`). Document vectors were cached from the dev runs (same corpus).

| Candidate | SciFact | NFCorpus | Alloprof | mix-fr-en | Condition 2 |
|---|---|---|---|---|---|
| C1 EmbeddingGemma 1 | +0.040 [+0.021, +0.060] | +0.019 [+0.006, +0.033] | +0.075 [+0.062, +0.087] | +0.061 [+0.052, +0.070] | pass, both groups |
| C8 Jina v5 small | | | +0.100 [+0.088, +0.113] | +0.080 [+0.071, +0.090] | pass, French/mixed |
| C7 Jina v5 nano | | | +0.091 [+0.079, +0.103] | +0.074 [+0.065, +0.083] | pass, French/mixed |

On held-out French and mixed data the order reverses from dev: both Jina models beat C1, nano included. The first C8 mix-fr-en run was
cut off by a user interrupt after its receipt was written; the rerun gave the identical interval (both sections are in the log).

## Stage B, conditions 3 to 5 (15 November)

Runner `t5_costs.py`, rows in `sweeps/t5_costs.jsonl`. Query p95: the same 100 dev queries per corpus, one at a time, through the
resident `llama-server` (CPU) called over HTTP from Python, as condition 3 specifies. RAM is the server's working set; VRAM the
GPU server's dedicated memory after 512 documents.

| Candidate | File | Query p95 (limit) | RAM, CPU server | GPU server RAM + VRAM | GPU index: min cosine, docs < 0.9999 | GPU index: nDCG diff |
|---|---|---|---|---|---|---|
| C7 Jina v5 nano | 233 MB | Alloprof 38.4 (34.4), mix 36.1 (33.8) | 316 MB | 519 + 662 MB | 0.9998; 93 and 611 | +0.0002 / −0.0002 |
| C1 EmbeddingGemma 1 | 334 MB | SciFact 58.2 (16.0), NFCorpus 23.1 (8.2), Alloprof 61.0 (34.4), mix 60.5 (33.8) | 332 MB | 805 + 897 MB | 0.9993; 1,266 to 5,552 | −0.0009 to 0.0000 |
| C8 Jina v5 small | 639 MB | Alloprof 107.6 (34.4), mix 98.0 (33.8) | 1,083 MB | 903 + 1,920 MB | 0.9999; 1 and 2 | 0.0000 |

- **Condition 3 fails for all three.** Nano is closest (about 10 % over).
- **Condition 4 passes for all three** (at most 1.2 GB).
- **Condition 5 fails for all three** on the 0.9999 cosine bar, while the GPU-built index scores within 0.001 of the CPU-built one
  everywhere (Jina small: identical). The bar is kept as written; GPU indexing is not offered for these models.

**Outcome of test 5: no candidate is adopted.** All three are clearly better on held-out data, but none meets the query cost limit
through a server. Test 6 (`LLAMACPP-INPROCESS.md`) measures how much of that cost is the HTTP transport.
