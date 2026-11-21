# candle in place of ONNX Runtime (#358 step 1), 2026-11-21

Question: can candle, a pure-Rust ML library, replace ONNX Runtime in Footnote's encoder: the same vectors, at a speed close enough?

Result: candle gives the same vectors (cosine 1.0 on gte-small f32, at least 0.99972 on bekko-a8m) but encodes passages 3.9 to 7.4 times slower and single queries 1.8 to 4.9 times slower than ONNX Runtime, so it fails the speed bar; step 2 (a hand-written int8 encoder) is next.

## Setup

- Engines: Footnote's own encoder (`footnote_search::encoder`, ONNX Runtime 1.31) against candle 0.11.1 (`candle-transformers` BERT for gte-small, ModernBERT for bekko-a8m), weights from each model's `model.safetensors` loaded as f32.
- Recipe kept identical on the candle side: the model's own `tokenizer.json`, truncation at 512 tokens, padding to the longest text of each batch of 8, attention-mask mean pooling and L2 normalization in f64.
- Models: bekko-a8m (`hotchpotch/bekko-embedding-v1-a8m` at `c721113`, ONNX as Footnote installs it; safetensors are bf16); gte-small as shipped (`Xenova/gte-small` `model_quantized.onnx`, int8) and unquantized (`Xenova/gte-small` `onnx/model.onnx`, f32) against `thenlper/gte-small` at `17e1f34` (safetensors are f16, the same weights).
- Texts: 350 passages (300 paragraphs and 50 long runs of 12 paragraphs that hit the 512-token limit) and 100 headings as queries, cut from Footnote's own `docs/` and `bench/experiments/` Markdown by [make_texts.py](candle-encoder/make_texts.py) (seed 358).
- Threads: passages 12 for both engines (Footnote's bulk session uses half the logical CPUs); queries 2 for both (Footnote's query session).
- Harness: [candle-encoder/](candle-encoder/src/main.rs), its own crate outside the workspace. Release build, Windows 11, Intel Core Ultra 7 270K Plus (24 logical CPUs), machine idle (CPU 6% before the runs). One run each.

## Results

Parity: cosine between the two engines' vectors for the same text, and how many of each of the first 50 passages' 10 nearest neighbours the two engines share.

| model (ONNX side) | cosine min | cosine mean | texts below 0.9999 | top-10 overlap |
|---|---:|---:|---:|---:|
| gte-small f32 | 1.0000000 | 1.0000000 | 0 of 350 | 100% |
| gte-small int8 (shipped) | 0.98764 | 0.99786 | 350 of 350 | 88% |
| bekko-a8m (shipped) | 0.99972 | 0.99990 | 122 of 350 | 98% |

Speed:

| model | passages/s, ONNX | passages/s, candle | query median, ONNX | query median, candle | load, ONNX → candle |
|---|---:|---:|---:|---:|---:|
| gte-small int8 (shipped) | 67.8 | 9.2 (7.4× slower) | 1.78 ms | 8.77 ms (4.9× slower) | 268 → 85 ms |
| gte-small f32 | 44.6 | 9.2 (4.9× slower) | not run | not run | 419 → 86 ms |
| bekko-a8m | 121.6 | 31.1 (3.9× slower) | 1.42 ms | 2.52 ms (1.8× slower) | 942 → 783 ms |

- Peak memory, query runs: gte-small 158 MiB ONNX, 207 MiB candle; bekko-a8m 596 MiB ONNX, 823 MiB candle (candle widens bekko's bf16 weights, mostly the 256,000 × 384 token table, to f32).
- AVX2 does not help: the same candle build with `-C target-cpu=x86-64-v3` ran 9.3 passages/s on gte-small f32 and 31.0 on bekko-a8m, the same as the default build. The gap is in candle's CPU kernels, not in the compiler flags.
- Binary size, stripped release builds of the harness: candle with tokenizers alone 6.0 MiB; Footnote's search crate with ONNX Runtime alone 27.4 MiB; both 30.3 MiB. ONNX Runtime's own share is not separated here.

## Reading

- candle is correct. On the same f32 weights it reproduces ONNX Runtime to 1e-12, so a pure-Rust engine needs no maths of its own to be exact.
- The shipped gte-small is int8: any f32 engine changes its vectors (mean cosine 0.998, 88% top-10 overlap), so moving gte-small off ONNX Runtime means a reindex or a matching int8 path.
- bekko-a8m's small gap (min 0.99972) is unexplained: the installed ONNX file (130 MB) is smaller than its 256,000-token table in f32 (393 MB), so that export likely stores part of the weights at lower precision. Not checked.
- Speed decides it: indexing 4 to 7 times slower is not acceptable for Library and Code indexing, so candle does not replace ONNX Runtime as is.

## Decision

- **Not adopted.** ONNX Runtime stays.
- **Next (#358 step 2):** a hand-written int8 encoder for BERT-family models, scored against the same parity and speed bars with this harness; its int8 dot product is shared with the vector scoring work in #424.
- **Step 3 (Model2Vec)** is unchanged.
