"""codebase-memory-mcp project database -> rules-style TSV of its CALLS edges (file, line, callee name, callee file, strategy)."""
import json, sqlite3, sys

c = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
rows = c.execute("""SELECT a.file_path, e.properties, b.name, b.file_path FROM edges e JOIN nodes a ON a.id = e.source_id
                    JOIN nodes b ON b.id = e.target_id WHERE e.type = 'CALLS'""")
with open(sys.argv[2], "w", encoding="utf-8") as out:
    for src, props, name, dst in rows:
        p = json.loads(props or "{}")
        if src and dst and p.get("line"):
            out.write(f"{src.replace(chr(92), '/')}\t{p['line']}\t{name}\t{dst.replace(chr(92), '/')}\t{p.get('strategy', '')}\n")
