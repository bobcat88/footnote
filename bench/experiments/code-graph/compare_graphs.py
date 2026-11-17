"""Precision and recall of a tree-sitter code graph against a compiler graph (Footnote Code, 17 Nov 2026).

python -I compare_graphs.py <compiler.db> <treesitter.db>

An edge is (file of the reference, its line, target name, target's file). Only files present in both graphs count, and only
references to items the compiler graph also defines (both sides limited to functions and types).
"""
import sqlite3
import sys

# Optional last argument: a path prefix to leave out (e.g. tests/, where the compiler graph misses calls to test helpers).
SKIP = sys.argv[3] if len(sys.argv) > 3 else None


def edges(db, files=None):
    c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    rows = c.execute(
        """SELECT rf.path, r.line, s.name, sf.path, s.kind FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id
           WHERE s.kind IN ('function', 'type')"""
    ).fetchall()
    paths = {p for (p,) in c.execute("SELECT path FROM files")}
    return {(a.replace("\\", "/"), l, n, b.replace("\\", "/")) for a, l, n, b, _ in rows}, {p.replace("\\", "/") for p in paths}


comp, comp_files = edges(sys.argv[1])
ts, ts_files = edges(sys.argv[2])
both = {f for f in comp_files & ts_files if not (SKIP and f.startswith(SKIP))}
comp = {e for e in comp if e[0] in both and e[3] in both}
ts = {e for e in ts if e[0] in both and e[3] in both}
hit = comp & ts
print(f"files in both: {len(both)}")
print(f"compiler edges: {len(comp)}  tree-sitter edges: {len(ts)}  in both: {len(hit)}")
print(f"precision: {len(hit) / max(1, len(ts)):.3f}  recall: {len(hit) / max(1, len(comp)):.3f}")
wrong = sorted(ts - comp)[:12]
print("sample of tree-sitter edges the compiler does not have:")
for e in wrong:
    print("  ", e)
