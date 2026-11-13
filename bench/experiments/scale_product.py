"""E14: latency of the SHIPPED binary at scale, in its shortlist strategy.

    bench/.labenv/Scripts/python.exe -I bench/experiments/scale_product.py 100000

What is real: the documents' text (150-word spans cut from real corpora), the indexer (`footnote index`), the Tantivy index, the resident
MCP session, the two embedding models (every query is really encoded), the shortlist, the fetch by id, the fusion.
What is synthetic: the stored document vectors, random unit vectors quantized to int8 (encoding 100,000 chunks with two models would
take hours and the vectors' values do not change the cost of the search). So this measures SPEED, never precision.
The state directory is a scratch folder. Rows go to bench/sweeps/scale_product.jsonl.
"""
from __future__ import annotations

import hashlib
import json
import os
import random
import shutil
import sqlite3
import statistics
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
sys.path.insert(0, str(BENCH))
import engines  # noqa: E402
import fetch  # noqa: E402

REPO = engines.REPO
DIM = 384


def spans(n: int):
    corpus, _, _ = fetch.load("mix-fr-en", "test")
    words = [(t + " " + x).split() for t, x in corpus.values()]
    rng = random.Random(20261112)
    for i in range(n):
        w = rng.choice(words)
        s = rng.randrange(0, max(1, len(w) - 150))
        yield i, " ".join(w[s:s + 150])


def sha(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def main() -> int:
    n = int(sys.argv[1])
    work = engines.WORK / "scale_product"
    if work.exists():
        shutil.rmtree(work)
    folder = work / "corpus"
    folder.mkdir(parents=True)
    t0 = time.perf_counter()
    for i, text in spans(n):
        (folder / f"d{i}.txt").write_text(text + "\n", encoding="utf-8")
    db = work / "k.db"
    env_state = work / "state"
    env_state.mkdir()
    os.environ["FOOTNOTE_STATE_DIR"] = str(env_state)
    t1 = time.perf_counter()
    engines.run_cli(db, "index", "--root", str(folder), "--allow", str(folder), timeout=7200)
    index_s = time.perf_counter() - t1
    # synthetic vectors for both models, with the signatures the binary expects
    helper = sha((REPO / "tools" / "embed.py").read_bytes())
    models = [REPO / "artifacts" / "models" / m for m in ("bekko-a8m", "gte-small")]
    con = sqlite3.connect(db)
    rng = random.Random(7)
    t2 = time.perf_counter()
    rows = con.execute("SELECT c.id,s.sha256,s.revision,c.heading,c.text FROM chunks c JOIN sources s ON s.id=c.source_id").fetchall()
    for m in models:
        manifest = (m / "manifest.json").read_bytes()
        identity = sha(manifest)
        con.execute("INSERT OR REPLACE INTO embedding_models VALUES(?,?,?,384)", (identity, manifest.decode(), helper))
        batch = []
        for cid, s_sha, rev, heading, text in rows:
            v = [rng.gauss(0, 1) for _ in range(DIM)]
            norm = sum(x * x for x in v) ** 0.5
            v = [x / norm for x in v]
            scale = max(abs(x) for x in v) / 127.0
            blob = struct.pack("<f", scale) + bytes((round(x / scale) & 0xFF) for x in v)
            signature = sha(f"{s_sha}:{rev}:{heading}\n{text}".encode())
            batch.append((identity, "chunk", cid, signature, blob))
        con.executemany("INSERT OR REPLACE INTO embeddings VALUES(?,?,?,?,?)", batch)
    con.commit()
    con.close()
    vec_s = time.perf_counter() - t2
    # queries: real, encoded for real
    _, queries, _ = fetch.load("mix-fr-en", "test")
    qs = [engines.clip(q) for q in list(queries.values())[:300]]
    eng = engines.ShippedEnsemble()
    eng.db = db
    os.environ["FOOTNOTE_STATE_DIR"] = str(env_state)
    for q in qs[:20]:
        eng.search("w", q, 10)  # warm-up: model load and caches
    times = []
    for q in qs:
        a = time.perf_counter()
        eng.search("q", q, 10)
        times.append((time.perf_counter() - a) * 1000)
    eng.close()
    pct = lambda p: sorted(times)[int(p * (len(times) - 1))]
    row = {"experiment": "E14", "chunks": len(rows), "documents": n, "strategy": "full_dense_scan" if len(rows) <= 50000 else "lexical_shortlist",
           "queries": len(qs), "p50_ms": round(statistics.median(times), 1), "p95_ms": round(pct(0.95), 1), "max_ms": round(max(times), 1),
           "index_s": round(index_s, 1), "synthetic_vectors_s": round(vec_s, 1), "db_mb": round(db.stat().st_size / 1e6, 1),
           "tantivy_mb": round(sum(f.stat().st_size for f in Path(str(db) + ".tantivy").rglob("*") if f.is_file()) / 1e6, 1),
           "note": "document vectors are random int8; every query is encoded by the two real models; speed only"}
    print(json.dumps(row))
    with (BENCH / "sweeps" / "scale_product.jsonl").open("a", encoding="utf-8") as f:
        f.write(json.dumps(row) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
