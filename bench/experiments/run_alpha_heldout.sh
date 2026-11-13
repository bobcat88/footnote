#!/bin/sh
# Pooled heldout check of alpha 0.60 against 0.55 for the two-model blend (F1-F4 control result).
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
for ds in scifact nfcorpus alloprof mix-fr-en; do
  for e in ens-k12-a055 ens-k12-a060; do $PY -I bench/run.py $ds lab:$e --split heldout --no-repeat 2>&1 | grep "nDCG@10" | cut -c1-40 | sed "s/^/$ds $e /"; done
done
echo DONE
