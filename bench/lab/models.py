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
    # Same model and revision, unquantized: build-speed test 1 (bench/experiments/BUILD-SPEED.md).
    "gte-small-fp32": {"repo": "Xenova/gte-small", "onnx": "onnx/model.onnx", "pooling": "mean", "max_len": 512,
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
    # Quantized exports of the same EmbeddingGemma (overnight Q3-1b): same prefixes and in-graph pooling, smaller files
    "embeddinggemma-300m-q8": {"repo": "onnx-community/embeddinggemma-300m-ONNX", "onnx": "onnx/model_quantized.onnx",
                               "extra": {"model_quantized.onnx_data": "onnx/model_quantized.onnx_data"}, "pooling": "graph",
                               "output": "sentence_embedding", "max_len": 512, "q": "task: search result | query: ",
                               "p": "title: none | text: ", "mteb": "google__embeddinggemma-300m", "lang": "multi"},
    "embeddinggemma-300m-q4": {"repo": "onnx-community/embeddinggemma-300m-ONNX", "onnx": "onnx/model_q4.onnx",
                               "extra": {"model_q4.onnx_data": "onnx/model_q4.onnx_data"}, "pooling": "graph",
                               "output": "sentence_embedding", "max_len": 512, "q": "task: search result | query: ",
                               "p": "title: none | text: ", "mteb": "google__embeddinggemma-300m", "lang": "multi"},
    "harrier-270m": {"repo": "onnx-community/harrier-oss-v1-270m-ONNX", "onnx": "onnx/model.onnx",
                     "extra": {"model.onnx_data": "onnx/model.onnx_data"}, "pooling": "last", "max_len": 512,
                     "q": "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: ", "p": "",
                     "mteb": "microsoft__harrier-oss-v1-270m", "lang": "multi"},
    "granite-small-en-r2": {"repo": "onnx-community/granite-embedding-small-english-r2-ONNX", "onnx": "onnx/model.onnx",
                            "extra": {"model.onnx_data": "onnx/model.onnx_data"}, "pooling": "cls", "max_len": 512, "q": "", "p": "",
                            "mteb": "ibm-granite__granite-embedding-small-english-r2", "lang": "en"},
    "arctic-m-v2": {"repo": "Snowflake/snowflake-arctic-embed-m-v2.0", "onnx": "onnx/model.onnx", "pooling": "cls", "max_len": 512,
                    "q": "query: ", "p": "", "mteb": "Snowflake__snowflake-arctic-embed-m-v2.0", "lang": "multi"},
    # Test 5 (bench/experiments/LLAMACPP-MODELS.md): ONNX candidates that would run on the shipped ONNX Runtime path
    "embeddinggemma2-onnx-q8": {"repo": "onnx-community/embeddinggemma-2-ONNX", "onnx": "onnx/model_quantized.onnx",
                                "extra": {"model_quantized.onnx_data": "onnx/model_quantized.onnx_data"}, "pooling": "graph",
                                "output": "sentence_embedding", "max_len": 512, "q": "task: search result | query: ",
                                "p": "title: none | text: ", "mteb": "google__embeddinggemma-2", "lang": "multi"},
    "embeddinggemma2-onnx": {"repo": "onnx-community/embeddinggemma-2-ONNX", "onnx": "onnx/model.onnx",
                             "extra": {"model.onnx_data": "onnx/model.onnx_data"}, "pooling": "graph",
                             "output": "sentence_embedding", "max_len": 512, "q": "task: search result | query: ",
                             "p": "title: none | text: ", "mteb": "google__embeddinggemma-2", "lang": "multi"},
    "granite-311m-multi-r2": {"repo": "ibm-granite/granite-embedding-311m-multilingual-r2", "onnx": "onnx/model_quint8_avx2.onnx",
                              "pooling": "cls", "max_len": 512, "q": "", "p": "",
                              "mteb": "ibm-granite__granite-embedding-311m-multilingual-r2", "lang": "multi"},
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


# GGUF models run by llama.cpp (bench/experiments/LLAMACPP-MODELS.md, test 5). Files live in artifacts/gguf and are checked against the
# pinned SHA-256 at every load. "ngl" > 0 encodes documents on the GPU; queries always run on the CPU.
_GEMMA = {"gguf": {"repo": "ggml-org/embeddinggemma-300M-GGUF", "revision": "0f741b5a6585bd53aeb15cd1372c56f2a0f65e12",
                   "file": "embeddinggemma-300M-Q8_0.gguf", "sha256": "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"},
          "max_len": 512, "q": "task: search result | query: ", "p": "title: none | text: ", "lang": "multi"}
_GEMMA2 = {**_GEMMA, "gguf": {"repo": "ggml-org/embeddinggemma-2-GGUF", "revision": "6ea3e3a309d12a4779cfda98b8c19800d2d5da71",
                               "file": "embeddinggemma-2-Q8_0.gguf",
                               "sha256": "2188ac1deca4b77dffefd603c2776a9d76d9d74ec01841392982ebb840b09135"}}
_QWEN3 = {"gguf": {"repo": "Qwen/Qwen3-Embedding-0.6B-GGUF", "revision": "370f27d7550e0def9b39c1f16d3fbaa13aa67728",
                   "file": "Qwen3-Embedding-0.6B-Q8_0.gguf", "sha256": "06507c7b42688469c4e7298b0a1e16deff06caf291cf0a5b278c308249c3e439"},
          "pooling": "last", "suffix": "<|endoftext|>", "max_len": 512,
          "q": "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: ", "p": "", "lang": "multi"}
_BGEM3 = {"gguf": {"repo": "ggml-org/bge-m3-Q8_0-GGUF", "revision": "9eba04c5d75ba5a1595e45de734d36bef4e5cb98",
                   "file": "bge-m3-q8_0.gguf", "sha256": "aa473d51f451a22f0fcf39ba3330c14bed38a385712b1113440f69df4047a173"},
          "max_len": 512, "q": "", "p": "", "lang": "multi"}
_JINA = {"pooling": "last", "max_len": 512, "q": "Query: ", "p": "Document: ", "lang": "multi"}
_JNANO = {**_JINA, "gguf": {"repo": "jinaai/jina-embeddings-v5-text-nano-retrieval-GGUF", "revision": "59cfaceeeb7d738c404659435af4c0da74d06c96",
                            "file": "v5-nano-retrieval-Q8_0.gguf", "sha256": "86b6e6279e9b9e71389f02a082764a2ac2b15a50e37482c26f98d69092f12442"}}
_JSMALL = {**_JINA, "gguf": {"repo": "jinaai/jina-embeddings-v5-text-small-retrieval-GGUF", "revision": "78b0ebcb4c870fdfef409e578b65288b49a4fa90",
                             "file": "v5-small-retrieval-Q8_0.gguf", "sha256": "b759677362414e664160ffb017fbc74c300feaa4ad4085f69f2cc1bfa12ccb71"}}
# GPU class (documents and queries on the GPU): too slow for the CPU
MODELS["gguf-qwen3-4b-gpu"] = {**_QWEN3, "ngl": 99, "qngl": 99, "gguf": {
    "repo": "Qwen/Qwen3-Embedding-4B-GGUF", "revision": "f4602530db1d980e16da9d7d3a70294cf5c190be", "file": "Qwen3-Embedding-4B-Q8_0.gguf",
    "sha256": "b60ae5ce2dd6a0b77f82cadf21def1f310a3e10cde380ad0081b07a9d416949d"}}
MODELS["gguf-qwen3-8b-gpu"] = {**_QWEN3, "ngl": 99, "qngl": 99, "gguf": {
    "repo": "Qwen/Qwen3-Embedding-8B-GGUF", "revision": "69d0e58a13e463cd99a9b83e3f5fee7c10265fab", "file": "Qwen3-Embedding-8B-Q8_0.gguf",
    "sha256": "d20ddc71e8a5c4344f2343481e242233a997dc5eaff442427a945836c97b4deb"}}
for _name, _spec in {"gguf-jina-v5-nano": _JNANO, "gguf-jina-v5-small": _JSMALL, "gguf-embeddinggemma": _GEMMA, "gguf-embeddinggemma2": _GEMMA2, "gguf-qwen3-0.6b": _QWEN3,
                     "gguf-bge-m3": _BGEM3}.items():
    MODELS[_name] = {**_spec, "ngl": 0}
    MODELS[_name + "-gpu"] = {**_spec, "ngl": 99}
GGUF_DIR = REPO / "artifacts" / "gguf"

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
    req = urllib.request.Request(url, headers={"User-Agent": "footnote-bench/1"})
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


def gguf_path(name: str) -> Path:
    g = MODELS[name]["gguf"]
    path = GGUF_DIR / g["file"]
    if not path.exists():
        print(f"downloading {g['repo']}:{g['file']} ...", flush=True)
        _get(f"https://huggingface.co/{g['repo']}/resolve/{g['revision']}/{g['file']}", path)
    if sha256_file(path) != g["sha256"]:
        raise SystemExit(f"{name}: {path} does not match its pinned SHA-256; refusing to use it")
    return path


def lock_entry(name: str) -> dict:
    spec = MODELS.get(name) or RERANKERS[name]
    if "gguf" in spec:
        g = spec["gguf"]
        return {"repo": g["repo"], "revision": g["revision"], "files": {g["file"]: g["sha256"]}}
    if "local" in spec:
        manifest = json.loads((REPO / spec["local"] / "manifest.json").read_text())
        return {"repo": manifest["repository"], "revision": manifest["revision"],
                "files": {n: f["sha256"] for n, f in manifest["files"].items()}}
    entry = read_lock()[name]
    return {"repo": entry["repo"], "revision": entry["revision"], "files": {n: f["sha256"] for n, f in entry["files"].items()}}
