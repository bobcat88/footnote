#!/bin/sh
# Receipts for the shipped binary (resident MCP session, default model pair) on the official test sets.
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
for ds in syntec scifact nfcorpus alloprof mix-fr-en; do
  echo "== $ds"; $PY -I bench/run.py $ds shipped-ensemble-mcp 2>&1 | tail -3
done
echo DONE
