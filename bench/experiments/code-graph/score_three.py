"""Score Footnote, graphify and codebase-memory-mcp call links against one compiler graph, same classes as score_rules.py.

python -I score3.py <compiler.db> <footnote rules.tsv> <graphify.tsv> <cbm.tsv> <name>
(the last two from graphify_to_tsv.py and cbm_to_tsv.py)
"""
import json, re, sqlite3, sys

c = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
files = {p.replace("\\", "/") for (p,) in c.execute("SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'")}
comp, lines, named = set(), set(), set()
for a, l, n, b in c.execute("""SELECT rf.path, r.line, s.name, sf.path FROM refs r JOIN files rf ON rf.id = r.file_id
       JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id WHERE r.origin = 'compiler'"""):
    a, b = a.replace("\\", "/"), b.replace("\\", "/")
    comp.add((a, l, n, b)); lines.add((a, l)); named.add((a, l, n))


def tsv(path):
    for line in open(path, encoding="utf-8"):
        p = line.rstrip("\n").split("\t")
        if len(p) >= 4 and p[1].isdigit():
            yield p[0], int(p[1]), p[2], p[3]


def cbm(path):
    raw = open(path, encoding="utf-8-sig").read().strip()
    text = json.loads(raw)["content"][0]["text"]
    for row in text.splitlines()[1:]:
        m = re.match(r"\s+(\S+) \"?(\d+)\"? (\S+) (\S+) \S+$", row)
        if m:
            yield m[1], int(m[2]), m[3], m[4]


def score(edges):
    k = {"same": 0, "elsewhere": 0, "no_edge": 0, "skipped": 0}
    for e in set(edges):
        if e[0] in files and e[3] in files:
            k["same" if e in comp else "elsewhere" if e[:3] in named else "no_edge" if e[:2] in lines else "skipped"] += 1
    links = sum(k.values())
    judged = k["same"] + k["elsewhere"] + k["no_edge"]
    return f"links={links:6} right={k['same']:6} wrong={k['elsewhere']:5} no_edge={k['no_edge']:5} skipped={k['skipped']:5}  analyzed precision={k['same'] / max(1, judged):.3f}  conflict={k['elsewhere'] / max(1, k['same'] + k['elsewhere']):.3f}"


name = sys.argv[5]
print(f"== {name} (compiler files: {len(files)})")
print(f"  footnote  {score(tsv(sys.argv[2]))}")
print(f"  graphify  {score(tsv(sys.argv[3]))}")
print(f"  cbm       {score(tsv(sys.argv[4]))}")
