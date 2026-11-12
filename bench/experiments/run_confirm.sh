#!/bin/sh
# Heldout confirmation of the optimization candidates (receipts under bench/receipts/heldout).
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
for ds in scifact nfcorpus alloprof mix-fr-en; do
  for e in ens-bekko-gte ml-bekko-a8m-int8-two-stage ens-bekko-gte-two-stage; do
    echo "== $ds $e"; $PY -I bench/run.py $ds lab:$e --split heldout 2>&1 | tail -3
  done
done
echo DONE
