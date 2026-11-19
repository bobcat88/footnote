import sys
from collections import Counter, defaultdict
for d in sys.argv[1:]:
    oracle = set()
    for l in open(f"{d}.oracle.tsv", encoding="utf-8"):
        f, ln, n, t = l.rstrip("\n").split("\t")
        oracle.add((f, int(ln), n, t))
    files = {o[0] for o in oracle}
    print(f"== {d}: oracle edges {len(oracle)}")
    for v in ("base", "new"):
        links, right, byrule = set(), set(), defaultdict(Counter)
        for l in open(f"{d}.{v}.tsv", encoding="utf-8"):
            f, ln, n, t, r = l.rstrip("\n").split("\t")[:5]
            f, t = f.replace("\\", "/"), t.replace("\\", "/")
            if f not in files:
                continue
            k = (f, int(ln), n, t)
            if k in links:
                continue
            links.add(k)
            ok = k in oracle
            byrule[r]["n"] += 1
            byrule[r]["ok"] += ok
            if ok:
                right.add(k)
        p = len(right) / max(len(links), 1)
        rc = len(right) / max(len(oracle), 1)
        print(f"  {v:4} links {len(links):5} right {len(right):5} precision {p:.2f} recall {rc:.2f}")
        for r, c in sorted(byrule.items()):
            print(f"       {r:14} {c['n']:5} at {c['ok']/c['n']:.2f}")
