"""Classify the compiler's function edges that the default tier misses, by cause (Footnote Code, 19 Nov 2026).

python -I classify_misses.py <work folder> <sources folder> <name>:<group> ...
Uses <work>/<name>.tsv (default-tier rows only) and <work>/<name>-comp/ (compiler graph); reads the line of each missed edge from
<sources>/<name>/<file> and sorts it by what stands before `.name(`: a call chain, `self.attr.`, `self.`, a plain value, `super()`,
a decorator, a bare name, or other. Text patterns, not types: the buckets say where to look, not what the fix is.
"""
import collections, glob, re, sqlite3, sys

work, src = sys.argv[1], sys.argv[2]
out = collections.defaultdict(collections.Counter)
example = {}
for arg in sys.argv[3:]:
    name, group = arg.split(":")
    db = glob.glob(f"{work}/{name}-comp/**/*.db", recursive=True)[0]
    c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    files = {p.replace("\\", "/") for (p,) in c.execute("SELECT DISTINCT f.path FROM files f JOIN symbols s ON s.file_id = f.id WHERE s.scip NOT LIKE 'treesitter %'")}
    truth = set()
    for a, l, n, b in c.execute("""SELECT rf.path, r.line, s.name, sf.path FROM refs r JOIN files rf ON rf.id = r.file_id
           JOIN symbols s ON s.id = r.symbol_id JOIN files sf ON sf.id = s.file_id WHERE r.origin = 'compiler' AND s.kind = 'function'"""):
        a, b = a.replace("\\", "/"), b.replace("\\", "/")
        if a in files and b in files:
            truth.add((a, l, n, b))
    got, at = set(), set()
    for line in open(f"{work}/{name}.tsv", encoding="utf-8"):
        s, ln, n, d, rule = line.rstrip("\n").split("\t")
        if not rule.startswith("guess_"):
            got.add((s, int(ln), n, d))
        at.add((s, int(ln)))
    cache = {}
    for a, l, n, b in sorted(truth - got):
        lines = cache.setdefault(a, open(f"{src}/{name}/{a}", encoding="utf-8", errors="replace").read().split("\n"))
        text = lines[l - 1] if l - 1 < len(lines) else ""
        m = re.search(r"(?<![\w$])" + re.escape(n) + r"\b", text)
        before = text[: m.start()] if m else ""
        t = before.rstrip()
        if n.startswith("<"):
            k = "scip-typescript <constructor>/<get>/<set> reference: a `new X()` or property access, not a call"
        elif n in ("constructor", "__init__") and (a, l) in at:
            k = "constructor: linked to the class under its own name (scoring artifact, not a miss)"
        elif n in ("constructor", "__init__"):
            k = "constructor: `new X()` or `X()` not linked at all"
        elif text.lstrip().startswith("@"):
            k = "decorator"
        elif re.match(r"\s*(import|from|export)\b", text) or re.match(r"\s*(public |private )?(class|interface|def|function|async def|abstract)\b", text) and not before.endswith("."):
            k = "import, export or declaration line"
        elif not m:
            k = "name not on the line (multi-line call)"
        elif re.search(r"super\(\)\.$|super\.$", t):
            k = "super call"
        elif re.search(r"\b(self|this)\.\w+\.$", t):
            k = "self.attr.method(): field of unknown type"
        elif re.search(r"\b(self|this)\.$", t):
            k = "self.method() not linked"
        elif t.endswith(").") or t.endswith("]."):
            k = "call chain: f().method()"
        elif t.endswith("?.") and re.search(r"\w\?\.$", t):
            k = "optional chain a?.method()"
        elif re.search(r"\w\.$", t):
            k = "x.method(): value of unknown type"
        elif t.endswith("."):
            k = "other member access"
        elif re.search(r"(new|await)\s*$", t) or re.search(r"\w+\s*$", t) and not t.endswith("."):
            k = "construction or keyword before the name"
        else:
            k = "bare name: not linked (imported, re-exported, local, alias)"
        out[group][k] += 1
        out[group]["total missed"] += 1
        out[group]["compiler function edges"] += 0
        example.setdefault((group, k), (a, l, text.strip()[:90]))
    out[group]["compiler function edges"] += len(truth)
for g, cnt in out.items():
    tot = cnt["total missed"]
    print(f"\n{g}: {tot} missed of {cnt['compiler function edges']} compiler function edges")
    print("| Cause | Missed | Share | Example |")
    print("|---|---:|---:|---|")
    for k, v in cnt.most_common():
        if k in ("total missed", "compiler function edges"):
            continue
        e = example[(g, k)]
        print(f"| {k} | {v} | {v / tot:.1%} | `{e[0]}:{e[1]}` {e[2].replace('|', '/')} |")
