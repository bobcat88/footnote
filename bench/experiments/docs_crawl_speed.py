"""Crawl speed, candidate E of DOCS-OPTIMIZE-2026-11-16.md: sequential crawler against adaptive concurrency per host.

    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_crawl_speed.py <sequential.exe> <adaptive.exe>

The seven development libraries (Grounded Docs' recipe and docs-confirm's), each crawled by both binaries back to back, the
order alternating per library so a warm server cache does not favour one side. Reports wall time, pages, failures,
slow-downs, final concurrency and coverage of judged URLs. Output: bench/work/dev/crawl-speed/.
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

import yaml

ARGS, sys.argv = sys.argv, sys.argv[:1]
sys.path.insert(0, str(Path(__file__).parent))
import docs_vs_grounded as g  # noqa: E402

OUT = g.REPO / "bench/work/dev/crawl-speed"
CONFIRM = json.loads((g.REPO / "bench/datasets/docs-confirm/recipe.json").read_text(encoding="utf-8"))
LIBS = {**g.RECIPE, **{k: tuple(v) for k, v in CONFIRM.items()}}
DATASETS = [g.REPO / "bench/work/gd/dataset.yaml", g.REPO / "bench/datasets/docs-confirm/dataset.yaml"]


def judged(lib: str) -> set[str]:
    out = set()
    for d in DATASETS:
        for e in yaml.safe_load(d.read_text(encoding="utf-8"))["entries"]:
            if e["vars"]["library"] == lib:
                out |= {g.normalize_url(r["url"]) for r in e["vars"]["qrels"] if r["grade"] >= 2}
    return out


def crawl(exe: str, side: str, lib: str, url: str, pages: int, exclude: str | None) -> dict:
    folder = OUT / side / lib
    args = [exe, "crawl", url, "--out", str(folder), "--max-pages", str(pages)]
    if exclude:
        args += ["--exclude", exclude[1:-1]]
    if side == "adaptive":
        args += ["--concurrency", "2"]  # same start every time; the per-host memory is not exercised here
    t = time.perf_counter()
    r = subprocess.run(args, capture_output=True, text=True, encoding="utf-8", timeout=7200)
    secs = time.perf_counter() - t
    v = json.loads(r.stdout or "{}")
    urls = json.loads((folder / "urls.json").read_text(encoding="utf-8")) if (folder / "urls.json").exists() else {}
    rel = judged(lib)
    got = {g.normalize_url(u) for u in urls.values()}
    return {"seconds": secs, "exit": r.returncode, "pages": v.get("pages"), "skipped": v.get("skipped"),
            "slowed_down": v.get("slowed_down"), "concurrency_end": v.get("concurrency"),
            "coverage": len(rel & got) / len(rel) if rel else None, "error": v.get("error")}


def main() -> None:
    seq, ada = ARGS[1], ARGS[2]
    OUT.mkdir(parents=True, exist_ok=True)
    results = {}
    for i, (lib, (url, pages, exclude)) in enumerate(LIBS.items()):
        order = [("sequential", seq), ("adaptive", ada)]
        if i % 2:
            order.reverse()
        results[lib] = {side: crawl(exe, side, lib, url, pages, exclude) for side, exe in order}
        print(time.strftime("%H:%M:%S"), lib, json.dumps(results[lib]), flush=True)
    total = {side: sum(r[side]["seconds"] for r in results.values()) for side in ("sequential", "adaptive")}
    print("TOTAL seconds", json.dumps(total), "speed-up", round(total["sequential"] / total["adaptive"], 2), flush=True)
    (OUT / "results.json").write_text(json.dumps({"libraries": results, "total_seconds": total}, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
