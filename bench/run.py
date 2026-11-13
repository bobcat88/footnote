"""Run one engine on one dataset and write an immutable receipt (protocol v1).

    python bench/run.py scifact bm25-or-lexical
    python bench/run.py scifact shipped-hybrid --queries 20     # smoke run, receipt is marked partial

A receipt is never overwritten. Rankings are stored gzip-compressed next to it, hashed, so a later run can be diffed
and fusion/rerank experiments can reuse a ranking without re-running the engine.
"""
from __future__ import annotations

import argparse
import ctypes
import datetime as dt
import gzip
import hashlib
import json
import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import engines  # noqa: E402
import fetch  # noqa: E402
import metrics  # noqa: E402

PROTOCOL = "v1"
HERE = Path(__file__).resolve().parent
RECEIPTS = HERE / "receipts"
K = 20


def get_engine(name: str, lab_json: str | None = None):
    """`lab:<name>` builds a lab engine (config from --lab-json, else lab/configs.json); `ext:<name>` a competing product
    (bench/competitors/<product>_engine.py); anything else is a Footnote product engine."""
    if name.startswith("lab:"):
        sys.path.insert(0, str(HERE / "lab"))
        import lab
        cfg = json.loads(lab_json) if lab_json else json.loads((HERE / "lab" / "configs.json").read_text())[name[4:]]
        return lab.LabEngine(name[4:], cfg)
    if name.startswith("ext:"):  # competing products, each run from its own pinned environment (bench/competitors/)
        sys.path.insert(0, str(HERE / "competitors"))
        product = name[4:].split("-")[0]
        module = __import__(f"{product}_engine")
        return module.ENGINES[name[4:]]()
    return engines.ENGINES[name]()


def harness_hash() -> str:
    """One hash over every file that decides a result: a receipt is only comparable to receipts with the same value."""
    h = hashlib.sha256()
    for name in ("metrics.py", "fetch.py", "engines.py", "run.py", "run_lme.py", "convert_parquet.py", "lab/lab.py", "lab/models.py"):
        h.update(name.encode() + (HERE / name).read_bytes())
    return h.hexdigest()


def git(*args: str) -> str:
    return subprocess.run(["git", "-C", str(engines.REPO), *args], capture_output=True, text=True).stdout.strip()


def machine() -> dict:
    ram = None
    if os.name == "nt":
        class Mem(ctypes.Structure):
            _fields_ = [("l", ctypes.c_ulong), ("load", ctypes.c_ulong), ("total", ctypes.c_ulonglong),
                        ("avail", ctypes.c_ulonglong), ("pt", ctypes.c_ulonglong), ("pa", ctypes.c_ulonglong),
                        ("vt", ctypes.c_ulonglong), ("va", ctypes.c_ulonglong), ("ve", ctypes.c_ulonglong)]
        m = Mem(); m.l = ctypes.sizeof(Mem)
        ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(m))
        ram = round(m.total / 2**30, 1)
    return {"os": platform.platform(), "cpu": platform.processor(), "logical_cpus": os.cpu_count(), "ram_gib": ram,
            "python": platform.python_version()}


