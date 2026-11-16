"""Diagnostic after the sealed final (DOCS-OPTIMIZE-2026-11-16.md, deviation 2): re-crawl Flask once Cloudflare stops
challenging this machine, then score the Flask questions on that crawl. Not the verdict; the as-run crawl is.

    bench/.labenv/Scripts/python.exe -I bench/experiments/heldout3_flask_recrawl.py <footnote.exe>

Checks one page every 15 minutes for at most 4 hours (polite to the blocked host), then crawls into
bench/work/heldout3/crawl/recrawl/flask, builds, searches with the final's arguments and reports nDCG@5 against Grounded.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

EXE = sys.argv[1]
os.environ.update(DOCS_RUN="bench/work/heldout3/run1", DOCS_DATASET="bench/datasets/docs-heldout3/dataset.yaml",
                  DOCS_RECIPE="bench/datasets/docs-heldout3/recipe.json")
sys.argv = sys.argv[:1]
sys.path.insert(0, str(Path(__file__).parent))
import docs_vs_grounded as g  # noqa: E402
import yaml  # noqa: E402

OUT = (g.REPO / "bench/work/heldout3/crawl/recrawl").resolve()
PROBE = "https://flask.palletsprojects.com/en/stable/quickstart/"


def open_now() -> bool:
    try:
        with urllib.request.urlopen(urllib.request.Request(PROBE, headers={"User-Agent": "footnote-bench"}), timeout=20) as r:
            return r.status == 200 and b"Just a moment" not in r.read(4096)
    except Exception:
        return False


def main() -> None:
    for attempt in range(17):
        if open_now():
            break
        print(time.strftime("%H:%M:%S"), "still blocked", flush=True)
        time.sleep(900)
    else:
        print("gave up: Flask still blocked after 4 hours", flush=True)
        return
    url, pages, exclude = g.RECIPE["flask"]
    folder = OUT / "flask"
    t = time.time()
    r = subprocess.run([EXE, "crawl", url, "--out", str(folder), "--max-pages", str(pages), "--exclude", exclude[1:-1]],
                       capture_output=True, text=True, encoding="utf-8")
    crawl_s = time.time() - t
    print(time.strftime("%H:%M:%S"), f"crawl {crawl_s:.0f} s", r.stdout.strip()[-300:], flush=True)
    db = OUT / "flask.db"
    t = time.time()
    g.FOOTNOTE = EXE
    g.fn(db, "index", "--root", str(folder), "--allow", str(folder))
    g.fn(db, "rebuild-embeddings", *[a for m in g.MODELS for a in ("--model", m)])
    build_s = time.time() - t
    urls = {g.key(str(folder / f)): u for f, u in json.loads((folder / "urls.json").read_text(encoding="utf-8")).items()}
    qs = [e["vars"] for e in yaml.safe_load(g.DATASET.read_text(encoding="utf-8"))["entries"] if e["vars"]["library"] == "flask"]
    allq = [e["vars"] for e in yaml.safe_load(g.DATASET.read_text(encoding="utf-8"))["entries"]]
    gd = json.loads((g.RUN / "gd-hybrid.json").read_text(encoding="utf-8"))
    args = ["--limit", "5", "--per-source", "1", "--mode", "hybrid", *[a for m in g.MODELS for a in ("--model", m)]]
    mine, theirs = [], []
    for q in qs:
        hits = g.fn(db, "search", q["query"], *args)["results"]
        mine.append(g.metrics([urls.get(g.key(h["path"]), h["path"]) for h in hits], q["qrels"])["ndcg_at_5"])
        theirs.append(g.metrics(gd[str(allq.index(q))], q["qrels"])["ndcg_at_5"])
    d, lo, hi = g.bootstrap(mine, theirs, seed=20261117)
    rel = {g.normalize_url(r["url"]) for q in qs for r in q["qrels"] if r["grade"] >= 2}
    cov = len(rel & {g.normalize_url(u) for u in urls.values()}) / len(rel)
    result = {"crawl_seconds": crawl_s, "build_seconds": build_s, "pages": len(urls), "coverage": cov,
              "footnote_ndcg5": sum(mine) / len(mine), "grounded_ndcg5": sum(theirs) / len(theirs), "diff": d, "ci95": [lo, hi]}
    (OUT / "flask-recrawl.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
    print("FLASK RECRAWL", json.dumps(result), flush=True)


if __name__ == "__main__":
    main()
