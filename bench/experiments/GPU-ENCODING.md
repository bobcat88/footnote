# GPU encoding with DirectML: results and the install-time accelerator idea

Measured 14 November 2026, 01:05 to 01:40, on an otherwise quiet machine: Intel Core Ultra 7 270K Plus (24 logical CPUs) and AMD Radeon RX 6800, Windows 11, ONNX Runtime DirectML 1.23.0 in an isolated environment (`bench/.gpuenv`). Script: [gpu_encode.py](gpu_encode.py). Raw outputs: `bench/work/gpu-encode-scifact.json` (variable padding, the lab default) and `bench/work/gpu-encode-scifact-pad512.json` (every GPU input padded to 512 tokens). SciFact test: 5,183 documents, 300 queries, batch 32. Same lab `Encoder`, same model files; only the ONNX execution provider changes.

## Results

| Model | Precision | Batch docs/s CPU → GPU | Same results as CPU | One query, p50 CPU → GPU |
|---|---|---|---|---|
| bekko-a8m | fp32 | 87.8 → 283.8 (**3.2×**) | yes: cosine 1.000000, nDCG identical | 1.0 ms → 11.3 ms |
| gte-small | int8 | 46.9 → 67.0 (1.4×) | **no**: top 10 identical for 40 % of queries, nDCG 0.7256 → 0.7218 | 1.9 ms → 64.8 ms |
| embeddinggemma-300m | fp32 | 10.6 → 12.2 (1.2×) | yes | 13.0 ms → 129.8 ms |
| mmarco-mminilm reranker, 50 candidates | int8 | 1,419 → 1,348 ms per query | **no**: scores differ by up to 1.08, top-10 order identical for 10 % | |

Fixed 512-token padding (one input shape per batch size) did not help: bekko-a8m batch fell to 1.8×, a single EmbeddingGemma query rose to 1.3 s. ONNX Runtime reports that some nodes are not assigned to DirectML, so part of each graph runs on the CPU with copies in between.

## What this means

- **Queries stay on the CPU.** One query at a time is 10 to 34 times slower on the GPU for every model tested.
- **Batch encoding gains only with fp32 models**, and only clearly for the smallest one (3.2×). EmbeddingGemma, the model that would need it most, gains 1.2×.
- **Quantized models change their results on DirectML.** Our shipped pair includes int8 `gte-small`; building an index on the GPU would produce different vectors than the CPU path. That alone rules it out for the product as shipped.
- **Q3-2 fails as pre-registered**: the reranker needs at most 100 ms per query; it takes about 1.35 s on either device.

## OpenVINO on the CPU

Same script with `--provider openvino` (`bench/.ovenv`: `onnxruntime-openvino` 1.24.1 with OpenVINO 2025.4.1; OpenVINO 2026.4 does not load with this build). Both sides run on the same CPU; only the runtime changes. Raw output: `bench/work/openvino-encode-scifact.json`.

| Model | Batch docs/s ORT CPU → OpenVINO | Same results | One query, p50 ORT → OpenVINO |
|---|---|---|---|
| bekko-a8m (fp32) | 87.5 → 141.2 (1.6×) | yes | 0.9 ms → 2.4 ms |
| gte-small (int8) | 46.8 → 31.8 (0.7×) | **no**: top 10 identical for 37 % of queries | 1.7 ms → 9.1 ms |
| embeddinggemma-300m (fp32) | 10.6 → 3.3 (0.3×) | yes | 11.9 ms → 96.6 ms |
| mmarco-mminilm reranker (int8) | 1,410 → 6,129 ms per query | **no** | |

OpenVINO through the ONNX Runtime provider helps only batch indexing with the small fp32 model and loses everywhere else, likely because every new input shape is recompiled. Native OpenVINO with static shapes and a compiled-model cache was not tested. **ONNX Runtime's CPU provider stays the default.**

## The install-time accelerator idea

Detect the GPU at install, use DirectML on Windows, CUDA on NVIDIA, CPU otherwise. The mechanism is easy (on Windows the DirectML package already contains the CPU provider, so one package with a runtime fallback is enough). The measurements say it is not worth shipping today:

1. The only clear gain is batch indexing with an fp32 model, 3.2×. Indexing is a one-time cost per document; queries, which users wait for, get slower.
2. The shipped int8 model gives different results on DirectML, so a GPU index and a CPU index would disagree.
3. Microsoft has moved DirectML to maintenance in favour of Windows ML, so a new dependency on it would age badly.

Worth re-testing, in this order, before reopening the idea: CUDA on an NVIDIA machine (a different and more mature provider, not measured here); fp32 or fp16 exports of the models instead of int8; Windows ML's automatic provider selection; ONNX Runtime IO binding to avoid copies. Until one of them shows a gain on queries or keeps identical results, the CPU stays the only path.
