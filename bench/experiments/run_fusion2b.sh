#!/bin/sh
# Controls for fusion2: floored formula at higher alpha, and completion at 0.7, on the same dev sets.
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
BASE='{"dense":{"model":"bekko-a8m","extra":{"model":"gte-small","weight":0.5}},"lexical":{"k1":1.2,"tokenizer":{"stem":"en"}},"fusion":{"kind":"cc","norm":"zscore","alpha":0.55},"chunker":{"kind":"whole"}}'
until grep -q "^DONE" bench/work/fusion2.log; do sleep 10; done
for ds in scifact nfcorpus alloprof mix-fr-en; do
  $PY -I bench/sweep.py fusion2 --datasets $ds --split dev --tag research:fusion --only "alpha=0.7" --base-json "$BASE"
  $PY -I bench/sweep.py fusion2 --datasets $ds --split dev --tag research:fusion --only "F0 control" --base-json "$BASE"
done
echo DONE
