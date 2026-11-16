"""Development-set scoring for DOCS-OPTIMIZE-2026-11-16.md: one Footnote binary, both seen documentation sets.

    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_dev.py <label> <footnote.exe> [extra search args]

Indexes each set's exported pages (Grounded Docs' crawl) into bench/work/dev/<label>/<set>/<lib>.db, searches hybrid with the
shipped pair and `--per-source 1` (8 at a time; quality only), scores against that set's Grounded Docs hybrid run, and compares
with the label `baseline` when it exists.
"""
from __future__ import annotations

import json
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import yaml

ARGS, sys.argv = sys.argv, sys.argv[:1]
sys.path.insert(0, str(Path(__file__).parent))
import docs_vs_grounded as g  # noqa: E402

SETS = {"gd": (g.REPO / "bench/work/gd/run1", g.REPO / "bench/work/gd/dataset.yaml"),
        "confirm": (g.REPO / "bench/work/confirm/run1", g.REPO / "bench/datasets/docs-confirm/dataset.yaml")}
OUT = g.REPO / "bench/work/dev"


def run(label: str, extra: list[str]) -> dict:
    scores = {}
    for name, (run_dir, dataset) in SETS.items():
        qs = [e["vars"] for e in yaml.safe_load(dataset.read_text(encoding="utf-8"))["entries"]]
        manifest = json.loads((run_dir / "manifest.json").read_text(encoding="utf-8"))
        folder = OUT / label / name
        folder.mkdir(parents=True, exist_ok=True)
        for lib in sorted({q["library"] for q in qs}):
            db = folder / f"{lib}.db"
            if not db.exists():
                t = time.time()
                g.fn_build(db, run_dir / "corpus" / lib)
                print(f"{time.strftime('%H:%M:%S')} {label}/{name}/{lib} built in {time.time() - t:.0f} s", flush=True)
        args = ["--limit", "5", "--per-source", "1", "--mode", "hybrid", *[a for m in g.MODELS for a in ("--model", m)], *extra]

        def one(i: int):
            q = qs[i]
            hits = g.fn(folder / f"{q['library']}.db", "search", q["query"], *args)["results"]
            return [manifest.get(g.key(h["path"]), h["path"]) for h in hits]

        with ThreadPoolExecutor(8) as ex:
            ranked = list(ex.map(one, range(len(qs))))
        scores[name] = {"ndcg_at_5": [g.metrics(r, q["qrels"])["ndcg_at_5"] for r, q in zip(ranked, qs)],
                        "library": [q["library"] for q in qs], "ranked": ranked}
    (OUT / label / "scores.json").write_text(json.dumps(scores), encoding="utf-8")
    return scores


def report(label: str, scores: dict) -> None:
    base_file = OUT / "baseline" / "scores.json"
    base = json.loads(base_file.read_text(encoding="utf-8")) if base_file.exists() and label != "baseline" else None
    pooled, pooled_base = [], []
    for name, s in scores.items():
        nd = s["ndcg_at_5"]
        line = f"{label:12} {name:8} nDCG@5 {sum(nd) / len(nd):.4f}"
        if base:
            b = base[name]["ndcg_at_5"]
            d, lo, hi = g.bootstrap(nd, b)
            line += f"  vs baseline {d:+.4f} [{lo:+.4f}, {hi:+.4f}]"
            pooled_base += b
        pooled += nd
        print(line, flush=True)
    line = f"{label:12} pooled   nDCG@5 {sum(pooled) / len(pooled):.4f}"
    if base:
        d, lo, hi = g.bootstrap(pooled, pooled_base)
        line += f"  vs baseline {d:+.4f} [{lo:+.4f}, {hi:+.4f}]"
    print(line, flush=True)


if __name__ == "__main__":
    label, g.FOOTNOTE = ARGS[1], ARGS[2]
    report(label, run(label, ARGS[3:]))
