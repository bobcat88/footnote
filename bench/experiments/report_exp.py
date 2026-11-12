"""Markdown tables for the optimization experiments, straight from sweeps/*.jsonl (read-only).

    python bench/experiments/report_exp.py > bench/experiments/TABLES.md

One row per experiment, one column per dev dataset, each cell the nDCG@10 change against that dataset's base row (E00 or the E16 base).
"""
import collections
import json
from pathlib import Path

SW = Path(__file__).resolve().parent.parent / "sweeps"
DS = ["scifact", "nfcorpus", "alloprof", "mix-fr-en"]


def load(name):
    p = SW / f"{name}.jsonl"
    return [json.loads(l) for l in p.read_text(encoding="utf-8").splitlines() if l.strip()] if p.exists() else []


def table(rows, base_label, title):
    by = collections.defaultdict(dict)
    for r in rows:
        if "ndcg@10" in r and r["split"] == "dev" and r.get("tag", "").startswith("exp"):
            by[r["label"]][r["dataset"]] = r["ndcg@10"]
    base = by.get(base_label, {})
    print(f"\n### {title}\n\n| Experiment | " + " | ".join(DS) + " |\n|---|" + "---|" * len(DS))
    print(f"| *{base_label}* (nDCG@10) | " + " | ".join(f"{base.get(d, float('nan')):.4f}" for d in DS) + " |")
    for label, vals in by.items():
        if label == base_label:
            continue
        cells = [f"{vals[d] - base[d]:+.4f}" if d in vals and d in base else "-" for d in DS]
        print(f"| {label} | " + " | ".join(cells) + " |")


table(load("experiments"), "E00 base", "E01 to E11: one change at a time")
table(load("ensemble"), "E16 base (one model)", "E16: blending a second dense model")
gated = load("gated_rerank")
if gated:
    table(gated, "E12 no rerank", "E12: reranker only where the dense gap is small (200 queries per dataset)")
