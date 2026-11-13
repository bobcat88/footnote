#!/bin/sh
# Head-to-head on identical inputs: Footnote (shipped binary, resident MCP), mem0 2.2.1, Cognee 1.6.8. Same datasets, metrics and receipts.
# Starts when the model gates are done so latency is measured on a quiet machine. Cheapest datasets first; failures are recorded, not skipped.
export PYTHONDONTWRITEBYTECODE=1
until grep -q "^DONE" bench/work/gates.log; do sleep 20; done
LAB=bench/.labenv/Scripts/python.exe; MEM0=bench/.venv-mem0/Scripts/python.exe; COG=bench/.venv-cognee/Scripts/python.exe
run() { echo "== $2 $3"; $1 -I bench/run.py $2 $3 2>&1 | grep -E "nDCG@10|p50|Error|error:" | grep -v INFO | cut -c1-160; }
for ds in syntec nfcorpus scifact alloprof; do
  run $LAB  $ds shipped-ensemble-mcp
  run $MEM0 $ds ext:mem0
  run $MEM0 $ds ext:mem0-multilingual
  run $COG  $ds ext:cognee
  run $COG  $ds ext:cognee-lexical
  run $COG  $ds ext:cognee-multilingual
done
run $LAB mix-fr-en shipped-ensemble-mcp
run $MEM0 mix-fr-en ext:mem0-multilingual
run $COG mix-fr-en ext:cognee-multilingual
echo DONE
