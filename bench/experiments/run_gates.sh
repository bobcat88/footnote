#!/bin/sh
# MTEB-reproduction gates for the candidate models (bench/research/embedding-boost.md). A model that fails is not used.
PY=bench/.labenv/Scripts/python.exe
export PYTHONDONTWRITEBYTECODE=1
$PY -I bench/lab/check_model.py harrier-270m syntec 2>&1 | tail -2
$PY -I bench/lab/check_model.py embeddinggemma-300m nfcorpus 2>&1 | tail -2
$PY -I bench/lab/check_model.py arctic-m-v2 nfcorpus 2>&1 | tail -2
echo DONE
