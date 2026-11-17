"""Score CodeGraph's call edges against the compiler graph, the same way as Footnote's base layer (17 Nov 2026).

DO_NOT_TRACK=1 CODEGRAPH_TELEMETRY=0 codegraph init <copy of the repository>
python -I score_codegraph.py <compiler.db> <copy>/.codegraph/codegraph.db [path prefix to leave out]
"""
import sqlite3
import sys

SKIP = sys.argv[3] if len(sys.argv) > 3 else None
c = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
files = {p.replace("\\", "/") for (p,) in c.execute("SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'")}
files = {f for f in files if not (SKIP and f.startswith(SKIP))}
comp = {
    (a.replace("\\", "/"), l, n, b.replace("\\", "/"))
    for a, l, n, b in c.execute(
        """SELECT rf.path, r.line, s.name, sf.path FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id WHERE s.kind IN ('function','type') AND r.origin = 'compiler'"""
    )
}
comp = {e for e in comp if e[0] in files and e[3] in files}
g = sqlite3.connect(f"file:{sys.argv[2]}?mode=ro", uri=True)
cg = {
    (a.replace("\\", "/"), l, n, b.replace("\\", "/"))
    for a, l, n, b in g.execute(
        """SELECT s.file_path, e.line, t.name, t.file_path FROM edges e JOIN nodes s ON s.id = e.source
           JOIN nodes t ON t.id = e.target WHERE e.kind = 'calls' AND e.line IS NOT NULL"""
    )
}
cg = {e for e in cg if e[0] in files and e[3] in files}
hit = cg & comp
print(f"compiler edges: {len(comp)}  codegraph edges: {len(cg)}  in both: {len(hit)}")
print(f"precision: {len(hit) / max(1, len(cg)):.3f}  recall: {len(hit) / max(1, len(comp)):.3f}")
for e in sorted(cg - comp)[:8]:
    print("  not in compiler graph:", e)
