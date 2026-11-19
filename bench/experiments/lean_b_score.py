"""Cost per accepted task per arm (#239). Usage: lean_b_score.py runs.jsonl [--price in,write,read,out] (USD per Mtok)."""
import collections, json, sys

def score(rows, price):
    a = collections.defaultdict(lambda: dict(n=0, ok=0, cost=0.0, out=0, turns=0))
    for r in rows:
        s = a[r["arm"]]
        s["n"] += 1
        s["ok"] += bool(r["accepted"])
        s["out"] += r["output"]
        s["turns"] += r["turns"]
        s["cost"] += sum(r[k] * p for k, p in zip(("uncached_in", "cache_write", "cache_read", "output"), price)) / 1e6
    # failed runs stay in the numerator: their spend counts against the arm
    out = {k: dict(v, per_accepted=v["cost"] / v["ok"] if v["ok"] else None) for k, v in a.items()}
    base = (out.get("bare") or {}).get("per_accepted")
    for v in out.values():
        v["vs_bare_pct"] = None if not base or v["per_accepted"] is None else round(100 * (v["per_accepted"] / base - 1), 1)
    return out

if __name__ == "__main__":
    price = (5, 6.25, 0.5, 25)  # placeholder list prices; pass --price to override
    if "--price" in sys.argv:
        price = tuple(float(x) for x in sys.argv[sys.argv.index("--price") + 1].split(","))
    rows = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
    for arm, v in score(rows, price).items():
        print(arm, v)
