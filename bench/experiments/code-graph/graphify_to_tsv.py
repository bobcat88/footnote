"""graphify graph.json -> rules-style TSV (file, line, callee name, callee file, relation) for its call edges."""
import json, sys

g = json.load(open(sys.argv[1], encoding="utf-8"))
nodes = {n["id"]: n for n in g["nodes"]}
with open(sys.argv[2], "w", encoding="utf-8") as out:
    for e in g.get("links") or g.get("edges"):
        if e.get("relation") not in ("calls", "indirect_call"):
            continue
        t = nodes.get(e["target"])
        loc = (e.get("source_location") or "").lstrip("L").split("-")[0]
        if not t or not loc.isdigit() or not t.get("source_file"):
            continue
        name = t["label"].rstrip("()").rsplit(".", 1)[-1]  # `Store.save()` -> `save`
        out.write(f"{e['source_file'].replace(chr(92), '/')}\t{loc}\t{name}\t{t['source_file'].replace(chr(92), '/')}\t{e['relation']}\n")
