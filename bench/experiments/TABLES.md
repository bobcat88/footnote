### E01 to E11: one change at a time

| Experiment | scifact | nfcorpus | alloprof | mix-fr-en |
|---|---|---|---|---|
| *E00 base* (nDCG@10) | 0.7485 | 0.3369 | 0.4832 | 0.5661 |
| E01 bm25 k1=1.2 b=0.75 (Tantivy fixed) | +0.0015 | +0.0010 | -0.0052 | -0.0008 |
| E01 bm25 k1=2.0 b=0.75 | -0.0008 | +0.0009 | +0.0037 | -0.0008 |
| E01 bm25 k1=1.5 b=0.5 | +0.0027 | +0.0005 | -0.0001 | -0.0015 |
| E02 drop query words with idf<1.0 | -0.0001 | +0.0005 | -0.0034 | +0.0006 |
| E02 drop query words with idf<2.0 | +0.0020 | -0.0013 | -0.0053 | -0.0060 |
| E02 drop query words with idf<3.0 | -0.0056 | -0.0049 | -0.0139 | -0.0089 |
| E03 lexical PRF docs=3 terms=8 w=0.3 | -0.0118 | -0.0019 | -0.0206 | -0.0127 |
| E03 lexical PRF docs=5 terms=10 w=0.5 | -0.0197 | -0.0071 | -0.0299 | -0.0215 |
| E03 lexical PRF docs=5 terms=20 w=0.3 | -0.0081 | -0.0049 | -0.0288 | -0.0158 |
| E04 title boost x2 | +0.0035 | +0.0021 | +0.0000 | +0.0005 |
| E04 title boost x3 | +0.0041 | +0.0024 | +0.0000 | +0.0000 |
| E05 dense PRF docs=3 beta=0.3 | +0.0014 | +0.0074 | -0.0152 | -0.0070 |
| E05 dense PRF docs=3 beta=0.6 | -0.0006 | +0.0085 | -0.0223 | -0.0114 |
| E05 dense PRF docs=5 beta=0.5 | +0.0025 | +0.0065 | -0.0138 | -0.0061 |
| E06 adaptive alpha 0.4..0.8 | +0.0025 | +0.0027 | -0.0096 | -0.0065 |
| E06 adaptive alpha 0.3..0.9 | +0.0026 | +0.0014 | -0.0136 | -0.0096 |
| E07 fusion pool=20 | +0.0008 | -0.0053 | -0.0201 | -0.0096 |
| E07 fusion pool=50 | +0.0024 | -0.0012 | -0.0073 | -0.0032 |
| E07 fusion pool=200 | +0.0020 | -0.0000 | +0.0010 | +0.0014 |
| E08 two-stage dense on lexical top-100 | -0.0056 | -0.0102 | -0.0276 | -0.0218 |
| E08 two-stage dense on lexical top-1000 | +0.0015 | -0.0025 | -0.0010 | -0.0015 |
| E09 vectors f16 | +0.0000 | +0.0000 | -0.0004 | +0.0000 |
| E10 vectors int8 | +0.0001 | +0.0001 | -0.0003 | +0.0001 |
| E11 dims=256 | -0.0020 | +0.0015 | -0.0131 | -0.0089 |
| E11 dims=192 | +0.0025 | -0.0035 | -0.0232 | -0.0120 |
| E11 dims=128 | -0.0024 | -0.0091 | -0.0491 | -0.0249 |

### E16: blending a second dense model

| Experiment | scifact | nfcorpus | alloprof | mix-fr-en |
|---|---|---|---|---|
| *E16 base (one model)* (nDCG@10) | 0.7485 | 0.3369 | 0.4832 | 0.5661 |
| E16 + e5-small-multi weight=0.3 | +0.0080 | +0.0011 | +0.0021 | +0.0035 |
| E16 + e5-small-multi weight=0.5 | +0.0104 | -0.0016 | -0.0022 | +0.0027 |
| E16 + minilm-l12-multi weight=0.3 | +0.0023 | +0.0043 | +0.0075 | +0.0040 |
| E16 + minilm-l12-multi weight=0.5 | -0.0038 | -0.0021 | -0.0006 | -0.0054 |
| E16 + gte-small weight=0.3 | +0.0096 | +0.0135 | +0.0203 | +0.0118 |
| E16 + gte-small weight=0.5 | +0.0122 | +0.0177 | +0.0237 | +0.0150 |

### E12: reranker only where the dense gap is small (200 queries per dataset)

| Experiment | scifact | nfcorpus | alloprof | mix-fr-en |
|---|---|---|---|---|
| *E12 no rerank* (nDCG@10) | nan | nan | 0.5083 | 0.4913 |
| E12 bge-reranker-v2-m3 pool=10 gate=0.0 | - | - | +0.0385 | - |
| E12 bge-reranker-v2-m3 pool=10 gate=0.01 | - | - | +0.0125 | - |
| E12 bge-reranker-v2-m3 pool=10 gate=0.03 | - | - | +0.0333 | - |
| E12 bge-reranker-v2-m3 pool=10 gate=0.06 | - | - | +0.0260 | - |
