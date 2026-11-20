# Test 5 held-out confirmation

Condition 2 of `LLAMACPP-MODELS.md`: paired bootstrap 95 % interval of the nDCG@10 difference above zero on each corpus of the group. Configurations frozen from the dev sweep.

## T5 gguf-embeddinggemma alone in the hybrid, nfcorpus

```text
nfcorpus (heldout), 323 queries
  A = bench\receipts\heldout\20261113T231542Z-nfcorpus-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.3735
  B = bench\receipts\heldout\20261115T070247Z-nfcorpus-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.3924
  B - A = +0.0189  95% CI [+0.0055, +0.0325]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231542Z-nfcorpus-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T070247Z-nfcorpus-lab_llamacpp_e23f96399078-heldout.json).

## T5 gguf-embeddinggemma alone in the hybrid, scifact

```text
scifact (heldout), 300 queries
  A = bench\receipts\heldout\20261113T231530Z-scifact-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.7460
  B = bench\receipts\heldout\20261115T070259Z-scifact-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.7862
  B - A = +0.0401  95% CI [+0.0211, +0.0598]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231530Z-scifact-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T070259Z-scifact-lab_llamacpp_e23f96399078-heldout.json).

## T5 gguf-embeddinggemma alone in the hybrid, alloprof

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261115T070409Z-alloprof-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.5630
  B - A = +0.0747  95% CI [+0.0623, +0.0873]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T070409Z-alloprof-lab_llamacpp_e23f96399078-heldout.json).

## T5 gguf-embeddinggemma alone in the hybrid, mix-fr-en

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261115T070549Z-mix-fr-en-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.5601
  B - A = +0.0610  95% CI [+0.0518, +0.0704]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T070549Z-mix-fr-en-lab_llamacpp_e23f96399078-heldout.json).

## T5 gguf-jina-v5-small alone in the hybrid, mix-fr-en

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261115T070854Z-mix-fr-en-lab_llamacpp_826ec37c3c51-heldout.rankings.json.gz: nDCG@10 0.5792
  B - A = +0.0800  95% CI [+0.0708, +0.0895]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T070854Z-mix-fr-en-lab_llamacpp_826ec37c3c51-heldout.json).

## T5 gguf-jina-v5-small alone in the hybrid, alloprof

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261115T070928Z-alloprof-lab_llamacpp_826ec37c3c51-heldout.rankings.json.gz: nDCG@10 0.5887
  B - A = +0.1003  95% CI [+0.0881, +0.1126]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T070928Z-alloprof-lab_llamacpp_826ec37c3c51-heldout.json).

## T5 gguf-jina-v5-small alone in the hybrid, mix-fr-en

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261115T071225Z-mix-fr-en-lab_llamacpp_826ec37c3c51-heldout.rankings.json.gz: nDCG@10 0.5792
  B - A = +0.0800  95% CI [+0.0708, +0.0895]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T071225Z-mix-fr-en-lab_llamacpp_826ec37c3c51-heldout.json).

## T5 gguf-jina-v5-nano alone in the hybrid, alloprof

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261115T071318Z-alloprof-lab_llamacpp_0af85a59323b-heldout.rankings.json.gz: nDCG@10 0.5793
  B - A = +0.0909  95% CI [+0.0793, +0.1030]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T071318Z-alloprof-lab_llamacpp_0af85a59323b-heldout.json).

## T5 gguf-jina-v5-nano alone in the hybrid, mix-fr-en

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261115T071436Z-mix-fr-en-lab_llamacpp_0af85a59323b-heldout.rankings.json.gz: nDCG@10 0.5729
  B - A = +0.0737  95% CI [+0.0648, +0.0828]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261115T071436Z-mix-fr-en-lab_llamacpp_0af85a59323b-heldout.json).

## GPU-built index (condition 5): gguf-jina-v5-nano, alloprof (A = CPU-built, B = GPU-built)

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261115T071318Z-alloprof-lab_llamacpp_0af85a59323b-heldout.rankings.json.gz: nDCG@10 0.5793
  B = bench\receipts\heldout\20261115T074800Z-alloprof-lab_llamacpp_be47379ffe59-heldout.rankings.json.gz: nDCG@10 0.5795
  B - A = +0.0002  95% CI [-0.0002, +0.0007]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T074800Z-alloprof-lab_llamacpp_be47379ffe59-heldout.json).

