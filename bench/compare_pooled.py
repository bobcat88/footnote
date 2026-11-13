"""Paired bootstrap across several datasets at once, for effects too small to resolve on one set.

    python bench/compare_pooled.py --split heldout lab_ens-k12-a055 lab_ens-k12-a060 scifact nfcorpus alloprof mix-fr-en

Same per-query nDCG@10 as compare.py. The statistic is the MACRO mean of the per-dataset differences (each dataset weighs the same, so one large
set cannot carry the result). Resampling is stratified: queries are resampled within each dataset, 10,000 times, fixed seed.
"""
from __future__ import annotations

import argparse
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare  # noqa: E402
import fetch  # noqa: E402


def latest(engine: str, dataset: str, split: str) -> Path:
    folder = HERE / "receipts" / ("" if split == "test" else split)
    suffix = "" if split == "test" else f"-{split}"
    matches = sorted(folder.glob(f"*-{dataset}-{engine}{suffix}.rankings.json.gz"))
    if not matches:
        raise SystemExit(f"no {split} receipt for {dataset}/{engine}")
    return matches[-1]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("a")
    ap.add_argument("b")
    ap.add_argument("datasets", nargs="+")
    ap.add_argument("--split", default="test")
    ap.add_argument("--resamples", type=int, default=10000)
    args = ap.parse_args()
    per_ds = {}
    for ds in args.datasets:
        _, _, qrels = fetch.load(ds, args.split)
        pa = compare.per_query(compare.load_rankings(str(latest(args.a, ds, args.split)), ds), qrels)
        pb = compare.per_query(compare.load_rankings(str(latest(args.b, ds, args.split)), ds), qrels)
        qs = sorted(set(pa) & set(pb))
        per_ds[ds] = [pb[q] - pa[q] for q in qs]
        d = sum(per_ds[ds]) / len(qs)
        print(f"  {ds:<10} {len(qs):>5} queries  B - A = {d:+.4f}")
    rng = random.Random(20261113)
    macro = lambda samples: sum(sum(s) / len(s) for s in samples) / len(samples)
    observed = macro(list(per_ds.values()))
    boots = sorted(macro([rng.choices(v, k=len(v)) for v in per_ds.values()]) for _ in range(args.resamples))
    lo, hi = boots[int(0.025 * args.resamples)], boots[int(0.975 * args.resamples)]
    verdict = "B better" if lo > 0 else "A better" if hi < 0 else "no consistent difference"
    print(f"macro mean B - A = {observed:+.4f}  95% CI [{lo:+.4f}, {hi:+.4f}]  -> {verdict}  ({args.split}, {len(per_ds)} datasets)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
