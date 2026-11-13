# ICM and Footnote comparison suite

Status: experimental. This suite prepares controlled tests; it does not by itself fix retrieval or establish that either product wins. Existing production behavior is unchanged.

## What is implemented

1. `icm_ablation.py` runs a **2 × 2 lab matrix**: SQLite FTS5 Porter or Tantivy English stemmer, crossed with z-score convex combination (alpha 0.55) or RRF (k=60). It indexes the same whole documents and reuses the same resident single-model dense vectors for every arm. It saves per-query rankings, input hashes, dataset pins, model-file hashes and software versions; up to 200 queries per arm are repeated to check deterministic rankings. Each lexical engine uses its own native tokenizer and scoring implementation; the match is not token-for-token identical. Search keeps up to 1,000 lexical candidates and 100 per fusion arm. Dense search is a full scan. This is not the shipped two-model, 2,000-character-chunk product, nor is it the ICM binary.
2. `icm_ablation_compare.py` validates matching dataset, split, data hash, pool, query IDs and ranking hashes, verifies that the repeated rankings match, then produces paired bootstrap intervals for nDCG@10 and recall@10/20. It refuses partial results unless explicitly allowed for exploration. Run on dev before any model or fusion is chosen; only then run held-out confirmation with frozen choices.
3. `longmem_footnote.py` indexes user turns of each LongMemEval session in a fresh temporary work area and runs the release MCP binary on all 500 questions, including the 30 `_abs` questions. It maps retrieved chunks back to distinct session IDs. The product still splits source files into chunks, unlike ICM's mostly one-memory-per-session protocol; treat this difference as part of the product comparison, not identical ingestion.
4. `longmem_icm.py` runs a supplied trusted ICM binary as a localhost server with a fresh temporary DB and explicit configuration per question. It forces keyword-only mode (`--no-embeddings`), uses the v2 engine, requests 50 units, uses user turns and no dates, splits text at 60,000 UTF-8 bytes, and rejects unknown or merged memory IDs. It never runs `icm init` or configures a host. This is **not** ICM's default hybrid result. It records binary hash, per-question ingest/query time, DB/WAL/SHM sizes, and ranks. Model cache, executable size and peak memory are not included in those file sizes.
5. `footprint_inventory.py` measures the actual bytes in explicitly named local artifact files or directories, rejects overlapping paths and symlinks, and reports each category separately. It does not infer whether stores contain equivalent data, and does not measure peak RAM or temporary files.
6. `longmem_score.py` scores Footnote, local ICM or ICM's published recall JSON on the same pinned dataset. It validates gold sessions and query coverage; reports any/all recall@5/@10, distinct-session nDCG@10, answerable/abstention slices and per-type results. It supports paired bootstrap for identical query sets. Raw unit ranks are used for recall; repeated split-session hits consume ranks. Footnote ranks are distinct sessions and its adapter requests at most 20; ICM requests 50 raw units. These limits and the different ingestion paths affect comparability. `--intersection` is explicitly exploratory and reports discarded question counts.

All timings and footprint data in the ablation are **component-only**: lexical time covers score retrieval up to 1,000 hits; dense time covers matrix multiplication with query embeddings already cached; fusion, encoding, MCP, startup and concurrent clients are excluded. No warm-up pass is claimed. Absolute times from this harness cannot be compared with full-product latency. FTS5 artifact includes text; Tantivy's artifact does not. Do not compare them as total storage.

## Prerequisites

- Windows paths below use PowerShell and RTK. The existing `bench/.labenv` needs NumPy, ONNX Runtime, tokenizers, snowballstemmer and `tantivy==0.27.0`. The latter was added to that local environment with `rtk proxy uv pip install --python bench/.labenv/Scripts/python.exe tantivy==0.27.0`; no global environment changed.
- Public benchmark data must be cached and pinned by `bench/datasets.lock.json`; the existing fetcher may download missing public data. The ablation refuses to download missing model files itself. It requires existing model files and lock entries where the registry supports them.
- Build Footnote's release binary and provision its models first; the Footnote runner calls the project's existing release adapter. The ICM runner needs a trusted ICM binary supporting v2 HTTP recall; it probes for the engine field and fails closed if absent. It starts no embedding model. The locally tested build was ICM source commit `3b9fcd7d726d82268f3d67a11ab63fb645442536` (0.11.5), built with `--no-default-features --features backend-sqlite,http-api`. Its public LongMemEval artifact is a separate vendor run on a different machine/version.
- Outputs must be new paths. No result file is overwritten. The two session-level runners checkpoint after each question. To resume after interruption, rerun the same command with `--resume` and the same output path. Resume fails if the question set, dataset pin, binary, runner or relevant helper/model hashes differ. Checkpoint files remain beside the output and are not final scored receipts; progress is printed after every question. Run one experiment at a time on an otherwise quiet machine if timing matters. Do not run two processes with the same output/checkpoint path. Do not use private documents, real agent databases or configuration in these tests.

## Commands

