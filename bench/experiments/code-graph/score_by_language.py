"""Per language and rule totals over projects scored by score_project.ps1 (#216), same classes as score_rules.py.

python -I score_by_language.py <work folder> <name>:<group> ...
"""
import collections, glob, sqlite3, sys

LANG = {"py": "Python", "ts": "TypeScript", "tsx": "TypeScript", "js": "JavaScript", "jsx": "JavaScript", "mjs": "JavaScript", "cjs": "JavaScript", "rs": "Rust"}
root = sys.argv[1]
tot = collections.defaultdict(collections.Counter)
for arg in sys.argv[2:]:
    name, group = arg.split(":")
    db = glob.glob(f"{root}/{name}-comp/**/*.db", recursive=True)[0]
    c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    files = {p.replace("\\", "/") for (p,) in c.execute("SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'")}
    comp, lines, named = set(), set(), set()
    for a, l, n, b in c.execute("""SELECT rf.path, r.line, s.name, sf.path FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id WHERE r.origin = 'compiler'"""):
        a, b = a.replace("\\", "/"), b.replace("\\", "/")
        comp.add((a, l, n, b)); lines.add((a, l)); named.add((a, l, n))
    for line in open(f"{root}/{name}.tsv", encoding="utf-8"):
        src, ln, n, dst, rule = line.rstrip("\n").split("\t")
        if src not in files or dst not in files:
            continue
        ln = int(ln)
        k = "same" if (src, ln, n, dst) in comp else "elsewhere" if (src, ln, n) in named else "no_edge" if (src, ln) in lines else "skipped"
        lang = LANG.get(src.rsplit(".", 1)[-1], "other")
        for r in (rule, "all"):
            tot[(group, lang, r)][k] += 1
            tot[(group, lang, r)]["links"] += 1
print("| Group | Language | Rule | Links | Same | Compiler links elsewhere | Analyzed, no edge | Not analyzed | Analyzed precision | Conflict rate |")
print("|---|---|---|---:|---:|---:|---:|---:|---:|---:|")
order = {"same_file": 0, "receiver_type": 1, "import": 2, "type_path": 3, "module_path": 4, "all": 9}
for (g, lang, r), k in sorted(tot.items(), key=lambda x: (x[0][0], x[0][1], order.get(x[0][2], 5))):
    if lang == "Rust":
        continue  # Rust is measured in CODE-RUST-PRECISION-2026-11-18.md
    judged = k["same"] + k["elsewhere"] + k["no_edge"]
    print(f"| {g} | {lang} | {r} | {k['links']} | {k['same']} | {k['elsewhere']} | {k['no_edge']} | {k['skipped']} | "
          f"{k['same'] / max(1, judged):.3f} | {k['elsewhere'] / max(1, k['same'] + k['elsewhere']):.3f} |")
