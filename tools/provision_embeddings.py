"""Explicit provisioning only: fetch immutable PUBLIC model artifacts, never documents.

    python tools/provision_embeddings.py                  provision every recipe below
    python tools/provision_embeddings.py bekko-a8m gte-small

Each artifact is pinned to a Hugging Face commit. Recipes with `sha256` are verified against the pinned hashes (the same ones the
benchmark lock holds: bench/models.lock.json) and refused on a mismatch; the manifest written beside the files is what the Rust core
and the embedding helper check before every use.
"""
import hashlib
import json
import sys
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
CANDIDATES = {
    # Default pair (benchmark protocol v1, see bench/experiments/RESULTS.md): a multilingual model and an English one, blended 50/50.
    "bekko-a8m": {
        "repo": "hotchpotch/bekko-embedding-v1-a8m", "revision": "c721113d59a1d91b447450324f51c4b3332c924a",
        "onnx": "onnx/model.onnx", "query_prefix": "", "passage_prefix": "", "max_length": 512, "pad_token": "<pad>",
        "quantization": "none (fp32)",
        "sha256": {"model.onnx": "96d8cc6199e96357b21b2fb12f6d7ffd2d4abc7b182fe94b5468fbd6dc819af7",
                   "tokenizer.json": "8bd47075711f75a143d1b78e01a41cc65c1c591b00d3cfeffc23db07adce1392"},
    },
    "gte-small": {
        "repo": "Xenova/gte-small", "revision": "5927d1727bb12db490052a1b33265ad78058de08",
        "onnx": "onnx/model_quantized.onnx", "query_prefix": "", "passage_prefix": "", "max_length": 512, "pad_token": "[PAD]",
        "quantization": "onnx-dynamic-int8",
        "sha256": {"model.onnx": "18dec105109b6004369799ca4761fb8fb413c64172c02147bcfac186b5c5f6cb",
                   "tokenizer.json": "da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0"},
    },
    # Earlier models, kept for the existing tests and for comparison.
    "e5-small": {
        "repo": "Xenova/multilingual-e5-small", "revision": "761b726dd34fb83930e26aab4e9ac3899aa1fa78",
        "onnx": "onnx/model_quantized.onnx", "query_prefix": "query: ", "passage_prefix": "passage: ", "max_length": 512,
        "pad_token": "<pad>", "quantization": "onnx-dynamic-int8", "sha256": {},
    },
    "minilm": {
        "repo": "Xenova/paraphrase-multilingual-MiniLM-L12-v2", "revision": "2c4055b12046f11709e9df2c122e59ffbdc2f900",
        "onnx": "onnx/model_quantized.onnx", "query_prefix": "", "passage_prefix": "", "max_length": 128, "pad_token": "<pad>",
        "quantization": "onnx-dynamic-int8", "sha256": {},
    },
}


def provision(name):
    r = CANDIDATES[name]
    destination = ROOT / "artifacts/models" / name
    destination.mkdir(parents=True, exist_ok=True)
    artifacts = {}
    for remote, local in [("tokenizer.json", "tokenizer.json"), (r["onnx"], "model.onnx")]:
        path = destination / local
        url = f"https://huggingface.co/{r['repo']}/resolve/{r['revision']}/{remote}"
        if not path.exists():
            partial = path.with_suffix(".partial")
            urllib.request.urlretrieve(url, partial)
            partial.replace(path)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        pinned = r["sha256"].get(local)
        if pinned and digest != pinned:
            path.unlink()
            raise SystemExit(f"{name}/{local}: sha256 {digest} does not match the pinned {pinned}; file removed")
        artifacts[local] = {"sha256": digest, "bytes": path.stat().st_size, "url": url}
    manifest = {"repository": r["repo"], "revision": r["revision"], "files": artifacts, "pooling": "attention-mask-mean-l2",
                "query_prefix": r["query_prefix"], "passage_prefix": r["passage_prefix"], "max_length": r["max_length"],
                "pad_token": r["pad_token"], "dimension": 384, "quantization": r["quantization"]}
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"name": name, "path": destination.as_posix(), "manifest": manifest}))


if __name__ == "__main__":
    for name in sys.argv[1:] or CANDIDATES:
        provision(name)
