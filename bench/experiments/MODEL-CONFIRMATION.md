# Model heldout confirmation

Candidates must gain at least 0.01 nDCG@10 on both dev corpora of a language group. Configurations are frozen before heldout scoring. Positive bootstrap intervals alone do not authorize a production change; latency, memory and integration checks still apply. Failed or incomplete dev configurations are ineligible. Intervals are per comparison, without multiplicity correction.

## English: M2 embeddinggemma-300m alone in the hybrid

Dev gains: [0.08894999999999997, 0.014249999999999985].

```text
scifact (heldout), 300 queries
  A = bench\receipts\heldout\20261113T231530Z-scifact-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.7460
  B = bench\receipts\heldout\20261113T231538Z-scifact-lab_models2_6b66ec5f06f1-heldout.rankings.json.gz: nDCG@10 0.7891
  B - A = +0.0431  95% CI [+0.0243, +0.0626]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231530Z-scifact-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T231538Z-scifact-lab_models2_6b66ec5f06f1-heldout.json).

```text
nfcorpus (heldout), 323 queries
  A = bench\receipts\heldout\20261113T231542Z-nfcorpus-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.3735
  B = bench\receipts\heldout\20261113T231549Z-nfcorpus-lab_models2_6b66ec5f06f1-heldout.rankings.json.gz: nDCG@10 0.3900
  B - A = +0.0165  95% CI [+0.0032, +0.0298]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231542Z-nfcorpus-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T231549Z-nfcorpus-lab_models2_6b66ec5f06f1-heldout.json).

## French/mixed: M1 embeddinggemma-300m dense only

Dev gains: [0.06337999999999999, 0.08944999999999992].

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261113T231642Z-alloprof-lab_models2_f139fdd30471-heldout.rankings.json.gz: nDCG@10 0.5533
  B - A = +0.0649  95% CI [+0.0491, +0.0805]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T231642Z-alloprof-lab_models2_f139fdd30471-heldout.json).

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261113T231756Z-mix-fr-en-lab_models2_f139fdd30471-heldout.rankings.json.gz: nDCG@10 0.5585
  B - A = +0.0594  95% CI [+0.0471, +0.0718]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T231756Z-mix-fr-en-lab_models2_f139fdd30471-heldout.json).

## French/mixed: M2 arctic-m-v2 alone in the hybrid

Dev gains: [0.04639000000000004, 0.013999999999999901].

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261113T231829Z-alloprof-lab_models2_3a09f5d20f2d-heldout.rankings.json.gz: nDCG@10 0.5434
  B - A = +0.0550  95% CI [+0.0437, +0.0663]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T231829Z-alloprof-lab_models2_3a09f5d20f2d-heldout.json).

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261113T231914Z-mix-fr-en-lab_models2_3a09f5d20f2d-heldout.rankings.json.gz: nDCG@10 0.5407
  B - A = +0.0415  95% CI [+0.0330, +0.0503]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T231914Z-mix-fr-en-lab_models2_3a09f5d20f2d-heldout.json).

## French/mixed: M2 embeddinggemma-300m alone in the hybrid

Dev gains: [0.05798000000000003, 0.06618999999999997].

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261113T231950Z-alloprof-lab_models2_6b66ec5f06f1-heldout.rankings.json.gz: nDCG@10 0.5617
  B - A = +0.0733  95% CI [+0.0610, +0.0858]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T231950Z-alloprof-lab_models2_6b66ec5f06f1-heldout.json).

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261113T232039Z-mix-fr-en-lab_models2_6b66ec5f06f1-heldout.rankings.json.gz: nDCG@10 0.5622
  B - A = +0.0630  95% CI [+0.0539, +0.0725]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232039Z-mix-fr-en-lab_models2_6b66ec5f06f1-heldout.json).

## French/mixed: M2 harrier-270m alone in the hybrid

Dev gains: [0.017240000000000033, 0.014129999999999976].

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261113T232115Z-alloprof-lab_models2_bad79b88abde-heldout.rankings.json.gz: nDCG@10 0.5103
  B - A = +0.0219  95% CI [+0.0100, +0.0335]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232115Z-alloprof-lab_models2_bad79b88abde-heldout.json).

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261113T232202Z-mix-fr-en-lab_models2_bad79b88abde-heldout.rankings.json.gz: nDCG@10 0.5203
  B - A = +0.0211  95% CI [+0.0121, +0.0303]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232202Z-mix-fr-en-lab_models2_bad79b88abde-heldout.json).

## French/mixed: M3 bekko-a8m + arctic-m-v2

Dev gains: [0.03970000000000007, 0.015349999999999975].

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261113T232255Z-alloprof-lab_models2_9cfb5988a1a9-heldout.rankings.json.gz: nDCG@10 0.5342
  B - A = +0.0459  95% CI [+0.0369, +0.0549]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232255Z-alloprof-lab_models2_9cfb5988a1a9-heldout.json).

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261113T232404Z-mix-fr-en-lab_models2_9cfb5988a1a9-heldout.rankings.json.gz: nDCG@10 0.5316
  B - A = +0.0324  95% CI [+0.0263, +0.0386]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232404Z-mix-fr-en-lab_models2_9cfb5988a1a9-heldout.json).

## French/mixed: M3 bekko-a8m + embeddinggemma-300m

Dev gains: [0.04260000000000008, 0.0359799999999999].

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261113T232503Z-alloprof-lab_models2_1755dbbf25c7-heldout.rankings.json.gz: nDCG@10 0.5437
  B - A = +0.0554  95% CI [+0.0462, +0.0650]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232503Z-alloprof-lab_models2_1755dbbf25c7-heldout.json).

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261113T232622Z-mix-fr-en-lab_models2_1755dbbf25c7-heldout.rankings.json.gz: nDCG@10 0.5457
  B - A = +0.0465  95% CI [+0.0398, +0.0533]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232622Z-mix-fr-en-lab_models2_1755dbbf25c7-heldout.json).

## French/mixed: M4 embeddinggemma-300m 256 dims alone in the hybrid

Dev gains: [0.03583999999999998, 0.057010000000000005].

```text
alloprof (heldout), 1624 queries
  A = bench\receipts\heldout\20261113T231608Z-alloprof-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4884
  B = bench\receipts\heldout\20261113T232658Z-alloprof-lab_models2_e027f0b8a1a7-heldout.rankings.json.gz: nDCG@10 0.5389
  B - A = +0.0506  95% CI [+0.0382, +0.0633]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231608Z-alloprof-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232658Z-alloprof-lab_models2_e027f0b8a1a7-heldout.json).

```text
mix-fr-en (heldout), 2320 queries
  A = bench\receipts\heldout\20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.rankings.json.gz: nDCG@10 0.4991
  B = bench\receipts\heldout\20261113T232746Z-mix-fr-en-lab_models2_e027f0b8a1a7-heldout.rankings.json.gz: nDCG@10 0.5423
  B - A = +0.0431  95% CI [+0.0337, +0.0527]  -> B better
```

[Baseline receipt](../receipts/heldout/20261113T231710Z-mix-fr-en-lab_models2_baseline-heldout.json), [candidate receipt](../receipts/heldout/20261113T232746Z-mix-fr-en-lab_models2_e027f0b8a1a7-heldout.json).