def rankings_hash(rankings: dict[str, list[str]]) -> str:
    blob = json.dumps({q: rankings[q] for q in sorted(rankings)}, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(blob.encode()).hexdigest()


def one_pass(engine, queries: dict[str, str], corpus_ids: set[str]) -> tuple[dict[str, list[str]], list[float]]:
    rankings, times = {}, []
    for qid, text in queries.items():
        t = time.perf_counter()
        rankings[qid] = metrics.dedupe(engine.search(qid, text, K))[:K]
        foreign = set(rankings[qid]) - corpus_ids
        if foreign:  # integrity: an engine answering from another index must never pass silently
            raise RuntimeError(f"query {qid}: ids outside the corpus returned: {sorted(foreign)[:3]}")
        times.append((time.perf_counter() - t) * 1000)
    return rankings, times


def pct(values: list[float], p: float) -> float:
    s = sorted(values)
    return s[min(len(s) - 1, int(round(p * (len(s) - 1))))]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("dataset", choices=sorted(n for n, s in fetch.DATASETS.items() if s["kind"] != "longmemeval"))
    ap.add_argument("engine", help="product engine: " + ", ".join(sorted(engines.ENGINES)) + " | lab:<name>")
    ap.add_argument("--split", choices=["test", "dev", "heldout"], default="test",
                    help="dev = tuning set, heldout = test minus tuning queries, test = official evaluation set")
    ap.add_argument("--lab-json", help="inline lab config (JSON) for lab:<name>")
    ap.add_argument("--queries", type=int, help="score only the first N queries (receipt is marked partial)")
    ap.add_argument("--no-repeat", action="store_true", help="skip the second pass (determinism then unchecked)")
    ap.add_argument("--det-queries", type=int, default=200,
                    help="second pass covers the first N queries only (keeps slow engines tractable)")
    args = ap.parse_args()

    corpus, queries, qrels = fetch.load(args.dataset, args.split)
    partial = bool(args.queries and args.queries < len(queries))
    if partial:
        queries = dict(list(queries.items())[: args.queries])
        qrels = {q: qrels[q] for q in queries}
    engine = get_engine(args.engine, args.lab_json)
    print(f"[{args.dataset}/{engine.id}] preparing index over {len(corpus)} documents ...", flush=True)
    build = engine.prepare(args.dataset, corpus)
    print(f"  build: {build}", flush=True)
    print(f"[{args.dataset}/{engine.id}] pass 1 over {len(queries)} queries ...", flush=True)
    r1, t1 = one_pass(engine, queries, set(corpus))
    h1 = rankings_hash(r1)
    deterministic = None
    det_n = 0
    if not args.no_repeat:
        sub = dict(list(queries.items())[: args.det_queries])
        print(f"  pass 2 (determinism check on {len(sub)} queries) ...", flush=True)
        r2, _ = one_pass(engine, sub, set(corpus))
        deterministic = rankings_hash(r2) == rankings_hash({q: r1[q] for q in sub})
        det_n = len(sub)
    scores = metrics.evaluate(r1, qrels)
    pin = fetch.dataset_pin(args.dataset)
    now = dt.datetime.now(dt.timezone.utc)
    stamp = now.strftime("%Y%m%dT%H%M%SZ")
    safe_id = engine.id.replace(":", "_")
    name = f"{stamp}-{args.dataset}-{safe_id}{'' if args.split == 'test' else '-' + args.split}{'-partial' if partial else ''}"
    receipt = {
        "protocol": PROTOCOL,
        "timestamp_utc": now.isoformat(timespec="seconds"),
        "partial": partial,
        "dataset": {"id": args.dataset, "split": args.split, "pin": pin, "documents": len(corpus), "queries": len(queries)},
        "engine": {"id": engine.id, "config": engine.config, "versions": engine.versions()},
        "code": {"commit": git("rev-parse", "HEAD"), "dirty": bool(git("status", "--porcelain", "--", "src", "bench")),
                 "metrics_sha256": hashlib.sha256((HERE / "metrics.py").read_bytes()).hexdigest(),
                 "harness_sha256": harness_hash()},
        "machine": machine(),
        "build": build,
        "metrics": {k: (round(v, 6) if isinstance(v, float) else v) for k, v in scores.items()},
        "latency_ms": {"scope": engine.latency_scope,
                       "p50": round(statistics.median(t1), 2), "p95": round(pct(t1, 0.95), 2),
                       "mean": round(statistics.fmean(t1), 2), "queries": len(t1)},
        "determinism": {"checked": deterministic is not None, "queries_checked": det_n, "identical_rankings": deterministic,
                        "rankings_sha256": h1},
    }
    folder = RECEIPTS if args.split == "test" else RECEIPTS / args.split  # tuning receipts never mix with reportable ones
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / f"{name}.json"
    if path.exists():
        raise SystemExit(f"{path} exists; receipts are immutable")
    path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    with gzip.open(folder / f"{name}.rankings.json.gz", "wt", encoding="utf-8") as f:
        json.dump({q: r1[q] for q in sorted(r1)}, f, ensure_ascii=False, separators=(",", ":"))
    m = receipt["metrics"]
    print(f"\n{name}\n  nDCG@10={m['ndcg@10']}  R@5={m['recall@5']}  R@10={m['recall@10']}  R@20={m['recall@20']}  "
          f"MRR@10={m['mrr@10']}  empty={m['queries_empty']}/{m['queries_scored']}\n  "
          f"p50={receipt['latency_ms']['p50']} ms  p95={receipt['latency_ms']['p95']} ms  deterministic={deterministic}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
