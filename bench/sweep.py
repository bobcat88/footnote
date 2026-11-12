"""Run a named family of lab configurations on a dataset split and append the rows to bench/sweeps/<sweep>.jsonl.

    bench/.labenv/Scripts/python.exe -I bench/sweep.py models --datasets scifact,nfcorpus --split dev

Sweeps are TUNING runs: use --split dev. Reporting happens through run.py with receipts. Rows are append-only, never edited.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE / "lab"))
import fetch  # noqa: E402
import lab  # noqa: E402
import metrics  # noqa: E402
import sweeps  # noqa: E402

OUT = HERE / "sweeps"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("sweep", choices=sorted(sweeps.SWEEPS))
    ap.add_argument("--datasets", required=True)
    ap.add_argument("--split", default="dev", choices=["dev", "heldout", "test"])
    ap.add_argument("--only", help="substring filter on config labels")
    ap.add_argument("--base-json", default="{}", help="base lab config the sweep varies (the previous stage's winner)")
    ap.add_argument("--max-queries", type=int, help="use only the first N queries (slow stages such as reranking)")
    ap.add_argument("--tag", default="", help="free label stored in every row (e.g. the profile: en or mix)")
    args = ap.parse_args()
    OUT.mkdir(exist_ok=True)
    sink = OUT / f"{args.sweep}.jsonl"
    for ds in args.datasets.split(","):
        corpus, queries, qrels = fetch.load(ds, args.split)
        if args.max_queries:
            queries = dict(list(queries.items())[: args.max_queries])
            qrels = {q: qrels[q] for q in queries}
        base = lab.merge(lab.DEFAULT, json.loads(args.base_json))
        for label, cfg in sweeps.SWEEPS[args.sweep](ds, base):
            if args.only and args.only not in label:
                continue
            t0 = time.perf_counter()
            try:
                eng = lab.LabEngine(label, cfg)
                info = eng.prepare(ds, corpus)
                eng.warm_queries(list(queries.values()))
                rankings = {q: metrics.dedupe(eng.search(q, text, 20))[:20] for q, text in queries.items()}
                m = metrics.evaluate(rankings, qrels)
                row = {"sweep": args.sweep, "dataset": ds, "split": args.split, "label": label, "cfg": eng.cfg,
                       "ndcg@10": round(m["ndcg@10"], 5), "recall@10": round(m["recall@10"], 5), "mrr@10": round(m["mrr@10"], 5),
                       "empty": m["queries_empty"], "queries": m["queries_scored"], "tag": args.tag, "chunks": info["chunks"],
                       "embed_ms": round(info["embed_ms"]), "seconds": round(time.perf_counter() - t0, 1)}
                if getattr(eng, "rr_total", 0) and eng.cfg["rerank"]["on"]:
                    row["reranked_share"] = round(eng.rr_calls / eng.rr_total, 3)
            except Exception as e:  # a failure is a result: record it and continue
                row = {"sweep": args.sweep, "dataset": ds, "split": args.split, "label": label, "error": f"{type(e).__name__}: {e}"[:300]}
            with sink.open("a", encoding="utf-8") as f:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
            print(f"{ds:<10} {label:<46} " + (f"ndcg@10={row['ndcg@10']:.4f} R@10={row['recall@10']:.4f} ({row['seconds']}s)" if "ndcg@10" in row else row["error"]),
                  flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
