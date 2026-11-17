"""Score codebase-memory-mcp's CALLS edges against the compiler graph, the same way Footnote's tree-sitter layer was scored.

python -I score_cbm.py <compiler.db> <cbm_calls.json>
"""
import json
import sqlite3
import sys

# Optional last argument: a folder to leave out wherever it sits (e.g. tests/, where the compiler graph misses calls to test helpers).
SKIP = sys.argv[3] if len(sys.argv) > 3 else None
from collections import Counter

c = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
comp_files = {p.replace("\\", "/") for (p,) in c.execute("SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'")}
comp_files = {f for f in comp_files if not (SKIP and SKIP in "/" + f)}
comp = {
    (a.replace("\\", "/"), l, n, b.replace("\\", "/"))
    for a, l, n, b in c.execute(
        """SELECT rf.path, r.line, s.name, sf.path FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id WHERE s.kind IN ('function','type') AND r.origin = 'compiler'"""
    )
}
outer = json.load(open(sys.argv[2], encoding="utf-8"))
inner = json.loads(outer["content"][0]["text"])
rows = inner["rows"]
cbm, strategies = set(), Counter()
for src, line, name, dst, strategy in rows:
    src, dst = (src or "").replace("\\", "/"), (dst or "").replace("\\", "/")
    if src in comp_files and dst in comp_files:
        cbm.add((src, int(line), name, dst))
        strategies[strategy] += 1
comp = {e for e in comp if e[0] in comp_files and e[3] in comp_files}
hit = cbm & comp
print(f"rows exported: {len(rows)}  on the {len(comp_files)} compiler files: {len(cbm)}")
print(f"compiler edges: {len(comp)}  codebase-memory edges: {len(cbm)}  in both: {len(hit)}")
print(f"precision: {len(hit) / max(1, len(cbm)):.3f}  recall: {len(hit) / max(1, len(comp)):.3f}")
print("strategies:", dict(strategies.most_common(8)))
by = Counter()
for e in cbm:
    pass
print("sample not in compiler graph:")
for e in sorted(cbm - comp)[:10]:
    print("  ", e)