```powershell
rtk proxy bench/.labenv/Scripts/python.exe -m unittest discover -s bench/experiments -p 'test_*.py' -v

# Partial smoke, then full development split (change output filenames for another run).
rtk proxy bench/.labenv/Scripts/python.exe -I bench/experiments/icm_ablation.py --dataset scifact --split dev --max-queries 20 --model bekko-a8m --output bench/work/smoke-scifact.json
rtk proxy bench/.labenv/Scripts/python.exe -I bench/experiments/icm_ablation.py --dataset scifact --split dev --model bekko-a8m --output bench/work/dev-scifact.json
rtk proxy bench/.labenv/Scripts/python.exe -I bench/experiments/icm_ablation_compare.py bench/work/dev-scifact.json tantivy:cc bench/work/dev-scifact.json tantivy:rrf --output bench/work/dev-scifact-paired.json

# Shipped Footnote vs independently built ICM, both on the first 10 questions.
rtk proxy bench/.labenv/Scripts/python.exe -I bench/experiments/longmem_footnote.py --max-questions 10 --output bench/work/footnote-10.json
rtk proxy bench/.labenv/Scripts/python.exe -I bench/experiments/longmem_icm.py --binary bench/.cache/icm-source/target/release/icm.exe --max-questions 10 --output bench/work/icm-keyword-10.json
rtk proxy bench/.labenv/Scripts/python.exe -I bench/experiments/longmem_score.py bench/work/footnote-10.json --b bench/work/icm-keyword-10.json --allow-subset --output bench/work/paired-10.json

# Record binary and model bytes separately; supply DB/index/WAL paths only when retained.
rtk proxy bench/.labenv/Scripts/python.exe -I bench/experiments/footprint_inventory.py --artifact binary=target/release/footnote.exe --artifact bekko=artifacts/models/bekko-a8m --artifact gte=artifacts/models/gte-small --output bench/work/runtime-footprint.json

# Fixed tuning/confirmation splits: pass --split dev or --split heldout to both runners.
# The scorer needs --allow-subset for either split; those flags use the existing stable hash split.
# Full all-question suite: remove --max-questions in both commands and choose new output paths.
# The ICM runner is keyword-only. Do not label this ICM's published hybrid result.
```

To rescore ICM's published 500-question artifact, supply its `longmemeval-s-icm-v2-session-user-nodate-20261006.json` to `longmem_score.py`; it checks session IDs and gold against the local dataset. For a partial Footnote run against that full artifact, add `--allow-subset --intersection`, which reports both original counts and the selected subset. This is a comparison against vendor-reported rankings from another machine, not a local head-to-head experiment. For code and the benchmark artifact see https://github.com/rtk-ai/icm/tree/main/bench/amb.

## Acceptance and remaining work

Use existing `PLAN.md` pre-registered quality/cost thresholds, paired intervals and a frozen held-out split before shipping a change. A smoke run or dev win is not proof of a fix. The full ICM hybrid run with its default E5-large model, reproducible local 100k/1M end-to-end quality, equal-content full-product footprint (including model weights and peak RAM), repeated cold/warm timing and user-visible agent effectiveness remain to be implemented and measured. The local keyword-only runner and the vendor artifact do not fill those gaps. ICM must not be installed or initialized into the user's live agent configurations merely to run this suite.

## Initial verification (2026-11-12)

- Twelve fixture tests pass. The scorer reproduced the vendor ICM result: 500 questions, any@5 0.974 and all@5 0.886. The vendor run used hybrid retrieval on different hardware. Raw scorer output: `bench/work/icm-published-score.json` (local scratch, not a new ICM execution).
- The local keyword-only ICM 0.11.5 build and the shipped Footnote hybrid runner completed 10 identical question IDs in isolated databases; the scorer checked all IDs and gold. Both had 10/10 any@5 and all@5. This is a smoke test, not a statistically useful ranking or a hybrid-to-hybrid comparison. Full 500-question executions have not been run. The 10-question pilot predates runner hashes and split flags and requested 20 ICM results rather than the current 50; it cannot establish parity with the published protocol. Raw runs: `bench/work/icm-local-keyword-10.json`, `bench/work/footnote-longmem-10.json`, and `bench/work/longmem-local-paired-10.json`.
- Full development ablations using one Bekko a8m model were run for SciFact (809 questions), NFCorpus (324), Alloprof (692) and mixed French/English (1,852). On SciFact and Alloprof, switching the Tantivy lab arm from convex combination to RRF changed nDCG@10 by -0.0093 (95% paired bootstrap interval [-0.0174, -0.0011]) and -0.0297 ([-0.0430, -0.0169]), respectively. These are dev-only exploratory results, not a shipped-binary test, not the complete ICM algorithm and not a held-out finding. No change should ship on these results. The v2 dev outputs repeat up to 200 queries per arm and record whether rankings match. The paired comparisons use those v2 outputs. Output files are under `bench/work/icm-ablation-*-dev-*.json`.
- One runtime file inventory measured the Footnote executable and both model folders only. It does not measure the database, index, peak RAM or comparable ICM footprint.

- A single-model SciFact dev ablation compared Bekko a8m with a25m using the same Tantivy/CC settings. The a25m difference was +0.0078 nDCG@10 with paired 95% bootstrap interval [-0.0011, +0.0164], which includes zero. This does not validate replacing the shipped two-model ensemble. Raw paired result: `bench/work/icm-ablation-scifact-model-paired.json`.
