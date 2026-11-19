"""Sphinx documentation rematch, Footnote against Grounded Docs 3.4.1 (protocol: DOCS-SPHINX-2026-11-19.md, written before any run).

    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_rematch.py <run-dir> <footnote.exe> <order> [cooldown-seconds]

<order> says which system crawls a library first, e.g. `django=fn,jinja=gd,sphinx=fn` (gd = Grounded Docs, fn = Footnote); a
cool-down separates the two systems on one library so a rate limit hit by the first cannot spoil the host for the second.
DOCS_DATASET and DOCS_RECIPE name the question set and crawl recipe, DOCS_SEED the bootstrap seed. Steps resume after a failure.
"""
from __future__ import annotations

import json
import os
import statistics
import sys
import time
from pathlib import Path

ARGS, sys.argv = sys.argv, sys.argv[:1]
RUN = Path(ARGS[1]).resolve()
os.environ["DOCS_RUN"] = str(RUN)
os.environ.setdefault("DOCS_CRAWL_WORK", str(RUN / "crawl"))
sys.path.insert(0, str(Path(__file__).parent))
import docs_crawl as c  # noqa: E402

g = c.g
EXE, ORDER = str(Path(ARGS[2]).resolve()), dict(kv.split("=") for kv in ARGS[3].split(","))
COOLDOWN = int(ARGS[4]) if len(ARGS) > 4 else 600
V = "minimal"
SEED = int(os.environ.get("DOCS_SEED", 20261119))
SEARCH = ["--limit", "5", "--per-source", "1", "--mode", "hybrid", *[a for m in g.MODELS for a in ("--model", m)]]


def timed(path: Path, key: str, f) -> None:
    done = json.loads(path.read_text()) if path.exists() else {}
    if key in done:
        return
    t = time.time()
    f()
    done[key] = round(time.time() - t, 1)
    path.write_text(json.dumps(done, indent=1))


def crawl_both(full: dict) -> None:
    for lib in full:
        g.RECIPE = {lib: full[lib]}
        steps = [("gd", g.scrape), ("fn", lambda: c.crawl(V, EXE))]
        if ORDER[lib] == "fn":
            steps.reverse()
        for i, (name, f) in enumerate(steps):
            if i:
                g.log(f"cool-down {COOLDOWN} s before {name} on {lib}")
                time.sleep(COOLDOWN)
            timed(RUN / "crawl-seconds.json", f"{lib}/{name}", f)
    g.RECIPE = full


def index_seconds(db_root: Path, corpus: callable) -> None:
    for lib in g.RECIPE:
        db = db_root / f"{lib}.db"
        db.parent.mkdir(parents=True, exist_ok=True)
        timed(RUN / "index-seconds.json", f"{db_root.name}/{lib}", lambda: g.fn_build(db, corpus(lib)))


def mean(xs):
    return sum(xs) / len(xs)


def main() -> None:
    RUN.mkdir(parents=True, exist_ok=True)
    (c.WORK / V).mkdir(parents=True, exist_ok=True)
    full = dict(g.RECIPE)
    qs = c.queries()
    g.log(f"rematch {RUN}: {len(qs)} queries, order {ORDER}, binary {EXE}")
    server = g.llama_server()
    try:
        crawl_both(full)
        gd = g.cached("gd-hybrid", lambda: g.gd_search(qs, True))
        manifest = json.loads((RUN / "manifest.json").read_text()) if (RUN / "manifest.json").exists() else g.export()
        g.FOOTNOTE = EXE
        c.coverage(V)
        c.endtoend(V, SEARCH)  # own crawl, indexed and searched twice (identical), paired against gd-hybrid.json
        fn_own = json.loads((c.WORK / V / "fn-hybrid-pass1.json").read_text())
        # Same text: Footnote on the pages Grounded Docs stored, to separate the crawl from the ranking.
        index_seconds(RUN / "footnote-same", lambda lib: RUN / "corpus" / lib)

        def same(i: int):
            q = qs[i]
            hits = g.fn(RUN / "footnote-same" / f"{q['library']}.db", "search", q["query"], *SEARCH)["results"]
            return [manifest.get(g.key(h["path"]), h["path"]) for h in hits]

        fn_same = g.cached("fn-same-hybrid", lambda: {str(i): same(i) for i in range(len(qs))})
        # Latency: both CLIs as the harness runs them, one query at a time, alternating so they share the machine's load.
        lat = {"gd": [], "fn": []}
        for i, q in enumerate(qs):
            for name in (("gd", "fn") if i % 2 == 0 else ("fn", "gd")):
                t = time.time()
                if name == "gd":
                    g.gd_search([q], True)
                else:
                    g.fn(c.WORK / V / "footnote" / f"{q['library']}.db", "search", q["query"], *SEARCH)
                lat[name].append(time.time() - t)
    finally:
        server.terminate()
        server.wait(timeout=30)
        g.log(f"llama-server {server.pid} stopped")
    systems = {"grounded": gd, "footnote": fn_own, "footnote_same_text": fn_same}
    per = {n: [g.metrics(r[str(i)], q["qrels"]) for i, q in enumerate(qs)] for n, r in systems.items()}
    nd = {n: [m["ndcg_at_5"] for m in ms] for n, ms in per.items()}
    libs = sorted({q["library"] for q in qs})
    sel = lambda n, lib=None: [x for x, q in zip(nd[n], qs) if lib in (None, q["library"])]
    out = {"seed": SEED, "binary": EXE, "latency_median_s": {k: statistics.median(v) for k, v in lat.items()},
           "latency_mean_s": {k: mean(v) for k, v in lat.items()},
           "crawl_seconds": json.loads((RUN / "crawl-seconds.json").read_text()),
           "index_seconds": json.loads((RUN / "index-seconds.json").read_text()) if (RUN / "index-seconds.json").exists() else {},
           "rows": {}}
    for scope in [None, *libs]:
        row = {"questions": len(sel("grounded", scope)),
               **{n: mean(sel(n, scope)) for n in systems}}
        for n in ("footnote", "footnote_same_text"):
            d, lo, hi = g.bootstrap(sel(n, scope), sel("grounded", scope), seed=SEED)
            row[f"{n}_minus_grounded"] = {"diff": d, "ci95": [lo, hi],
                                          "verdict": "better" if lo > 0 else "equivalent" if lo > -0.05 else "worse"}
        out["rows"][scope or "all"] = row
    for n in systems:
        for k in ("recall_at_5", "hit_at_3", "mrr"):
            out.setdefault("secondary", {}).setdefault(n, {})[k] = mean([m[k] for m in per[n]])
    (RUN / "rematch.json").write_text(json.dumps(out, indent=1))
    for scope, r in out["rows"].items():
        g.log(f"{scope:10} n={r['questions']:2} grounded {r['grounded']:.3f} footnote {r['footnote']:.3f} "
              f"({r['footnote_minus_grounded']['diff']:+.3f} {r['footnote_minus_grounded']['ci95']}) "
              f"same-text {r['footnote_same_text']:.3f} ({r['footnote_same_text_minus_grounded']['diff']:+.3f})")
    g.log(f"latency median s per query {out['latency_median_s']}; crawl {out['crawl_seconds']}; index {out['index_seconds']}")


if __name__ == "__main__":
    main()
