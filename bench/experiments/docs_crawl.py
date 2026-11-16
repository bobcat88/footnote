"""Documentation crawl bench (protocol: DOCS-CRAWL.md, written before this run). B: coverage; C: end to end.

    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_crawl.py coverage <variant> [footnote.exe]
    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_crawl.py endtoend <variant> [footnote.exe]

<variant> names the crawl folder (bench/work/crawl/<variant>/<lib>) and the crawler: "minimal" runs `footnote crawl`, others are
crawled by their own command beforehand. Grounded Docs' side comes from DOCS-VS-GROUNDED run1 (same recipe, its own crawl).
"""
from __future__ import annotations

import json
import hashlib
import sqlite3
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import yaml

ARGS, sys.argv = sys.argv, sys.argv[:1]  # docs_vs_grounded reads its run folder from argv at import
sys.path.insert(0, str(Path(__file__).parent))
import docs_vs_grounded as g  # noqa: E402

WORK = Path(g.os.environ.get("DOCS_CRAWL_WORK", g.REPO / "bench/work/crawl")).resolve()  # manifest keys must match absolute hit paths
GD = g.RUN


def queries() -> list[dict]:
    return [e["vars"] for e in yaml.safe_load((g.DATASET).read_text(encoding="utf-8"))["entries"]]


def log(variant: str, msg: str) -> None:
    line = f"{time.strftime('%H:%M:%S')} {msg}"
    print(line, flush=True)
    with open(WORK / variant / "run.log", "a", encoding="utf-8") as f:
        f.write(line + "\n")


def crawl(variant: str, exe: str) -> None:
    for lib, (url, pages, exclude) in g.RECIPE.items():
        out = WORK / variant / lib
        if (out / "urls.json").exists():
            continue
        args = [exe, "crawl", url, "--out", str(out), "--max-pages", str(pages)]
        if exclude:
            args += ["--exclude", exclude[1:-1]]  # the recipe's patterns are JavaScript literals: /.../
        t = time.time()
        r = g.subprocess.run(args, capture_output=True, text=True, encoding="utf-8", timeout=7200)
        log(variant, f"crawl {lib}: exit {r.returncode}, {time.time() - t:.0f} s, {r.stdout.strip()[-400:]} {r.stderr.strip()[-300:]}")
        if r.returncode != 0 or not (out / "urls.json").is_file():
            raise RuntimeError(f"crawl failed for {lib}; incomplete output retained at {out}")


def urls_of(variant: str, lib: str) -> dict[str, str]:
    """File path (manifest key form) -> page URL."""
    folder = WORK / variant / lib
    return {g.key(str(folder / f)): u for f, u in json.loads((folder / "urls.json").read_text(encoding="utf-8")).items()}


def coverage(variant: str) -> dict:
    gd_manifest = json.loads((GD / "manifest.json").read_text(encoding="utf-8"))
    out = {}
    for lib in g.RECIPE:
        rel = {g.normalize_url(r["url"]) for q in queries() if q["library"] == lib for r in q["qrels"] if r["grade"] >= 2}
        mine = {g.normalize_url(u) for u in urls_of(variant, lib).values()}
        theirs = {g.normalize_url(u) for f, u in gd_manifest.items() if f"/corpus/{lib}/" in f}
        out[lib] = {"relevant": len(rel), variant: len(rel & mine) / len(rel), "grounded": len(rel & theirs) / len(rel),
                    "pages": len(mine), "grounded_pages": len(theirs), "missed": sorted(rel - mine)}
    allrel = sum(v["relevant"] for v in out.values())
    out["overall"] = {k: sum(v[k] * v["relevant"] for v in out.values()) / allrel for k in (variant, "grounded")}
    (WORK / variant / "coverage.json").write_text(json.dumps(out, indent=1), encoding="utf-8")
    for lib, v in out.items():
        log(variant, f"coverage {lib:12} " + " ".join(f"{k} {v[k]:.3f}" if isinstance(v[k], float) else f"{k} {v[k]}"
                                                      for k in v if k != "missed"))
    return out


