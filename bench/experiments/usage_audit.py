"""Audit of footnote usage's log parsing (#238), last N days, counts only.

python -I usage_audit.py [days]
"""
import collections, json, os, sys, time
from pathlib import Path

DAYS = int(sys.argv[1]) if len(sys.argv) > 1 else 7
cut = time.time() - DAYS * 86400
home = Path(os.environ["USERPROFILE"])


def recent(root):
    return [p for p in root.rglob("*.jsonl") if p.stat().st_mtime >= cut] if root.exists() else []


# --- Codex: per-response records against the session's own running total.
print("== Codex")
for sub in ("sessions", "archived_sessions"):
    files = recent(home / ".codex" / sub)
    tot = collections.Counter()
    worst = []
    for f in files:
        recs, last_total = collections.Counter(), None
        for line in open(f, encoding="utf-8", errors="replace"):
            if '"token_usage_record"' in line:
                u = json.loads(line)["payload"]["usage"]
                recs["input"] += u.get("input_tokens", 0); recs["cached"] += u.get("cached_input_tokens", 0)
                recs["output"] += u.get("output_tokens", 0); recs["n"] += 1
            elif '"total_token_usage"' in line:
                info = (json.loads(line).get("payload") or {}).get("info") or {}
                last_total = info.get("total_token_usage") or last_total
        tot["files"] += 1; tot["records"] += recs["n"]
        if last_total:
            tot["with_total"] += 1
            for k, k2 in (("input", "input_tokens"), ("cached", "cached_input_tokens"), ("output", "output_tokens")):
                tot[f"rec_{k}"] += recs[k]; tot[f"total_{k}"] += last_total.get(k2, 0)
            d = recs["input"] - last_total.get("input_tokens", 0)
            if d:
                worst.append((abs(d), f.name, recs["input"], last_total.get("input_tokens", 0)))
        elif recs["n"] == 0:
            tot["no_records_no_total"] += 1
    print(f" {sub}: {dict(tot)}")
    for w in sorted(worst, reverse=True)[:3]:
        print("   input differs:", w)

# --- Claude Code: repeated lines of one request, and one request in several files.
print("== Claude Code")
files = recent(home / ".claude" / "projects")
lines_per = collections.defaultdict(list)  # key -> [(file, usage)]
for f in files:
    for line in open(f, encoding="utf-8", errors="replace"):
        if '"usage"' not in line or '"assistant"' not in line:
            continue
        try:
            v = json.loads(line)
        except ValueError:
            continue
        m = v.get("message") or {}
        if v.get("type") != "assistant" or not m.get("usage") or str(m.get("model", "")).startswith("<"):
            continue
        u = m["usage"]
        lines_per[f"{m.get('id')}|{v.get('requestId', '')}"].append(
            (str(f), u.get("output_tokens", 0), u.get("cache_read_input_tokens", 0), u.get("cache_creation_input_tokens", 0), u.get("input_tokens", 0)))
c = collections.Counter()
out_first = out_max = 0
ex = []
for k, rows in lines_per.items():
    c["requests"] += 1
    in_files = {r[0] for r in rows}
    if len(in_files) > 1:
        c["requests_in_several_files"] += 1
        c["extra_copies"] += len(in_files) - 1
    first = rows[0]
    out_first += first[1]; out_max += max(r[1] for r in rows)
    if len({r[1:] for r in rows}) > 1:
        c["requests_whose_lines_differ"] += 1
        if len(ex) < 3:
            ex.append([r[1:] for r in rows][:4])
print(" ", dict(c))
print(f"  output if the first line is kept: {out_first:,}  if the largest is kept: {out_max:,}")
for e in ex:
    print("   lines differ (output, cache_read, cache_write, input):", e)
subagent = [f for f in files if "subagents" in f.parts]
print(f"  files: {len(files)}, of which under a subagents folder: {len(subagent)}")
