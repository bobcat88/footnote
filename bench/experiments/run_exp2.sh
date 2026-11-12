#!/bin/sh
# E16 (dense ensemble) on four dev datasets, then E12 (gated reranker) on the two French/mixed sets, 200 queries each.
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
BASE='{"dense":{"model":"bekko-a8m"},"lexical":{"k1":1.5,"tokenizer":{"stem":"en"}},"fusion":{"kind":"cc","norm":"zscore","alpha":0.55},"chunker":{"kind":"whole"}}'
for ds in scifact nfcorpus alloprof mix-fr-en; do
  $PY -I bench/sweep.py ensemble --datasets $ds --split dev --tag exp:bekko-a8m --base-json "$BASE"
done
for ds in alloprof mix-fr-en; do
  $PY -I bench/sweep.py gated_rerank --datasets $ds --split dev --tag exp:bekko-a8m --max-queries 200 --base-json "$BASE"
done
echo DONE
