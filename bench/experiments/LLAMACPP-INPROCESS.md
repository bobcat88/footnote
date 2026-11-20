# Test 6: llama.cpp in-process, without the HTTP server (Jina v5 nano)

Written 15 November 2026, before any run. Follows test 5 (`LLAMACPP-MODELS.md`), where every survivor passed held-out quality but
failed condition 3 (query p95 at most twice the shipped pair's) through a `llama-server` called over HTTP from Python.

## Question

How much of Jina v5 nano's query time (p95 36 to 38 ms) is transport (a new HTTP connection per query, JSON, the server's slot
scheduling), and does calling llama.cpp directly from Rust, as Footnote would ship it, meet condition 3?

## Setup, frozen

- Model: Jina v5 text nano retrieval, Q8_0, the same pinned GGUF as test 5. Prompt `Query: ` + query.
- Runtime: the same llama.cpp build b11901 DLLs (`<local-llamacpp-runtime>`), CPU (`n_gpu_layers 0`), threads = half the
  logical CPUs, last-token pooling. No new build, no new dependency: Rust calls `llama.dll` through `LoadLibrary`.
- Queries: the same 100 dev queries per corpus as test 5 (Alloprof, mix-fr-en), clipped to 500 bytes, 3 warm-up calls, one query at a
  time, on a quiet machine (no other benchmark running).
- Arms:
  - **A** the test 5 path: Python, a new HTTP connection per query (re-measured today);
  - **B** the same server with one kept-alive HTTP connection (what a WebSocket or keep-alive would give);
  - **C** Rust, in-process, no server (`bench/llama_inproc/`).

## Rule

- **Validity:** C's vector for every query has cosine at least 0.9999 with A's. Otherwise C is not the same computation and its
  timings do not count.
- **Pass:** C's p95 is at most twice the shipped pair's (test 5 condition 3, the 13 November figures): Alloprof 34.4 ms, mix-fr-en 33.8 ms.
- Reported, not pass/fail: A and B (the transport share), p50, the shipped pair re-measured today for context (the threshold stays the
  13 November figure).

If C passes, Jina v5 nano meets every CPU-class condition of test 5 except GPU indexing (vectors at 0.9998, so documents are indexed on
the CPU), and becomes the candidate for an opt-in French/mixed profile, wired into Footnote's Rust encoder. EmbeddingGemma 1 and Jina
small would follow the same test only if nano passes.

## Results (15 November)

Runner `t6_inproc.py`, rows in `sweeps/t6_inproc.jsonl`. Milliseconds per query, 100 dev queries per corpus, quiet machine.

| Corpus | A: HTTP, new connection | B: HTTP kept alive | C: Rust in-process | C vs A cosine (min) | Limit | Verdict |
|---|---|---|---|---|---|---|
| Alloprof | p50 33.9, p95 36.7 | p50 24.2, p95 26.0 | p50 23.6, **p95 25.9** | 1.000000 | 34.4 | pass |
| mix-fr-en | p50 33.4, p95 37.1 | p50 24.2, p95 26.2 | p50 23.9, **p95 26.1** | 1.000000 | 33.8 | pass |

- **Valid and passing:** C computes the same vectors as the server, and its p95 is under the limit on both corpora.
- **Where the time went:** about 10 ms of A's 34 ms was opening a new HTTP connection per query. A kept-alive connection (B) removes
  almost all of it; leaving out HTTP altogether (C) saves only 0.1 to 0.6 ms more. The remaining ~24 ms is the model itself.
- **Caveat, reported as the rule asks:** the shipped pair re-measured today is faster than on 13 November: p95 3.4 + 6.3 = 9.7 ms on
  Alloprof, 3.2 + 6.4 = 9.6 ms on mix-fr-en, against 17.2 and 16.9. Against today's figures the limit would be about 19.4 ms and nano
  would not pass. The 13 November figure stays the rule's threshold; the reading is that nano in-process costs about 2.7 times today's
  pair per query, about 26 ms at p95.
- An early smoke test on two very short queries gave 4 to 8 ms; real queries are longer, so that figure did not hold.

## Product wiring (15 November, approved to integrate; rule written before the product runs)

Footnote now loads GGUF models itself (`--features llama`, crate `llama-cpp-2` 0.1.161, which compiles its own llama.cpp; model
directory `artifacts/models/jina-v5-nano` with `"runtime": "llama.cpp"`). Wiring check (`t6_product_check.py`): its vectors against
the b11901 server's, 100 queries and the 50 longest Alloprof documents. Result: cosine at least 0.9997 (71 queries and 6 documents
below 0.9999), so not bit-identical to test 6 (another llama.cpp version and compiler); mean 20.6 ms per query, faster than test 6.

Because the vectors moved slightly, adoption is decided end to end, on the product itself:

- **Engines:** `shipped-ensemble-mcp` (the shipped pair) against `nano-mcp` (same product, Jina v5 nano alone, llama build), both as
  an agent uses them (one resident MCP session), held-out split of Alloprof and mix-fr-en.
- **Pass:** paired bootstrap 95 % interval of the nDCG@10 difference above zero on both corpora (test 5 condition 2, on the product).
- Reported: per-query latency of both, index build time, database size.

## Product result (15 November)

Held-out split, both engines as an agent uses them (one resident MCP session).

| Corpus | Queries | Pair nDCG@10 | Nano nDCG@10 | Difference, 95 % interval | Verdict |
|---|---|---|---|---|---|
| Alloprof | 1,624 | 0.4757 | 0.5782 | +0.1025 [+0.0902, +0.1149] | pass |
| mix-fr-en | 2,320 | 0.4892 | 0.5720 | +0.0829 [+0.0737, +0.0925] | pass |

- **Quality passes on both corpora**, slightly above the lab figures (+0.091 and +0.074).
- Reported, not counted: index build 707 s and 2,174 s for nano against 272 s and 920 s for the pair; database 107 and 103 MB
  against 42 and 40 MB (768-wide vectors).
- **Latency and build times are not valid:** the machine was in use during the runs (nano p95 64.6 and 70.1 ms against ~26 ms in
  test 6). They are re-measured only on an idle machine, on an explicit go.

## Product latency on an idle machine (15 November evening)

Re-measured as asked above, same product runs (held-out, one resident MCP session), machine not in active use (Steam and iCUE open in the background at the start; Process Lasso, a governor, always on):

| Corpus | Pair p50 / p95 | Nano p50 / p95 | Nano p95 / pair p95 |
|---|---|---|---|
| Alloprof | 21.2 / 37.4 ms | 27.3 / 65.0 ms | 1.74 |
| mix-fr-en | 25.5 / 42.3 ms | 31.1 / 68.8 ms | 1.63 |

- These are whole searches as an agent sees them (encoding, lexical and vector search, fusion), not encoding alone, so they are
  not comparable with test 6's 26 ms.
- Nano adds about 6 ms at p50 and 26 to 28 ms at p95 per search, under twice the pair's p95 on both corpora. The first Alloprof
  nano run (p95 64.6 ms) was not noise: the re-run gives 65.0 ms.
- Reported, not judged (the product rule judges quality only).
