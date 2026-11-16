"""Recall switch confirmation (MEMORY-RECALL-SWITCH.md): keyword vs hybrid memory recall on a copy of the real memory database.

    bench/.labenv/Scripts/python.exe -I bench/experiments/memory_recall_switch.py <footnote.exe> quality
    bench/.labenv/Scripts/python.exe -I bench/experiments/memory_recall_switch.py <footnote.exe> latency

Inputs (git-ignored, they quote personal memories): bench/work/recall-switch/memory.db (a `footnote backup` copy) and
questions.json (frozen by freeze_questions.py before any run). Outputs: quality.json and latency.json in the same folder.
"""
from __future__ import annotations

import json
import os
import random
import statistics
import subprocess
import sys
import time
from pathlib import Path

EXE, STEP = sys.argv[1], sys.argv[2]
W = Path(__file__).resolve().parents[1] / "work/recall-switch"
DB = W / "memory.db"
MODELS = [str(Path(os.environ["LOCALAPPDATA"]) / "footnote/models" / m) for m in ("bekko-a8m", "gte-small")]
HYBRID = [a for m in MODELS for a in ("--model", m)]


def recall(query: str, project: str, extra: list[str]) -> list[int]:
    out = subprocess.run([EXE, "--db", str(DB), "memory-recall", query, "--project", project, "--limit", "3", *extra],
                         capture_output=True, text=True, encoding="utf-8", check=True).stdout
    return [m["id"] for m in json.loads(out)["memories"]]


def bootstrap(a: list[float], b: list[float], seed: int = 20261116, n: int = 10_000) -> tuple[float, float, float]:
    diffs = [x - y for x, y in zip(a, b)]
    rng = random.Random(seed)
    means = sorted(statistics.fmean(rng.choices(diffs, k=len(diffs))) for _ in range(n))
    return statistics.fmean(diffs), means[int(0.025 * n)], means[int(0.975 * n) - 1]


def quality() -> None:
    qs = json.loads((W / "questions.json").read_text(encoding="utf-8"))
    rows, any3, rr = [], {"keyword": [], "hybrid": []}, {"keyword": [], "hybrid": []}
    for q in qs:
        row = {"n": q["n"], "lang": q["lang"]}
        for mode, extra in (("keyword", []), ("hybrid", HYBRID)):
            ids = recall(q["query"], q["project"], extra)
            hit = [i in q["answers"] for i in ids]
            any3[mode].append(float(any(hit)))
            rr[mode].append(next((1 / (k + 1) for k, h in enumerate(hit) if h), 0.0))
            row[mode] = ids
        rows.append(row)
    d, lo, hi = bootstrap(any3["hybrid"], any3["keyword"])
    by_lang = {lang: {m: statistics.fmean(v for v, q in zip(any3[m], qs) if q["lang"] == lang) for m in any3}
               for lang in ("en", "fr")}
    result = {"questions": len(qs), "any_at_3": {m: statistics.fmean(v) for m, v in any3.items()},
              "mrr_at_3": {m: statistics.fmean(v) for m, v in rr.items()}, "by_language": by_lang,
              "hybrid_minus_keyword": d, "ci95": [lo, hi], "passes": lo > 0}
    (W / "quality.json").write_text(json.dumps({"result": result, "rows": rows}, indent=1), encoding="utf-8")
    print(json.dumps(result, indent=1))


def timed(cmd: list[str]) -> float:
    t = time.perf_counter()
    subprocess.run(cmd, capture_output=True, check=True)
    return (time.perf_counter() - t) * 1000


def latency() -> None:
    """Whole process per call, as the hook pays it: keyword in-process vs hybrid through a warm resident daemon."""
    qs = json.loads((W / "questions.json").read_text(encoding="utf-8"))
    daemon = subprocess.Popen([EXE, "--db", str(DB), "serve", "--mcp", "--daemon", "--idle-seconds", "600"],
                              stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        base = [EXE, "--db", str(DB), "memory-recall"]
        for _ in range(60):  # wait until the daemon answers, models loaded
            if subprocess.run([*base, "warm-up", "--resident", *HYBRID], capture_output=True).returncode == 0:
                break
            time.sleep(1)
        else:
            raise SystemExit("daemon never answered")
        times = {"keyword": [], "hybrid_resident": []}
        for _ in range(2):  # two passes, interleaved per question so load drift hits both modes alike
            for q in qs:
                args = [q["query"], "--project", q["project"], "--limit", "3"]
                times["keyword"].append(timed([*base, *args]))
                times["hybrid_resident"].append(timed([*base, *args, "--resident", *HYBRID]))
        p = lambda v, f: sorted(v)[int(f * len(v)) - 1]  # noqa: E731
        result = {m: {"calls": len(v), "p50_ms": round(statistics.median(v), 1), "p95_ms": round(p(v, 0.95), 1)}
                  for m, v in times.items()}
        result["added_p95_ms"] = round(result["hybrid_resident"]["p95_ms"] - result["keyword"]["p95_ms"], 1)
        result["passes"] = result["added_p95_ms"] < 50
        (W / "latency.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
        print(json.dumps(result, indent=1))
    finally:
        daemon.terminate()  # stopped by its own handle (PID), never by name
        daemon.wait(timeout=10)


{"quality": quality, "latency": latency}[STEP]()
