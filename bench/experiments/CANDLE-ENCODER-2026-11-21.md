# Pure-Rust engines in place of ONNX Runtime: candle and tract (#358), 2026-11-21

Question: can a pure-Rust engine, candle or tract, replace ONNX Runtime in Footnote's encoder: the same vectors, at a speed close enough?

Result: both give the same vectors on f32 models (tract cosine 1.0 on both; candle 1.0 on gte-small, 0.99972 minimum on bekko-a8m) but both are 3 to 7 times slower than ONNX Runtime at indexing and 1.7 to 4.9 times slower per query, tract runs the shipped int8 gte-small 67 times slower, and neither is adopted.

## Setup

- Engines: Footnote's own encoder (`footnote_search::encoder`, ONNX Runtime 1.31) against candle 0.11.1 (`candle-transformers` BERT for gte-small, ModernBERT for bekko-a8m, weights from each model's `model.safetensors` loaded as f32) and tract 0.23.10 (`tract-onnx`, the same `model.onnx` files Footnote loads, optimized plan with symbolic batch and length, multithreaded matrix products).
- Recipe kept identical: the model's own `tokenizer.json`, truncation at 512 tokens, padding to the longest text of each batch of 8, attention-mask mean pooling and L2 normalization in f64.
- Models: bekko-a8m (`hotchpotch/bekko-embedding-v1-a8m` at `c721113`, ONNX as Footnote installs it; safetensors are bf16); gte-small as shipped (`Xenova/gte-small` `model_quantized.onnx`, int8) and unquantized (`Xenova/gte-small` `onnx/model.onnx`, f32), with candle on `thenlper/gte-small` at `17e1f34` (safetensors are f16, the same weights).
- Texts: 350 passages (300 paragraphs and 50 long runs of 12 paragraphs that hit the 512-token limit) and 100 headings as queries, cut from Footnote's own `docs/` and `bench/experiments/` Markdown by [make_texts.py](candle-encoder/make_texts.py) (seed 358).
- Threads: passages 12 for every engine (Footnote's bulk session uses half the logical CPUs); queries 2 (Footnote's query session).
- Harness: [candle-encoder/](candle-encoder/src/main.rs), its own crate outside the workspace. Release build, Windows 11, Intel Core Ultra 7 270K Plus (24 logical CPUs), machine idle (CPU 6% before the runs). One run each.

## Results

Parity with ONNX Runtime: cosine between the two engines' vectors for the same text, and how many of each of the first 50 passages' 10 nearest neighbours they share.

| model (ONNX side) | engine | cosine min | cosine mean | texts below 0.9999 | top-10 overlap |
|---|---|---:|---:|---:|---:|
| gte-small f32 | candle | 1.0000000 | 1.0000000 | 0 of 350 | 100% |
| gte-small f32 | tract | 1.0000000 | 1.0000000 | 0 of 350 | 100% |
| gte-small int8 (shipped) | candle (f32 weights) | 0.98764 | 0.99786 | 350 of 350 | 88% |
| gte-small int8 (shipped) | tract | 0.99405 | 0.99867 | 350 of 350 | 90% |
| bekko-a8m (shipped) | candle | 0.99972 | 0.99990 | 122 of 350 | 98% |
| bekko-a8m (shipped) | tract | 1.0000000 | 1.0000000 | 0 of 350 | 100% |

Passages per second, 12 threads (times slower than ONNX Runtime on the same file):

| model | ONNX Runtime | candle | tract |
|---|---:|---:|---:|
| gte-small int8 (shipped) | 67.8 | 9.2 (7.4×, f32 weights) | 1.0 (67×) |
| gte-small f32 | 44.6 | 9.2 (4.9×) | 11.6 (3.9×) |
| bekko-a8m | 121.6 | 31.1 (3.9×) | 38.2 (3.2×) |

Query median, 2 threads:

| model | ONNX Runtime | candle | tract |
|---|---:|---:|---:|
| gte-small int8 (shipped) | 1.78 ms | 8.77 ms (4.9×, f32 weights) | not run (int8 path 67× slower) |
| gte-small f32 | 3.98 ms | not run | 7.41 ms (1.9×; 4.2× the shipped int8) |
| bekko-a8m | 1.42 ms | 2.52 ms (1.8×) | 2.34 ms (1.7×) |

- tract thread scaling on bekko-a8m passages: 12.7/s on 1 thread, 28.6 on 4, 37.7 on 12, 39.5 on 24. It flattens past 12, which suggests the steps between the matrix products run on one thread.
- candle with `-C target-cpu=x86-64-v3` (AVX2): 9.3 passages/s on gte-small f32 and 31.0 on bekko-a8m, the same as the default build.
- Peak memory, query runs: bekko-a8m 596 MiB ONNX Runtime, 823 MiB candle (it widens bf16 weights to f32), 515 MiB tract; gte-small f32 471 MiB ONNX Runtime, 269 MiB tract; gte-small int8 158 MiB ONNX Runtime, 207 MiB candle.
- Binary size, stripped release builds of the harness with one engine: candle 6.0 MiB, tract 30.6 MiB, Footnote's search crate with ONNX Runtime 27.4 MiB. ONNX Runtime's own share is not separated; tract alone is larger than it.

## Reading

- Both engines are correct. tract reproduces ONNX Runtime to 1e-12 on every f32 file, so candle's small bekko-a8m gap comes from candle (its ModernBERT code or the bf16 safetensors), not from the ONNX export.
- Neither handles the shipped int8 gte-small: candle has no int8 path for BERT, tract has one but it is 67 times slower and not exact (0.994). Moving gte-small off ONNX Runtime would mean shipping the f32 model (twice the download, 1.9 to 4 times slower than today) and a reindex.
- Speed decides it: indexing 3 to 7 times slower is not acceptable for Library and Code indexing. ONNX Runtime's lead comes from fused operators and hand-tuned int8 and f32 kernels that neither Rust engine has.
- The build problems that started #358 (glibc 2.38, the dynamic C runtime) were fixed in #416, so what a switch would still buy is a smaller binary and an easier macOS build; tract does not even give the smaller binary.

## Decision

- **Neither is adopted.** ONNX Runtime stays.
- **#358 step 2 (a hand-written int8 encoder) is not started:** matching ONNX Runtime would take weeks of SIMD kernel work for each CPU family, for no gain the release now needs.
- **Step 3 (Model2Vec)** is unchanged.