## GPU-built index (condition 5): gguf-jina-v5-nano, mix-fr-en (A = CPU-built, B = GPU-built)

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261115T071436Z-mix-fr-en-lab_llamacpp_0af85a59323b-heldout.rankings.json.gz: nDCG@10 0.5729
  B = bench\receipts\heldout\20261115T075330Z-mix-fr-en-lab_llamacpp_be47379ffe59-heldout.rankings.json.gz: nDCG@10 0.5726
  B - A = -0.0002  95% CI [-0.0005, +0.0000]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T075330Z-mix-fr-en-lab_llamacpp_be47379ffe59-heldout.json).

## GPU-built index (condition 5): gguf-embeddinggemma, alloprof (A = CPU-built, B = GPU-built)

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261115T070409Z-alloprof-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.5630
  B = bench\receipts\heldout\20261115T075821Z-alloprof-lab_llamacpp_720a2cffd89a-heldout.rankings.json.gz: nDCG@10 0.5623
  B - A = -0.0007  95% CI [-0.0018, +0.0003]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T075821Z-alloprof-lab_llamacpp_720a2cffd89a-heldout.json).

## GPU-built index (condition 5): gguf-embeddinggemma, mix-fr-en (A = CPU-built, B = GPU-built)

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261115T070549Z-mix-fr-en-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.5601
  B = bench\receipts\heldout\20261115T080727Z-mix-fr-en-lab_llamacpp_720a2cffd89a-heldout.rankings.json.gz: nDCG@10 0.5599
  B - A = -0.0002  95% CI [-0.0010, +0.0006]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T080727Z-mix-fr-en-lab_llamacpp_720a2cffd89a-heldout.json).

## GPU-built index (condition 5): gguf-embeddinggemma, nfcorpus (A = CPU-built, B = GPU-built)

```text
nfcorpus (heldout), 323 queries
  A = bench\receipts\heldout\20261115T070247Z-nfcorpus-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.3924
  B = bench\receipts\heldout\20261115T080944Z-nfcorpus-lab_llamacpp_720a2cffd89a-heldout.rankings.json.gz: nDCG@10 0.3915
  B - A = -0.0009  95% CI [-0.0023, +0.0001]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T080944Z-nfcorpus-lab_llamacpp_720a2cffd89a-heldout.json).

## GPU-built index (condition 5): gguf-embeddinggemma, scifact (A = CPU-built, B = GPU-built)

```text
scifact (heldout), 300 queries
  A = bench\receipts\heldout\20261115T070259Z-scifact-lab_llamacpp_e23f96399078-heldout.rankings.json.gz: nDCG@10 0.7862
  B = bench\receipts\heldout\20261115T081245Z-scifact-lab_llamacpp_720a2cffd89a-heldout.rankings.json.gz: nDCG@10 0.7861
  B - A = -0.0000  95% CI [-0.0001, +0.0000]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T081245Z-scifact-lab_llamacpp_720a2cffd89a-heldout.json).

## GPU-built index (condition 5): gguf-jina-v5-small, alloprof (A = CPU-built, B = GPU-built)

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261115T071118Z-alloprof-lab_llamacpp_826ec37c3c51-heldout.rankings.json.gz: nDCG@10 0.5887
  B = bench\receipts\heldout\20261115T081922Z-alloprof-lab_llamacpp_5131c47b7c92-heldout.rankings.json.gz: nDCG@10 0.5887
  B - A = +0.0000  95% CI [+0.0000, +0.0000]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T081922Z-alloprof-lab_llamacpp_5131c47b7c92-heldout.json).

## GPU-built index (condition 5): gguf-jina-v5-small, mix-fr-en (A = CPU-built, B = GPU-built)

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261115T071225Z-mix-fr-en-lab_llamacpp_826ec37c3c51-heldout.rankings.json.gz: nDCG@10 0.5792
  B = bench\receipts\heldout\20261115T083533Z-mix-fr-en-lab_llamacpp_5131c47b7c92-heldout.rankings.json.gz: nDCG@10 0.5792
  B - A = +0.0000  95% CI [+0.0000, +0.0000]  -> no consistent difference
```

[GPU receipt](../receipts/heldout/20261115T083533Z-mix-fr-en-lab_llamacpp_5131c47b7c92-heldout.json).

