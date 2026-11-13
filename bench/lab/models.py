"""Model registry and hash-locked download for the lab. Every model is pinned to a Hugging Face commit and SHA-256-locked on first use.

A model config (pooling, prefixes, max length) is only trusted after `check_model.py` reproduces its published MTEB score within tolerance.
"""
from __future__ import annotations

import hashlib
import json
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
REPO = BENCH.parent
CACHE = BENCH / ".cache" / "models"
LOCK = BENCH / "models.lock.json"
Q_BGE = "Represent this sentence for searching relevant passages: "

# name -> spec. `local` models are the ones the product already ships; the others are downloaded.
MODELS: dict[str, dict] = {
    # shipped by the product (reference points)
    "minilm-l12-multi": {"local": "artifacts/models/minilm", "pooling": "mean", "max_len": 128, "q": "", "p": "",
                         "mteb": "sentence-transformers__paraphrase-multilingual-MiniLM-L12-v2", "lang": "multi"},
    "e5-small-multi": {"local": "artifacts/models/e5-small", "pooling": "mean", "max_len": 512, "q": "query: ", "p": "passage: ",
                       "mteb": "intfloat__multilingual-e5-small", "lang": "multi"},
    # English candidates
    "bge-small-en": {"repo": "Xenova/bge-small-en-v1.5", "onnx": "onnx/model_quantized.onnx", "pooling": "cls",
                     "max_len": 512, "q": Q_BGE, "p": "", "mteb": "BAAI__bge-small-en-v1.5", "lang": "en"},
    "gte-small": {"repo": "Xenova/gte-small", "onnx": "onnx/model_quantized.onnx", "pooling": "mean", "max_len": 512,
                  "q": "", "p": "", "mteb": "thenlper__gte-small", "lang": "en"},
    "minilm-l6-en": {"repo": "Xenova/all-MiniLM-L6-v2", "onnx": "onnx/model_quantized.onnx", "pooling": "mean",
                     "max_len": 256, "q": "", "p": "", "mteb": "sentence-transformers__all-MiniLM-L6-v2", "lang": "en"},
    # multilingual candidates
    "bekko-a8m": {"repo": "hotchpotch/bekko-embedding-v1-a8m", "onnx": "onnx/model.onnx", "pooling": "mean", "max_len": 512,
                  "q": "", "p": "", "mteb": "hotchpotch__bekko-embedding-v1-a8m", "lang": "multi"},
    "bekko-a25m": {"repo": "hotchpotch/bekko-embedding-v1-a25m", "onnx": "onnx/model.onnx", "pooling": "mean", "max_len": 512,
                   "q": "", "p": "", "mteb": "hotchpotch__bekko-embedding-v1-a25m", "lang": "multi"},
    "granite-97m-multi": {"repo": "ibm-granite/granite-embedding-97m-multilingual-r2", "onnx": "onnx/model_quint8_avx2.onnx",
                          "pooling": "cls", "max_len": 512, "q": "", "p": "", "mteb": "ibm-granite__granite-embedding-97m-multilingual-r2", "lang": "multi"},
    "e5-base-multi": {"repo": "Xenova/multilingual-e5-base", "onnx": "onnx/model_quantized.onnx", "pooling": "mean",
                      "max_len": 512, "q": "query: ", "p": "passage: ", "mteb": "intfloat__multilingual-e5-base", "lang": "multi"},
    # 2026 candidates from bench/research/embedding-boost.md (prefixes and pooling from each model card)
    "embeddinggemma-300m": {"repo": "onnx-community/embeddinggemma-300m-ONNX", "onnx": "onnx/model.onnx",
                            "extra": {"model.onnx_data": "onnx/model.onnx_data"}, "pooling": "graph", "output": "sentence_embedding",
                            "max_len": 512, "q": "task: search result | query: ", "p": "title: none | text: ",
                            "mteb": "google__embeddinggemma-300m", "lang": "multi"},
    "harrier-270m": {"repo": "onnx-community/harrier-oss-v1-270m-ONNX", "onnx": "onnx/model.onnx",
                     "extra": {"model.onnx_data": "onnx/model.onnx_data"}, "pooling": "last", "max_len": 512,
                     "q": "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: ", "p": "",
                     "mteb": "microsoft__harrier-oss-v1-270m", "lang": "multi"},
    "granite-small-en-r2": {"repo": "onnx-community/granite-embedding-small-english-r2-ONNX", "onnx": "onnx/model.onnx",
                            "extra": {"model.onnx_data": "onnx/model.onnx_data"}, "pooling": "cls", "max_len": 512, "q": "", "p": "",
                            "mteb": "ibm-granite__granite-embedding-small-english-r2", "lang": "en"},
    "arctic-m-v2": {"repo": "Snowflake/snowflake-arctic-embed-m-v2.0", "onnx": "onnx/model.onnx", "pooling": "cls", "max_len": 512,
                    "q": "query: ", "p": "", "mteb": "Snowflake__snowflake-arctic-embed-m-v2.0", "lang": "multi"},
    "bge-m3": {"repo": "onnx-community/bge-m3-ONNX", "onnx": "onnx/model_quantized.onnx", "pooling": "cls", "max_len": 512,
               "q": "", "p": "", "mteb": "BAAI__bge-m3", "lang": "multi"},
}


