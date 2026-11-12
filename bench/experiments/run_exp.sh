#!/bin/sh
# E01..E11 on dev, default product config (bekko-a8m). One process per dataset so memory is released between corpora.
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
BASE='{"dense":{"model":"bekko-a8m"},"lexical":{"k1":1.5,"tokenizer":{"stem":"en"}},"fusion":{"kind":"cc","norm":"zscore","alpha":0.55},"chunker":{"kind":"whole"}}'
for ds in scifact nfcorpus alloprof mix-fr-en; do
  $PY -I bench/sweep.py experiments --datasets $ds --split dev --tag exp:bekko-a8m --base-json "$BASE"
done
echo DONE
