"""Where a Claude Code session's money goes (#238): per session turns, context growth, first-call size (fixed overhead),
re-writes after an idle gap, context added per tool, and Footnote's own hook injections. Counts and sizes only, no text kept.

python -I bench/experiments/session_costs.py [days]   (Opus 5.5 list prices; other models priced the same for shape only)
"""
import collections, json, os, sys, time
from datetime import datetime
from pathlib import Path

DAYS = int(sys.argv[1]) if len(sys.argv) > 1 else 7
READ, W5, W1H, OUT, IN = 0.20, 5.0, 8.0, 20.0, 4.0  # Opus 5.5, $ per million
cut = time.time() - DAYS * 86400
root = Path(os.environ.get("CLAUDE_CONFIG_DIR", Path.home() / ".claude")) / "projects"


def ts(v):
    try:
        return datetime.fromisoformat(v["timestamp"].replace("Z", "+00:00")).timestamp()
    except (KeyError, ValueError, AttributeError):
        return None


sessions = []
tool_bytes = collections.Counter()
inject_bytes = 0
for f in root.rglob("*.jsonl"):
    if f.stat().st_mtime < cut or "subagents" in f.parts:
        continue
    seen, calls, last_t = set(), [], None
    tool_of = {}
    for line in open(f, encoding="utf-8", errors="replace"):
        try:
            v = json.loads(line)
        except ValueError:
            continue
        m = v.get("message") or {}
        # Tool results: size of what each tool added to the conversation.
        if v.get("type") == "user" and isinstance(m.get("content"), list):
            for c in m["content"]:
                if c.get("type") == "tool_result":
                    size = len(json.dumps(c.get("content", ""), ensure_ascii=False))
                    tool_bytes[tool_of.get(c.get("tool_use_id"), "?")] += size
        if v.get("type") == "assistant" and isinstance(m.get("content"), list):
            for c in m["content"]:
                if c.get("type") == "tool_use":
                    tool_of[c.get("id")] = c.get("name", "?").split("__")[-1] if c.get("name", "").startswith("mcp__") else c.get("name", "?")
        if "Footnote agent memory" in line and v.get("type") in ("attachment", "system", "user"):
            inject_bytes += sum(len(p) for p in line.split("Footnote agent memory")[1:2])
        u = m.get("usage")
        if v.get("type") != "assistant" or not u or str(m.get("model", "")).startswith("<"):
            continue
        key = (m.get("id"), v.get("requestId"))
        if key in seen:
            continue
        seen.add(key)
        t = ts(v)
        prompt = u.get("input_tokens", 0) + u.get("cache_read_input_tokens", 0) + u.get("cache_creation_input_tokens", 0)
        w1 = (u.get("cache_creation") or {}).get("ephemeral_1h_input_tokens", 0)
        w5 = u.get("cache_creation_input_tokens", 0) - w1
        cost = (u.get("cache_read_input_tokens", 0) * READ + w5 * W5 + w1 * W1H + u.get("output_tokens", 0) * OUT + u.get("input_tokens", 0) * IN) / 1e6
        gap = (t - last_t) if (t and last_t) else 0
        calls.append({"prompt": prompt, "write": u.get("cache_creation_input_tokens", 0), "cost": cost, "gap": gap})
        last_t = t or last_t
    if calls:
        sessions.append((f, calls))

print(f"Claude Code sessions in the last {DAYS} days (subagents apart): {len(sessions)}")
tot = sum(c["cost"] for _, cs in sessions for c in cs)
# Cost by context size at the call: how much is spent on calls that re-read a long context.
bands = collections.Counter()
for _, cs in sessions:
    for c in cs:
        b = "<50k" if c["prompt"] < 50e3 else "50-100k" if c["prompt"] < 100e3 else "100-200k" if c["prompt"] < 200e3 else "200-400k" if c["prompt"] < 400e3 else ">400k"
        bands[b] += c["cost"]
print(f"total ${tot:,.0f}; cost by the context size of the call:")
for b in ["<50k", "50-100k", "100-200k", "200-400k", ">400k"]:
    print(f"  {b:9} ${bands[b]:8,.0f}  {bands[b] / max(tot, 1e-9):5.1%}")
firsts = sorted(cs[0]["prompt"] for _, cs in sessions)
print(f"first call of a session (fixed overhead plus the first message): median {firsts[len(firsts)//2]/1e3:.0f}k tokens")
idle = [c for _, cs in sessions for c in cs if c["gap"] > 3600 and c["write"] > 20e3]
print(f"re-writes after more than 1 h idle: {len(idle)} calls, ${sum(c['cost'] for c in idle):,.0f} ({sum(c['cost'] for c in idle)/max(tot,1e-9):.1%})")
turns = sorted(len(cs) for _, cs in sessions)
print(f"calls per session: median {turns[len(turns)//2]}, largest {turns[-1]}")
big = sorted(((sum(c['cost'] for c in cs), len(cs), max(c['prompt'] for c in cs)) for _, cs in sessions), reverse=True)[:5]
print("costliest sessions (cost, calls, peak context):", [(f"${c:,.0f}", n, f"{p/1e3:.0f}k") for c, n, p in big])
print("context added by tool results (about tokens, bytes/4):")
for name, b in tool_bytes.most_common(10):
    print(f"  {name:22} {b/4/1e6:6.2f}M")
print(f"Footnote memory injections: about {inject_bytes/4/1e3:,.0f}k tokens added")
