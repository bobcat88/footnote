"""Split lab encoding time into tokenization, ONNX inference and NumPy pooling, same steps as lab.Encoder.embed.

    bench/.labenv/Scripts/python.exe -I bench/experiments/profile_encode.py --dataset scifact --docs 2000 --model bekko-a8m --model gte-small

Answers which stage would an optimisation (Rust tokenizer, NumKong, Numba, OpenVINO...) actually touch?
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

BENCH = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(BENCH), str(BENCH / "lab")]
import fetch  # noqa: E402
from lab import Encoder  # noqa: E402


def profile(enc: Encoder, texts: list[str], batch: int = 16) -> dict:
    """Mirror of Encoder.embed with timers; returns seconds per stage."""
    t = {"tokenize": 0.0, "onnx": 0.0, "pool_normalize": 0.0}
    order = np.argsort([len(x) for x in texts], kind="stable")
    for i in range(0, len(texts), batch):
        idx = order[i:i + batch]
        s = time.perf_counter()
        enc_b = enc.tok.encode_batch([enc.spec["p"] + texts[j] for j in idx])
        ids = np.array([e.ids for e in enc_b], dtype=np.int64)
        mask = np.array([e.attention_mask for e in enc_b], dtype=np.int64)
        t["tokenize"] += time.perf_counter() - s
        feed = {"input_ids": ids, "attention_mask": mask}
        if "token_type_ids" in enc.inputs:
            feed["token_type_ids"] = np.zeros_like(ids)
        s = time.perf_counter()
        graph = enc.spec["pooling"] == "graph"
        h = enc.sess.run([enc.spec.get("output", "sentence_embedding")] if graph else None, feed)[0]
        t["onnx"] += time.perf_counter() - s
        s = time.perf_counter()
        if h.ndim == 3:
            m = mask[..., None].astype(np.float32)
            h = h[:, 0] if enc.spec["pooling"] == "cls" else (h * m).sum(1) / np.maximum(m.sum(1), 1e-9)
        h / np.maximum(np.linalg.norm(h, axis=1, keepdims=True), 1e-12)
        t["pool_normalize"] += time.perf_counter() - s
    total = sum(t.values())
    return {k: {"s": round(v, 3), "share": round(v / total, 4)} for k, v in t.items()} | {"total_s": round(total, 3),
                                                                                         "docs_per_s": round(len(texts) / total, 1)}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dataset", default="scifact")
    ap.add_argument("--docs", type=int, default=2000)
    ap.add_argument("--model", action="append", required=True)
    args = ap.parse_args()
    corpus, _, _ = fetch.load(args.dataset, "test")
    texts = [f"{corpus[d][0]}\n{corpus[d][1]}".strip() for d in sorted(corpus)[:args.docs]]
    for name in args.model:
        enc = Encoder(name)
        enc.embed(texts[:16], "passage")  # warm-up
        print(json.dumps({"model": name, "dataset": args.dataset, "docs": len(texts), **profile(enc, texts)}), flush=True)


if __name__ == "__main__":
    main()
