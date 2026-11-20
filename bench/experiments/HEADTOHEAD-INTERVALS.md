# Paired retrieval intervals

Paired bootstrap, 10,000 query resamples and fixed seed. Per-comparison intervals, without multiplicity correction. Competitor configurations differ in models/chunking; matched controls use identical frozen vectors and exclude encoding.

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T070259Z-syntec-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7952
  B = bench\receipts\20261113T070305Z-syntec-mem0.rankings.json.gz: nDCG@10 0.7274
  B - A = -0.0678  95% CI [-0.1168, -0.0195]  -> A better
```

[Baseline](../receipts/20261113T070259Z-syntec-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T070305Z-syntec-mem0.json).

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T070259Z-syntec-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7952
  B = bench\receipts\20261113T070313Z-syntec-mem0-multilingual.rankings.json.gz: nDCG@10 0.7447
  B - A = -0.0504  95% CI [-0.0975, -0.0036]  -> A better
```

[Baseline](../receipts/20261113T070259Z-syntec-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T070313Z-syntec-mem0-multilingual.json).

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T070259Z-syntec-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7952
  B = bench\receipts\20261113T070403Z-syntec-cognee.rankings.json.gz: nDCG@10 0.5529
  B - A = -0.2423  95% CI [-0.3035, -0.1821]  -> A better
```

[Baseline](../receipts/20261113T070259Z-syntec-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T070403Z-syntec-cognee.json).

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T070259Z-syntec-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7952
  B = bench\receipts\20261113T070447Z-syntec-cognee-lexical.rankings.json.gz: nDCG@10 0.7002
  B - A = -0.0950  95% CI [-0.1504, -0.0404]  -> A better
```

[Baseline](../receipts/20261113T070259Z-syntec-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T070447Z-syntec-cognee-lexical.json).

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T070259Z-syntec-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7952
  B = bench\receipts\20261113T070558Z-syntec-cognee-multilingual.rankings.json.gz: nDCG@10 0.6821
  B - A = -0.1130  95% CI [-0.1746, -0.0515]  -> A better
```

[Baseline](../receipts/20261113T070259Z-syntec-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T070558Z-syntec-cognee-multilingual.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T071411Z-nfcorpus-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.3680
  B = bench\receipts\20261113T071753Z-nfcorpus-mem0.rankings.json.gz: nDCG@10 0.3575
  B - A = -0.0105  95% CI [-0.0223, +0.0017]  -> no consistent difference
```

