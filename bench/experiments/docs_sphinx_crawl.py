"""Crawl-coverage variants on the Sphinx development libraries (protocol: DOCS-SPHINX-CRAWL-2026-11-19.md, written before the runs).

    python -I bench/experiments/docs_sphinx_crawl.py <run-dir> <footnote.exe> <variant> <cap> [exp] [recipe.json] [dataset.yaml]

One variant = one crawl (page cap `cap`, FOOTNOTE_CRAWL_EXP=`exp`), judged-URL coverage, index + embeddings, hybrid search
(--limit 5 --per-source 1), nDCG@5 and Recall@5 with Grounded Docs' metric code. Footnote only: Grounded Docs is never rerun here.
Development questions only; the sealed Flask/Python set (sha256 2e06dc86...) is spent and is not read.
"""
from __future__ import annotations

import json
import os
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

ARGS, sys.argv = sys.argv, sys.argv[:1]
RUN = Path(ARGS[1]).resolve()
EXE, VARIANT, CAP = str(Path(ARGS[2]).resolve()), ARGS[3], int(ARGS[4])
EXP = ARGS[5] if len(ARGS) > 5 else ""
REPO = Path(__file__).resolve().parents[2]
DEV = REPO / "bench/datasets/docs-sphinx-dev"
os.environ["DOCS_RUN"] = str(RUN)
os.environ["DOCS_DATASET"] = ARGS[7] if len(ARGS) > 7 else str(DEV / "dataset-core.yaml")
os.environ["DOCS_RECIPE"] = ARGS[6] if len(ARGS) > 6 else str(DEV / "recipe-core.json")
sys.path.insert(0, str(Path(__file__).parent))
import docs_vs_grounded as g  # noqa: E402
import yaml  # noqa: E402

g.FOOTNOTE = EXE
OUT = RUN / VARIANT
SEARCH = ["--limit", "5", "--per-source", "1", "--mode", "hybrid", *[a for m in g.MODELS for a in ("--model", m)]]


def say(msg: str) -> None:
    line = f"{time.strftime('%H:%M:%S')} [{VARIANT}] {msg}"
    print(line, flush=True)
    with open(OUT / "run.log", "a", encoding="utf-8") as f:
        f.write(line + "\n")


def crawl(lib: str, url: str, exclude) -> dict:
    out = OUT / "crawl" / lib
    args = [EXE, "crawl", url, "--out", str(out), "--max-pages", str(CAP)]
    if exclude:
        args += ["--exclude", exclude[1:-1]]
    t = time.time()
    env = {**os.environ, "FOOTNOTE_CRAWL_EXP": EXP}
    r = subprocess.run(args, capture_output=True, text=True, encoding="utf-8", timeout=7200, env=env)
    secs = time.time() - t
    say(f"crawl {lib}: exit {r.returncode}, {secs:.0f} s, {r.stdout.strip()[-300:]} {r.stderr.strip()[-200:]}")
    if r.returncode != 0 or not (out / "urls.json").is_file():
        raise RuntimeError(f"crawl failed for {lib}")
    return {"seconds": secs, **json.loads(r.stdout)}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    qs = [e["vars"] for e in yaml.safe_load(g.DATASET.read_text(encoding="utf-8"))["entries"]]
    res: dict = {"variant": VARIANT, "exp": EXP, "cap": CAP, "binary": EXE, "libs": {}}
    manifest: dict[str, str] = {}
    for lib, (url, _, exclude) in g.RECIPE.items():
        info = crawl(lib, url, exclude)
        folder = OUT / "crawl" / lib
        urls = {g.key(str(folder / f)): u for f, u in json.loads((folder / "urls.json").read_text(encoding="utf-8")).items()}
        manifest.update(urls)
        rel = {g.normalize_url(r["url"]) for q in qs if q["library"] == lib for r in q["qrels"] if r["grade"] >= 2}
        mine = {g.normalize_url(u) for u in urls.values()}
        t = time.time()
        db = OUT / "db" / f"{lib}.db"
        db.parent.mkdir(parents=True, exist_ok=True)
        g.fn(db, "index", "--root", str(folder), "--allow", str(folder))
        g.fn(db, "rebuild-embeddings", *[a for m in g.MODELS for a in ("--model", m)])
        build = time.time() - t
        with sqlite3.connect(f"file:{db.as_posix()}?mode=ro", uri=True) as c:
            chunks = c.execute("SELECT count(*) FROM chunks").fetchone()[0]
        res["libs"][lib] = {"relevant": len(rel), "coverage": len(rel & mine) / len(rel), "pages": len(mine),
                            "crawl_seconds": round(info["seconds"], 1), "build_seconds": round(build, 1),
                            "db_mb": round(db.stat().st_size / 1e6, 1), "chunks": chunks,
                            "discovered": info.get("discovered"), "queued_left": info.get("queued_left"),
                            "missed": sorted(rel - mine)}
        say(f"{lib}: coverage {res['libs'][lib]['coverage']:.3f} ({len(rel & mine)}/{len(rel)}), {len(mine)} pages, "
            f"crawl {info['seconds']:.0f} s, index {build:.0f} s, db {res['libs'][lib]['db_mb']} MB")
    ranked = {}
    for i, q in enumerate(qs):
        hits = g.fn(OUT / "db" / f"{q['library']}.db", "search", q["query"], *SEARCH)["results"]
        ranked[str(i)] = [manifest.get(g.key(h["path"]), h["path"]) for h in hits]
    ms = [g.metrics(ranked[str(i)], q["qrels"]) for i, q in enumerate(qs)]
    mean = lambda xs: sum(xs) / len(xs) if xs else float("nan")
    for scope in ["all", *g.RECIPE]:
        sel = [m for m, q in zip(ms, qs) if scope in ("all", q["library"])]
        res.setdefault("quality", {})[scope] = {"n": len(sel), "ndcg_at_5": mean([m["ndcg_at_5"] for m in sel]),
                                                "recall_at_5": mean([m["recall_at_5"] for m in sel])}
    allrel = sum(v["relevant"] for v in res["libs"].values())
    res["coverage_overall"] = sum(v["coverage"] * v["relevant"] for v in res["libs"].values()) / allrel
    res["per_question_ndcg"] = [m["ndcg_at_5"] for m in ms]
    (OUT / "result.json").write_text(json.dumps(res, indent=1), encoding="utf-8")
    say(f"RESULT coverage {res['coverage_overall']:.3f} quality {res['quality']}")


if __name__ == "__main__":
    main()