def endtoend(variant: str, search_args: list[str]) -> None:
    qs = queries()
    result_path = WORK / variant / "endtoend.json"
    if result_path.exists():
        raise FileExistsError(f"refusing to overwrite {result_path}")
    input_paths = [Path(g.FOOTNOTE), g.DATASET, GD / "gd-hybrid.json",
                   g.REPO / "bench/experiments/DOCS-CRAWL.md", Path(__file__), Path(g.__file__),
                   *[Path(m) / "manifest.json" for m in g.MODELS]]
    input_paths += [p for lib in g.RECIPE for p in (WORK / variant / lib).glob("*.md")]
    input_paths += [WORK / variant / lib / "urls.json" for lib in g.RECIPE]
    def sha(path):
        with path.open("rb") as f:
            return hashlib.file_digest(f, "sha256").hexdigest()
    inputs = {str(path): sha(path) for path in input_paths}
    input_receipt = WORK / variant / "endtoend-inputs.json"
    if input_receipt.exists() and json.loads(input_receipt.read_text(encoding="utf-8")) != inputs:
        raise RuntimeError("end-to-end inputs changed since the previous attempt")
    input_receipt.write_text(json.dumps(inputs, indent=2), encoding="utf-8")
    manifest = {}
    for lib in g.RECIPE:
        db = WORK / variant / "footnote" / f"{lib}.db"
        db.parent.mkdir(parents=True, exist_ok=True)
        complete = db.with_suffix(".complete.json")
        if not complete.exists():
            # Re-running index/rebuild is safe after an interrupted build; db existence alone is not proof of completeness.
            started = time.time()
            g.fn(db, "index", "--root", str(WORK / variant / lib), "--allow", str(WORK / variant / lib))
            g.fn(db, "rebuild-embeddings", *[a for m in g.MODELS for a in ("--model", m)])
            with sqlite3.connect(f"file:{db.as_posix()}?mode=ro", uri=True) as c:
                chunks = c.execute("SELECT count(*) FROM chunks c JOIN sources s ON s.id=c.source_id WHERE s.status='indexed'").fetchone()[0]
                rows = c.execute("SELECT model,count(*) FROM embeddings WHERE kind='chunk' GROUP BY model").fetchall()
                identities = {r[0] for r in c.execute("SELECT identity FROM embedding_models")}
            expected = {sha(Path(m) / "manifest.json") for m in g.MODELS}
            if not chunks or identities != expected or {r[0] for r in rows} != expected or any(n != chunks for _, n in rows):
                raise RuntimeError(f"incomplete embedding build {lib}: chunks={chunks}, vectors={rows}, models={identities}")
            complete.write_text(json.dumps({"chunks": chunks, "vectors": rows, "db_sha256": sha(db)}), encoding="utf-8")
            log(variant, f"footnote build {db.name}: {time.time() - started:.0f} s, {chunks} chunks per model")
        else:
            previous = json.loads(complete.read_text(encoding="utf-8"))
            if previous["db_sha256"] != sha(db):
                raise RuntimeError(f"completed database changed: {db}")
        manifest.update(urls_of(variant, lib))

    def one(i: int) -> tuple[str, list[str]]:
        q = qs[i]
        hits = g.fn(WORK / variant / "footnote" / f"{q['library']}.db", "search", q["query"], *search_args)["results"]
        return str(i), [manifest.get(g.key(h["path"]), h["path"]) for h in hits]

    runs = []
    for repeat in (1, 2):
        with ThreadPoolExecutor(8) as ex:  # quality only; no latency claim
            ranked = dict(ex.map(one, range(len(qs))))
        (WORK / variant / f"fn-hybrid-pass{repeat}.json").write_text(json.dumps(ranked), encoding="utf-8")
        runs.append(ranked)
        log(variant, f"end-to-end query pass {repeat}: {len(ranked)}")
    if runs[0] != runs[1]:
        raise RuntimeError("end-to-end rankings differ between repeats")
    gd = json.loads((GD / "gd-hybrid.json").read_text(encoding="utf-8"))
    nd = lambda r: [g.metrics(r[str(i)], q["qrels"])["ndcg_at_5"] for i, q in enumerate(qs)]
    d, lo, hi = g.bootstrap(nd(ranked), nd(gd), seed=int(g.os.environ.get("DOCS_SEED", 20261115)))
    verdict = "better" if lo > 0 else "equivalent" if lo > -0.05 else "worse"
    ms = [g.metrics(ranked[str(i)], q["qrels"]) for i, q in enumerate(qs)]
    summary = {k: sum(m[k] for m in ms) / len(ms) for k in ms[0]}
    log(variant, f"END-TO-END footnote({variant}) - grounded nDCG@5 {d:+.4f} [{lo:+.4f}, {hi:+.4f}] -> {verdict}; {summary}")
    if any(sha(path) != digest for path, digest in ((Path(p), h) for p, h in inputs.items())):
        raise RuntimeError("end-to-end inputs changed during the run")
    result_path.write_text(json.dumps({"summary": summary, "diff": d, "ci95": [lo, hi], "verdict": verdict,
                                      "search_args": search_args, "repeat_rankings_identical": True}, indent=1), encoding="utf-8")


if __name__ == "__main__":
    step, variant = ARGS[1], ARGS[2]
    if len(ARGS) > 3:
        g.FOOTNOTE = ARGS[3]
    (WORK / variant).mkdir(parents=True, exist_ok=True)
    g.log = lambda msg: log(variant, msg)
    if step == "coverage":
        if variant == "minimal":
            crawl(variant, g.FOOTNOTE)
        coverage(variant)
    else:
        endtoend(variant, ARGS[4:] or ["--limit", "5", "--mode", "hybrid", *[a for m in g.MODELS for a in ("--model", m)]])
