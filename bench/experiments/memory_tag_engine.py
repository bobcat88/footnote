"""Tag engine check (MEMORY-TAGS.md): recall with tags inferred from the prompt against recall without, on tagged copies.

    bench/.labenv/Scripts/python.exe -I bench/experiments/memory_tag_engine.py <footnote.exe>

Inputs (git-ignored): bench/work/recall-switch/{memory.db, questions.json, tags.json}. Builds tagged.db (every tag) and
tagged30.db (30 % of the memory-tag pairs removed, seed 20261121), then for each of the 100 questions runs hybrid memory-recall
with and without --infer-tags. Writes tag-engine.json.
"""
from __future__ import annotations

import json
import os
import random
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

EXE = sys.argv[1]
W = Path(__file__).resolve().parents[1] / "work/recall-switch"
MODELS = [str(Path(os.environ["LOCALAPPDATA"]) / "footnote/models" / m) for m in ("bekko-a8m", "gte-small")]
HYBRID = [a for m in MODELS for a in ("--model", m)]


def fn(db: Path, *args: str) -> dict:
    out = subprocess.run([EXE, "--db", str(db), *args], capture_output=True, text=True, encoding="utf-8", check=True).stdout
    return json.loads(out)


def build(name: str, tags: dict[str, list[str]]) -> Path:
    db = W / name
    for p in (db, Path(f"{db}.tantivy")):
        if p.is_dir():
            shutil.rmtree(p)
        elif p.exists():
            p.unlink()
    shutil.copyfile(W / "memory.db", db)
    for mid, ts in tags.items():
        if ts:
            fn(db, "memory-tag", mid, *[a for t in ts for a in ("--add", t)])
    return db


def run(db: Path, tags: dict[str, list[str]], qs: list[dict]) -> dict:
    rows = []
    for q in qs:
        base = fn(db, "memory-recall", q["query"], "--project", q["project"], "--limit", "3", *HYBRID)
        filt = fn(db, "memory-recall", q["query"], "--project", q["project"], "--limit", "3", *HYBRID, "--infer-tags")
        used = set(filt["tags_used"])
        alive = not used or any(not tags.get(str(a)) or used & set(tags.get(str(a), [])) for a in q["answers"])
        rows.append({"n": q["n"], "lang": q["lang"], "tags_used": sorted(used), "pool": base["candidates"],
                     "kept": filt["candidates"], "survives": alive,
                     "any3_base": any(m["id"] in q["answers"] for m in base["memories"]),
                     "any3_tags": any(m["id"] in q["answers"] for m in filt["memories"])})
    named = [r for r in rows if r["tags_used"]]
    return {
        "questions": len(rows), "prompts_naming_a_tag": len(named),
        "named_by_language": {lang: sum(r["lang"] == lang for r in named) for lang in ("en", "fr")},
        "survival": statistics.fmean(r["survives"] for r in rows),
        "survival_when_named": statistics.fmean(r["survives"] for r in named) if named else None,
        "any3_without": statistics.fmean(r["any3_base"] for r in rows),
        "any3_with": statistics.fmean(r["any3_tags"] for r in rows),
        "candidates_kept_when_named": statistics.fmean(r["kept"] / r["pool"] for r in named) if named else None,
        "lost": [r for r in rows if r["any3_base"] and not r["any3_tags"]],
        "gained": [r["n"] for r in rows if r["any3_tags"] and not r["any3_base"]],
        "rows": rows,
    }


def latency(db: Path, qs: list[dict]) -> dict:
    """Keyword recall in-process, as the hook falls back to it: whole process with and without the engine, interleaved."""
    t = {"without": [], "with": []}
    for q in qs * 2:
        for k, extra in (("without", []), ("with", ["--infer-tags"])):
            s = time.perf_counter()
            subprocess.run([EXE, "--db", str(db), "memory-recall", q["query"], "--project", q["project"], *extra],
                           capture_output=True, check=True)
            t[k].append((time.perf_counter() - s) * 1000)
    p95 = {k: sorted(v)[int(0.95 * len(v)) - 1] for k, v in t.items()}
    return {"p50_ms": {k: round(statistics.median(v), 1) for k, v in t.items()},
            "p95_ms": {k: round(v, 1) for k, v in p95.items()}, "added_p95_ms": round(p95["with"] - p95["without"], 1)}


def main() -> None:
    qs = json.loads((W / "questions.json").read_text(encoding="utf-8"))
    tags = json.loads((W / "tags.json").read_text(encoding="utf-8"))
    pairs = sorted((m, t) for m, ts in tags.items() for t in ts)
    rng = random.Random(20261121)
    drop = set(rng.sample(pairs, round(0.3 * len(pairs))))
    tags30 = {m: [t for t in ts if (m, t) not in drop] for m, ts in tags.items()}
    full_db, part_db = build("tagged.db", tags), build("tagged30.db", tags30)
    result = {"tags": {"memories_tagged": len(tags), "pairs": len(pairs), "distinct": len({t for _, t in pairs})},
              "complete": run(full_db, tags, qs), "incomplete_30pc": run(part_db, tags30, qs), "latency": latency(full_db, qs)}
    (W / "tag-engine.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
    for k in ("complete", "incomplete_30pc"):
        print(k, json.dumps({x: v for x, v in result[k].items() if x != "rows"}, indent=1))
    print("latency", result["latency"])


if __name__ == "__main__":
    main()
