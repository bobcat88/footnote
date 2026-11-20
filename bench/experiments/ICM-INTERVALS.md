# Footnote against ICM, paired intervals

Paired bootstrap, 10,000 query resamples, fixed seed, per comparison without multiplicity correction. A = shipped Footnote, B = ICM 0.11.5 keyword-only (no embeddings). Test split.

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T070259Z-syntec-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7952
  B = bench\receipts\20261114T004114Z-syntec-icm.rankings.json.gz: nDCG@10 0.6571
  B - A = -0.1380  95% CI [-0.2013, -0.0756]  -> A better
```

[Footnote receipt](../receipts/20261113T070259Z-syntec-shipped-ensemble-mcp.json), [ICM receipt](../receipts/20261114T004114Z-syntec-icm.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T071411Z-nfcorpus-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.3680
  B = bench\receipts\20261114T005010Z-nfcorpus-icm.rankings.json.gz: nDCG@10 0.3074
  B - A = -0.0605  95% CI [-0.0791, -0.0424]  -> A better
```

[Footnote receipt](../receipts/20261113T071411Z-nfcorpus-shipped-ensemble-mcp.json), [ICM receipt](../receipts/20261114T005010Z-nfcorpus-icm.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T092648Z-scifact-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7467
  B = bench\receipts\20261114T004250Z-scifact-icm.rankings.json.gz: nDCG@10 0.6644
  B - A = -0.0823  95% CI [-0.1102, -0.0547]  -> A better
```

[Footnote receipt](../receipts/20261113T092648Z-scifact-shipped-ensemble-mcp.json), [ICM receipt](../receipts/20261114T004250Z-scifact-icm.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T130039Z-alloprof-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4784
  B = bench\receipts\20261114T004451Z-alloprof-icm.rankings.json.gz: nDCG@10 0.3335
  B - A = -0.1449  95% CI [-0.1582, -0.1319]  -> A better
```

[Footnote receipt](../receipts/20261113T130039Z-alloprof-shipped-ensemble-mcp.json), [ICM receipt](../receipts/20261114T004451Z-alloprof-icm.json).

```text
mix-fr-en (test), 3039 queries
  A = bench\receipts\20261113T162140Z-mix-fr-en-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4890
  B = bench\receipts\20261114T005357Z-mix-fr-en-icm.rankings.json.gz: nDCG@10 0.3424
  B - A = -0.1466  95% CI [-0.1578, -0.1357]  -> A better
```

[Footnote receipt](../receipts/20261113T162140Z-mix-fr-en-shipped-ensemble-mcp.json), [ICM receipt](../receipts/20261114T005357Z-mix-fr-en-icm.json).

