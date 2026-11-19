"""Per-tier precision and recall (Footnote Code confidence tiers, 19 Nov 2026).

FOOTNOTE_CODE_GUESSES=1 FOOTNOTE_CODE_RULES=<name>.tsv footnote code-index <project>   (compiler indexer off PATH)
python -I score_tiers.py <work folder> <name>:<group> ...     (work folder holds <name>-comp/ with the compiler graph and <name>.tsv)

Tier "default" is every rule that is not a `guess_*` rule; each guess rule is scored alone, then cumulatively on top of the default.
Classes as in score_rules.py (same / elsewhere / no_edge / skipped). Recall = distinct correct links / the compiler's edges to
functions (calls and function references) in the files the compiler covered, the same denominator for every row.
"""
import collections, glob, sqlite3, sys

root = sys.argv[1]
rows = collections.defaultdict(lambda: collections.Counter())  # (group, tier) -> counts
edge = {}  # (group, tier) -> number of compiler edges (denominator), summed over projects
for arg in sys.argv[2:]:
    name, group = arg.split(":")
    db = glob.glob(f"{root}/{name}-comp/**/*.db", recursive=True)[0]
    c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    files = {p.replace("\\", "/") for (p,) in c.execute("SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'")}
    comp, lines, named, truth = set(), set(), set(), set()
    for a, l, n, b, k in c.execute("""SELECT rf.path, r.line, s.name, sf.path, s.kind FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id WHERE r.origin = 'compiler'"""):
        a, b = a.replace("\\", "/"), b.replace("\\", "/")
        comp.add((a, l, n, b)); lines.add((a, l)); named.add((a, l, n))
        if k == "function" and not n.startswith("<") and a in files and b in files:  # constructors and accessors are not calls
            truth.add((a, l, n, b))
    tiers = collections.defaultdict(set)  # tier -> set of distinct links
    classes = collections.defaultdict(collections.Counter)
    for line in open(f"{root}/{name}.tsv", encoding="utf-8"):
        src, ln, n, dst, rule = line.rstrip("\n").split("\t")
        if src not in files or dst not in files:
            continue
        link = (src, int(ln), n, dst)
        t = rule if rule.startswith("guess_") else "default"
        k = "same" if link in comp else "elsewhere" if link[:3] in named else "no_edge" if link[:2] in lines else "skipped"
        classes[t][k] += 1
        classes[t]["links"] += 1
        tiers[t].add(link)
    base = tiers["default"]
    guess_rules = sorted(t for t in tiers if t != "default")
    cumulative = set(base)
    for t in ["default"] + guess_rules + ["default+all guesses"]:
        if t == "default+all guesses":
            links = cumulative.union(*(tiers[g] for g in guess_rules))
            cl = sum((classes[g] for g in guess_rules), collections.Counter(classes["default"]))
        else:
            links, cl = tiers[t], classes[t]
        for key in ((group, t), (f"{name}", t)):
            r = rows[key]
            r.update(cl)
            r["right"] += len(links & truth)
            r["distinct"] += len(links)
            edge[key] = edge.get(key, 0) + len(truth)
print("| Scope | Tier | Links | Same | Elsewhere | No edge | Not analyzed | Analyzed precision | Conflict rate | Recall | Recall delta vs default |")
print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
for (scope, tier) in sorted(rows, key=lambda k: (k[0], k[1] != "default", k[1] == "default+all guesses", k[1])):
    k = rows[(scope, tier)]
    judged = k["same"] + k["elsewhere"] + k["no_edge"]
    rec = k["right"] / max(1, edge[(scope, tier)])
    d0 = rows[(scope, "default")]["right"] / max(1, edge[(scope, "default")])
    gain = rec if tier.startswith("guess_") else rec - d0  # a single guess tier is disjoint from the default: its own recall is its gain
    print(f"| {scope} | {tier} | {k['links']} | {k['same']} | {k['elsewhere']} | {k['no_edge']} | {k['skipped']} | "
          f"{k['same'] / max(1, judged):.3f} | {k['elsewhere'] / max(1, k['same'] + k['elsewhere']):.3f} | {rec:.3f} | {gain:+.3f} |")
