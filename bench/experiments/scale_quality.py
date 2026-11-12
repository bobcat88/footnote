"""E13: does precision hold when the index grows? Same judged queries, more and more distractor documents.

    bench/.labenv/Scripts/python.exe -I bench/experiments/scale_quality.py scifact --sizes 0,10000,100000

Distractors are 150-word spans cut at random from REAL documents of OTHER datasets (never the evaluated one, so no near-duplicate of a
relevant document can appear): SciFact is diluted with NFCorpus, Alloprof and Syntec text, Alloprof with SciFact, NFCorpus and Syntec text.
The spans are not judged; they are topically unrelated or loosely related text, which is what a growing personal corpus looks like.
Config: the frozen default (bekko-a8m, BM25 en stem, convex combination z-score alpha 0.55, whole documents), split heldout.
Rows are appended to bench/sweeps/scale.jsonl.
"""
from __future__ import annotations

import argparse
import json
import random
import statistics
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
sys.path.insert(0, str(BENCH))
sys.path.insert(0, str(BENCH / "lab"))
import fetch  # noqa: E402
import lab  # noqa: E402
import metrics  # noqa: E402

OTHERS = {"scifact": ("nfcorpus", "alloprof", "syntec"), "alloprof": ("scifact", "nfcorpus", "syntec"),
          "nfcorpus": ("scifact", "alloprof", "syntec")}
CFG = {"dense": {"model": "bekko-a8m"}, "lexical": {"k1": 1.5, "tokenizer": {"stem": "en"}},
       "fusion": {"kind": "cc", "norm": "zscore", "alpha": 0.55}, "chunker": {"kind": "whole"}}


def distractors(dataset: str, n: int) -> dict:
    words = []
    for other in OTHERS[dataset]:
        corpus, _, _ = fetch.load(other, "test")
        words += [(t + " " + x).split() for t, x in corpus.values()]
    rng = random.Random(20261112)
    out = {}
    for i in range(n):
        w = rng.choice(words)
        s = rng.randrange(0, max(1, len(w) - 150))
        out[f"zz{i}"] = ("", " ".join(w[s:s + 150]))
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("dataset")
    ap.add_argument("--sizes", default="0,10000,100000")
    ap.add_argument("--split", default="heldout")
    args = ap.parse_args()
    corpus, queries, qrels = fetch.load(args.dataset, args.split)
    for n in (int(x) for x in args.sizes.split(",")):
        big = {**corpus, **distractors(args.dataset, n)}
        eng = lab.LabEngine(f"scale{n}", CFG)
        t0 = time.perf_counter()
        info = eng.prepare(args.dataset, big)
        eng.warm_queries(list(queries.values()))
        build_s = round(time.perf_counter() - t0, 1)
        for mode, kind, shortlist in (("bm25", "lexical", 0), ("dense", "dense", 0), ("hybrid", "cc", 0),
                                      ("hybrid two-stage 1000", "cc", 1000), ("hybrid two-stage 5000", "cc", 5000)):
            eng.cfg["fusion"]["kind"] = kind
            eng.cfg["fusion"]["dense_from_lexical"] = shortlist
            times, rankings = [], {}
            for q, text in queries.items():
                a = time.perf_counter()
                rankings[q] = metrics.dedupe(eng.search(q, text, 20))[:20]
                times.append((time.perf_counter() - a) * 1000)
            m = metrics.evaluate(rankings, qrels)
            row = {"experiment": "E13", "dataset": args.dataset, "split": args.split, "distractors": n, "documents": len(big),
                   "mode": mode, "ndcg@10": round(m["ndcg@10"], 4), "recall@10": round(m["recall@10"], 4),
                   "p50_ms": round(statistics.median(times), 2), "build_s": build_s, "queries": m["queries_scored"]}
            print(json.dumps(row), flush=True)
            with (BENCH / "sweeps" / "scale.jsonl").open("a", encoding="utf-8") as f:
                f.write(json.dumps(row) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