RERANKERS: dict[str, dict] = {
    # English cross-encoder (Apache-2.0), 87 MB quantized
    "mxbai-xsmall": {"repo": "mixedbread-ai/mxbai-rerank-xsmall-v1", "onnx": "onnx/model_quantized.onnx", "max_len": 512, "lang": "en"},
    # multilingual mMiniLMv2 distilled on mMARCO (the one our earlier ablations used), 119 MB quantized
    "mmarco-mminilm": {"repo": "cross-encoder/mmarco-mMiniLMv2-L12-H384-v1", "onnx": "onnx/model_quint8_avx2.onnx", "max_len": 512, "lang": "multi"},
    # multilingual ceiling, 571 MB quantized
    "bge-reranker-v2-m3": {"repo": "onnx-community/bge-reranker-v2-m3-ONNX", "onnx": "onnx/model_quantized.onnx", "max_len": 512, "lang": "multi"},
}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 22), b""):
            h.update(block)
    return h.hexdigest()


def read_lock() -> dict:
    return json.loads(LOCK.read_text(encoding="utf-8")) if LOCK.exists() else {}


def write_lock(lock: dict) -> None:
    tmp = LOCK.with_suffix(".tmp")
    tmp.write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    tmp.replace(LOCK)


def _get(url: str, dest: Path | None = None):
    req = urllib.request.Request(url, headers={"User-Agent": "borg-bench/1"})
    with urllib.request.urlopen(req, timeout=300) as r:
        if dest is None:
            return r.read()
        dest.parent.mkdir(parents=True, exist_ok=True)
        with dest.open("wb") as out:
            while block := r.read(1 << 22):
                out.write(block)


def model_dir(name: str) -> Path:
    """Folder holding model.onnx + tokenizer.json, downloaded and hash-locked on first use."""
    spec = MODELS.get(name) or RERANKERS[name]
    if "local" in spec:
        return REPO / spec["local"]
    lock = read_lock()
    entry = lock.get(name)
    if entry is None:
        info = json.loads(_get(f"https://huggingface.co/api/models/{spec['repo']}"))
        entry = {"repo": spec["repo"], "revision": info["sha"], "files": {}}
        lock[name] = entry
    folder = CACHE / name
    wanted = {"model.onnx": spec["onnx"], "tokenizer.json": "tokenizer.json", **spec.get("extra", {})}
    changed = False
    for local, remote in wanted.items():
        dest = folder / local
        if not dest.exists():
            print(f"downloading {spec['repo']}:{remote} ...", flush=True)
            _get(f"https://huggingface.co/{spec['repo']}/resolve/{entry['revision']}/{remote}", dest)
        digest = sha256_file(dest)
        known = entry["files"].get(local)
        if known and known["sha256"] != digest:
            raise SystemExit(f"{name}/{local}: SHA-256 {digest} does not match the lock; refusing to use it")
        if not known:
            entry["files"][local] = {"source": remote, "sha256": digest, "bytes": dest.stat().st_size}
            changed = True
    if changed:
        write_lock(lock)
    return folder


def lock_entry(name: str) -> dict:
    spec = MODELS.get(name) or RERANKERS[name]
    if "local" in spec:
        manifest = json.loads((REPO / spec["local"] / "manifest.json").read_text())
        return {"repo": manifest["repository"], "revision": manifest["revision"],
                "files": {n: f["sha256"] for n, f in manifest["files"].items()}}
    entry = read_lock()[name]
    return {"repo": entry["repo"], "revision": entry["revision"], "files": {n: f["sha256"] for n, f in entry["files"].items()}}
