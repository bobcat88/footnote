#!/bin/sh
# Research-driven fusion experiments (bench/research/fusion-boost.md), dev splits, shipped-ensemble base with Tantivy's k1.
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
BASE='{"dense":{"model":"bekko-a8m","extra":{"model":"gte-small","weight":0.5}},"lexical":{"k1":1.2,"tokenizer":{"stem":"en"}},"fusion":{"kind":"cc","norm":"zscore","alpha":0.55},"chunker":{"kind":"whole"}}'
for ds in scifact nfcorpus alloprof mix-fr-en; do
  $PY -I bench/sweep.py fusion2 --datasets $ds --split dev --tag research:fusion --base-json "$BASE"
done
echo DONE
