"""Precision of each tree-sitter linking rule against the compiler graph (Footnote Code, 17 Nov 2026).

FOOTNOTE_CODE_RULES=rules.tsv footnote code-index <root>   (with the precise indexer off PATH, so tree-sitter runs)
python -I score_rules.py <compiler.db> rules.tsv
"""
import sqlite3
import sys

# Optional last argument: a folder to leave out wherever it sits (e.g. tests/, where the compiler graph misses calls to test helpers).
SKIP = sys.argv[3] if len(sys.argv) > 3 else None
from collections import Counter

c = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
files = {p.replace("\\", "/") for (p,) in c.execute("SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'")}
files = {f for f in files if not (SKIP and SKIP in "/" + f)}
comp = {
    (a.replace("\\", "/"), l, n, b.replace("\\", "/"))
    for a, l, n, b in c.execute(
        """SELECT rf.path, r.line, s.name, sf.path FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id WHERE s.kind IN ('function','type') AND r.origin = 'compiler'"""
    )
}
# Each link is classed by what the compiler says at its line (#215): same, links elsewhere, line analyzed but no edge, line not
# analyzed (code behind a feature the indexer did not enable). Precision on analyzed lines leaves only the last class out.
lines, named = set(), set()
for a, l, n in c.execute(
    "SELECT rf.path, r.line, s.name FROM refs r JOIN files rf ON rf.id = r.file_id JOIN symbols s ON s.id = r.symbol_id WHERE r.origin = 'compiler'"
):
    lines.add((a.replace("\\", "/"), l))
    named.add((a.replace("\\", "/"), l, n))
total, cls = Counter(), {}
for line in open(sys.argv[2], encoding="utf-8"):
    src, ln, name, dst, rule = line.rstrip("\n").split("\t")
    if src in files and dst in files:
        ln = int(ln)
        k = "same" if (src, ln, name, dst) in comp else "elsewhere" if (src, ln, name) in named else "no_edge" if (src, ln) in lines else "skipped"
        total[rule] += 1
        cls.setdefault(rule, Counter())[k] += 1
cls["all"] = sum(cls.values(), Counter())
print(f"{'rule':14} {'links':>5} {'same':>5} {'elsewhere':>9} {'no_edge':>7} {'skipped':>7}  raw    analyzed")
for rule, n in total.most_common() + [("all", sum(total.values()))]:
    k = cls[rule]
    judged = k["same"] + k["elsewhere"] + k["no_edge"]
    print(f"{rule:14} {n:5} {k['same']:5} {k['elsewhere']:9} {k['no_edge']:7} {k['skipped']:7}  {k['same'] / max(1, n):.3f}  {k['same'] / max(1, judged):.3f}")