[Baseline](../receipts/20261113T071411Z-nfcorpus-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T071753Z-nfcorpus-mem0.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T071411Z-nfcorpus-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.3680
  B = bench\receipts\20261113T072026Z-nfcorpus-mem0-multilingual.rankings.json.gz: nDCG@10 0.2958
  B - A = -0.0722  95% CI [-0.0917, -0.0534]  -> A better
```

[Baseline](../receipts/20261113T071411Z-nfcorpus-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T072026Z-nfcorpus-mem0-multilingual.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T071411Z-nfcorpus-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.3680
  B = bench\receipts\20261113T075702Z-nfcorpus-cognee.rankings.json.gz: nDCG@10 0.3385
  B - A = -0.0295  95% CI [-0.0439, -0.0154]  -> A better
```

[Baseline](../receipts/20261113T071411Z-nfcorpus-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T075702Z-nfcorpus-cognee.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T071411Z-nfcorpus-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.3680
  B = bench\receipts\20261113T083346Z-nfcorpus-cognee-lexical.rankings.json.gz: nDCG@10 0.3113
  B - A = -0.0567  95% CI [-0.0736, -0.0400]  -> A better
  WARNING B: 20261113T083346Z-nfcorpus-cognee-lexical.json is not reproducible (rankings not identical when 200 queries were repeated); this interval cannot support a reproducible-superiority claim
```

[Baseline](../receipts/20261113T071411Z-nfcorpus-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T083346Z-nfcorpus-cognee-lexical.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T071411Z-nfcorpus-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.3680
  B = bench\receipts\20261113T091511Z-nfcorpus-cognee-multilingual.rankings.json.gz: nDCG@10 0.2508
  B - A = -0.1172  95% CI [-0.1402, -0.0954]  -> A better
```

[Baseline](../receipts/20261113T071411Z-nfcorpus-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T091511Z-nfcorpus-cognee-multilingual.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T092648Z-scifact-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7467
  B = bench\receipts\20261113T093238Z-scifact-mem0.rankings.json.gz: nDCG@10 0.7285
  B - A = -0.0182  95% CI [-0.0397, +0.0029]  -> no consistent difference
```

[Baseline](../receipts/20261113T092648Z-scifact-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T093238Z-scifact-mem0.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T092648Z-scifact-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7467
  B = bench\receipts\20261113T093624Z-scifact-mem0-multilingual.rankings.json.gz: nDCG@10 0.5545
  B - A = -0.1922  95% CI [-0.2319, -0.1542]  -> A better
```

[Baseline](../receipts/20261113T092648Z-scifact-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T093624Z-scifact-mem0-multilingual.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T092648Z-scifact-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7467
  B = bench\receipts\20261113T103721Z-scifact-cognee.rankings.json.gz: nDCG@10 0.7192
  B - A = -0.0275  95% CI [-0.0515, -0.0037]  -> A better
```

[Baseline](../receipts/20261113T092648Z-scifact-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T103721Z-scifact-cognee.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T092648Z-scifact-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7467
  B = bench\receipts\20261113T113943Z-scifact-cognee-lexical.rankings.json.gz: nDCG@10 0.6642
  B - A = -0.0825  95% CI [-0.1116, -0.0546]  -> A better
```

[Baseline](../receipts/20261113T092648Z-scifact-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T113943Z-scifact-cognee-lexical.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T092648Z-scifact-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.7467
  B = bench\receipts\20261113T124851Z-scifact-cognee-multilingual.rankings.json.gz: nDCG@10 0.5733
  B - A = -0.1734  95% CI [-0.2141, -0.1354]  -> A better
```

[Baseline](../receipts/20261113T092648Z-scifact-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T124851Z-scifact-cognee-multilingual.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T130039Z-alloprof-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4784
  B = bench\receipts\20261113T130823Z-alloprof-mem0.rankings.json.gz: nDCG@10 0.3597
  B - A = -0.1187  95% CI [-0.1318, -0.1051]  -> A better
```

[Baseline](../receipts/20261113T130039Z-alloprof-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T130823Z-alloprof-mem0.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T130039Z-alloprof-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4784
  B = bench\receipts\20261113T131406Z-alloprof-mem0-multilingual.rankings.json.gz: nDCG@10 0.3538
  B - A = -0.1246  95% CI [-0.1393, -0.1097]  -> A better
```

[Baseline](../receipts/20261113T130039Z-alloprof-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T131406Z-alloprof-mem0-multilingual.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T130039Z-alloprof-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4784
  B = bench\receipts\20261113T135959Z-alloprof-cognee.rankings.json.gz: nDCG@10 0.2887
  B - A = -0.1898  95% CI [-0.2045, -0.1752]  -> A better
```

[Baseline](../receipts/20261113T130039Z-alloprof-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T135959Z-alloprof-cognee.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T130039Z-alloprof-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4784
  B = bench\receipts\20261113T145930Z-alloprof-cognee-lexical.rankings.json.gz: nDCG@10 0.3052
  B - A = -0.1732  95% CI [-0.1869, -0.1600]  -> A better
```

[Baseline](../receipts/20261113T130039Z-alloprof-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T145930Z-alloprof-cognee-lexical.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T130039Z-alloprof-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4784
  B = bench\receipts\20261113T155038Z-alloprof-cognee-multilingual.rankings.json.gz: nDCG@10 0.2966
  B - A = -0.1818  95% CI [-0.1972, -0.1662]  -> A better
```

[Baseline](../receipts/20261113T130039Z-alloprof-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T155038Z-alloprof-cognee-multilingual.json).

```text
mix-fr-en (test), 3039 queries
  A = bench\receipts\20261113T162140Z-mix-fr-en-shipped-ensemble-mcp.rankings.json.gz: nDCG@10 0.4890
  B = bench\receipts\20261113T164258Z-mix-fr-en-mem0-multilingual.rankings.json.gz: nDCG@10 0.3434
  B - A = -0.1456  95% CI [-0.1581, -0.1330]  -> A better
```

[Baseline](../receipts/20261113T162140Z-mix-fr-en-shipped-ensemble-mcp.json), [candidate](../receipts/20261113T164258Z-mix-fr-en-mem0-multilingual.json).

**No comparison for mix-fr-en / cognee-multilingual:** no complete receipt (the run failed or was not made).

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T232749Z-syntec-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.8207
  B = bench\receipts\20261113T232752Z-syntec-matched-lance-rrf.rankings.json.gz: nDCG@10 0.8078
  B - A = -0.0129  95% CI [-0.0444, +0.0175]  -> no consistent difference
```

[Baseline](../receipts/20261113T232749Z-syntec-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232752Z-syntec-matched-lance-rrf.json).

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T232749Z-syntec-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.8207
  B = bench\receipts\20261113T232754Z-syntec-matched-lance-cc.rankings.json.gz: nDCG@10 0.8281
  B - A = +0.0074  95% CI [-0.0063, +0.0228]  -> no consistent difference
```

[Baseline](../receipts/20261113T232749Z-syntec-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232754Z-syntec-matched-lance-cc.json).

```text
syntec (test), 100 queries
  A = bench\receipts\20261113T232749Z-syntec-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.8207
  B = bench\receipts\20261113T232755Z-syntec-matched-lance-tantivy.rankings.json.gz: nDCG@10 0.8207
  B - A = +0.0000  95% CI [+0.0000, +0.0000]  -> no consistent difference
```

[Baseline](../receipts/20261113T232749Z-syntec-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232755Z-syntec-matched-lance-tantivy.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T232758Z-nfcorpus-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.3448
  B = bench\receipts\20261113T232804Z-nfcorpus-matched-lance-rrf.rankings.json.gz: nDCG@10 0.3463
  B - A = +0.0015  95% CI [-0.0063, +0.0093]  -> no consistent difference
```

[Baseline](../receipts/20261113T232758Z-nfcorpus-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232804Z-nfcorpus-matched-lance-rrf.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T232758Z-nfcorpus-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.3448
  B = bench\receipts\20261113T232811Z-nfcorpus-matched-lance-cc.rankings.json.gz: nDCG@10 0.3450
  B - A = +0.0002  95% CI [-0.0008, +0.0012]  -> no consistent difference
```

[Baseline](../receipts/20261113T232758Z-nfcorpus-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232811Z-nfcorpus-matched-lance-cc.json).

```text
nfcorpus (test), 323 queries
  A = bench\receipts\20261113T232758Z-nfcorpus-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.3448
  B = bench\receipts\20261113T232814Z-nfcorpus-matched-lance-tantivy.rankings.json.gz: nDCG@10 0.3448
  B - A = +0.0000  95% CI [+0.0000, +0.0000]  -> no consistent difference
```

[Baseline](../receipts/20261113T232758Z-nfcorpus-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232814Z-nfcorpus-matched-lance-tantivy.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T232817Z-scifact-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.7319
  B = bench\receipts\20261113T232825Z-scifact-matched-lance-rrf.rankings.json.gz: nDCG@10 0.7234
  B - A = -0.0085  95% CI [-0.0223, +0.0052]  -> no consistent difference
```

[Baseline](../receipts/20261113T232817Z-scifact-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232825Z-scifact-matched-lance-rrf.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T232817Z-scifact-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.7319
  B = bench\receipts\20261113T232833Z-scifact-matched-lance-cc.rankings.json.gz: nDCG@10 0.7323
  B - A = +0.0004  95% CI [-0.0034, +0.0040]  -> no consistent difference
```

[Baseline](../receipts/20261113T232817Z-scifact-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232833Z-scifact-matched-lance-cc.json).

```text
scifact (test), 300 queries
  A = bench\receipts\20261113T232817Z-scifact-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.7319
  B = bench\receipts\20261113T232836Z-scifact-matched-lance-tantivy.rankings.json.gz: nDCG@10 0.7319
  B - A = +0.0000  95% CI [+0.0000, +0.0000]  -> no consistent difference
```

[Baseline](../receipts/20261113T232817Z-scifact-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232836Z-scifact-matched-lance-tantivy.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T232845Z-alloprof-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.4622
  B = bench\receipts\20261113T232919Z-alloprof-matched-lance-rrf.rankings.json.gz: nDCG@10 0.4509
  B - A = -0.0113  95% CI [-0.0185, -0.0041]  -> A better
```

[Baseline](../receipts/20261113T232845Z-alloprof-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232919Z-alloprof-matched-lance-rrf.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T232845Z-alloprof-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.4622
  B = bench\receipts\20261113T232953Z-alloprof-matched-lance-cc.rankings.json.gz: nDCG@10 0.4617
  B - A = -0.0005  95% CI [-0.0028, +0.0019]  -> no consistent difference
```

[Baseline](../receipts/20261113T232845Z-alloprof-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T232953Z-alloprof-matched-lance-cc.json).

```text
alloprof (test), 2316 queries
  A = bench\receipts\20261113T232845Z-alloprof-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.4622
  B = bench\receipts\20261113T233004Z-alloprof-matched-lance-tantivy.rankings.json.gz: nDCG@10 0.4622
  B - A = +0.0000  95% CI [+0.0000, +0.0000]  -> no consistent difference
```

[Baseline](../receipts/20261113T232845Z-alloprof-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T233004Z-alloprof-matched-lance-tantivy.json).

```text
mix-fr-en (test), 3039 queries
  A = bench\receipts\20261113T233019Z-mix-fr-en-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.4757
  B = bench\receipts\20261113T233131Z-mix-fr-en-matched-lance-rrf.rankings.json.gz: nDCG@10 0.4630
  B - A = -0.0126  95% CI [-0.0185, -0.0067]  -> A better
```

[Baseline](../receipts/20261113T233019Z-mix-fr-en-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T233131Z-mix-fr-en-matched-lance-rrf.json).

```text
mix-fr-en (test), 3039 queries
  A = bench\receipts\20261113T233019Z-mix-fr-en-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.4757
  B = bench\receipts\20261113T233242Z-mix-fr-en-matched-lance-cc.rankings.json.gz: nDCG@10 0.4759
  B - A = +0.0002  95% CI [-0.0019, +0.0023]  -> no consistent difference
```

[Baseline](../receipts/20261113T233019Z-mix-fr-en-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T233242Z-mix-fr-en-matched-lance-cc.json).

```text
mix-fr-en (test), 3039 queries
  A = bench\receipts\20261113T233019Z-mix-fr-en-matched-sqlite-tantivy.rankings.json.gz: nDCG@10 0.4757
  B = bench\receipts\20261113T233301Z-mix-fr-en-matched-lance-tantivy.rankings.json.gz: nDCG@10 0.4757
  B - A = +0.0000  95% CI [+0.0000, +0.0000]  -> no consistent difference
```

[Baseline](../receipts/20261113T233019Z-mix-fr-en-matched-sqlite-tantivy.json), [candidate](../receipts/20261113T233301Z-mix-fr-en-matched-lance-tantivy.json).

