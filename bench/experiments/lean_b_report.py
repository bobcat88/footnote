"""Table and horizontal stacked-bar chart for lean_b runs.jsonl (#239). Usage: lean_b_report.py runs.jsonl out.png"""
import collections, json, sys
import matplotlib; matplotlib.use("Agg")
import matplotlib.pyplot as plt
rows = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8")]
A = collections.defaultdict(list)
for r in rows: A[r["arm"]].append(r)
order = [a for a in ("bare", "ponytail", "lean-b-code", "lean-b-full") if a in A]
m = lambda a, k: sum(r[k] for r in A[a]) / len(A[a])
def tok(a): return m(a, "uncached_in") + m(a, "cache_write"), m(a, "cache_read"), m(a, "output")
def rel(v, b, word):
    p = round(100 * (v - b) / b) if b else 0
    return "baseline" if v == b else f"{abs(p)}% {word[0] if p < 0 else word[1]}"
base = order[0]; print("| arm | runs | uncached in (incl. cache write) | cached in | output | time | accepted |\n|---|---|---|---|---|---|---|")
for a in order:
    u, c, o = tok(a); bu, bc, bo = tok(base); t = m(a, "seconds"); ok = sum(r["accepted"] for r in A[a])
    print(f"| {a} | {len(A[a])} | {u:,.0f} ({rel(u,bu,('less','more'))}) | {c:,.0f} ({rel(c,bc,('less','more'))}) | {o:,.0f} ({rel(o,bo,('less','more'))}) "
          f"| {t:.0f} s ({rel(t,m(base,'seconds'),('faster','slower'))}) | {ok}/{len(A[a])} ({rel(ok/len(A[a]),sum(r['accepted'] for r in A[base])/len(A[base]),('lower','higher'))}) |")
fig, ax = plt.subplots(figsize=(11, 3.6)); y = range(len(order))[::-1]
for yy, a in zip(y, order):
    u, c, o = tok(a); l = 0
    for v, col, n in ((u, "#c0392b", "uncached in"), (c, "#7fb3d5", "cached in"), (o, "#27ae60", "output")):
        ax.barh(yy, v, left=l, color=col, label=n if yy == max(y) else None); ax.text(l + v / 2, yy, f"{v:,.0f}", ha="center", va="center", fontsize=8); l += v
    ok = sum(r["accepted"] for r in A[a]); ax.text(l * 1.01, yy, f"{m(a,'seconds'):.0f} s, {ok}/{len(A[a])} accepted", va="center", fontsize=9)
ax.set_yticks(list(y)); ax.set_yticklabels(order); ax.set_xlim(0, ax.get_xlim()[1] * 1.3); ax.set_xlabel("mean tokens per run"); ax.legend(loc="lower right")
plt.tight_layout(); plt.savefig(sys.argv[2], dpi=130)
