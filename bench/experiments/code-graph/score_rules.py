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
total, right = Counter(), Counter()
for line in open(sys.argv[2], encoding="utf-8"):
    src, ln, name, dst, rule = line.rstrip("\n").split("\t")
    if src in files and dst in files:
        total[rule] += 1
        right[rule] += (src, int(ln), name, dst) in comp
for rule, n in total.most_common():
    print(f"{rule:16} {n:5} links  precision {right[rule] / n:.3f}")
print(f"{'all':16} {sum(total.values()):5} links  precision {sum(right.values()) / max(1, sum(total.values())):.3f}")
