"""Precision and recall of a tree-sitter code graph against a compiler graph (Footnote Code, 17 Nov 2026).

python -I compare_graphs.py <compiler.db> <treesitter.db> [folder to leave out, e.g. tests/]

An edge is (file of the reference, its line, target name, target's file). The truth is the compiler-graded links only (a
graph built with rust-analyzer installed also holds parser links for other files), on the files the compiler covered. Both
sides are limited to functions and types.
"""
import sqlite3
import sys

# Optional last argument: a folder to leave out wherever it sits (e.g. tests/, where the compiler graph misses calls to test helpers).
SKIP = sys.argv[3] if len(sys.argv) > 3 else None


def edges(db, origin):
    c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    rows = c.execute(
        """SELECT rf.path, r.line, s.name, sf.path FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id
           WHERE s.kind IN ('function', 'type') AND r.origin = ?""",
        (origin,),
    ).fetchall()
    return {(a.replace("\\", "/"), l, n, b.replace("\\", "/")) for a, l, n, b in rows}


c = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
covered = {
    p.replace("\\", "/")
    for (p,) in c.execute(
        "SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'"
    )
}
both = {f for f in covered if not (SKIP and SKIP in "/" + f)}
comp = {e for e in edges(sys.argv[1], "compiler") if e[0] in both and e[3] in both}
ts = {e for e in edges(sys.argv[2], "resolved") if e[0] in both and e[3] in both}
hit = comp & ts
print(f"files compared: {len(both)}")
print(f"compiler edges: {len(comp)}  tree-sitter edges: {len(ts)}  in both: {len(hit)}")
print(f"precision: {len(hit) / max(1, len(ts)):.3f}  recall: {len(hit) / max(1, len(comp)):.3f}  (raw)")
# The compiler graph has holes (#215): code behind features it did not enable, calls it files under a trait, test modules.
# Each parser edge it lacks is classed by what the compiler says at that line, and precision is given on analyzed lines.
lines, named = set(), set()
for a, l, n in c.execute(
    "SELECT rf.path, r.line, s.name FROM refs r JOIN files rf ON rf.id = r.file_id JOIN symbols s ON s.id = r.symbol_id WHERE r.origin = 'compiler'"
):
    lines.add((a.replace("\\", "/"), l))
    named.add((a.replace("\\", "/"), l, n))
miss = ts - comp
elsewhere = [e for e in miss if e[:3] in named]
no_edge = [e for e in miss if e[:3] not in named and e[:2] in lines]
skipped = len(miss) - len(elsewhere) - len(no_edge)
print(f"compiler links elsewhere: {len(elsewhere)}  analyzed line, no edge: {len(no_edge)}  line not analyzed: {skipped}")
print(f"precision on analyzed lines: {len(hit) / max(1, len(hit) + len(elsewhere) + len(no_edge)):.3f}")
for e in sorted(elsewhere)[:12]:
    print("  compiler links elsewhere:", e)
