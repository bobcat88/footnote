"""mem0 (mem0ai) as a bench engine: raw storage, no LLM call, local embedding model, Qdrant in local file mode.

    bench/.venv-mem0/Scripts/python.exe -I bench/run.py scifact ext:mem0
    bench/.venv-mem0/Scripts/python.exe -I bench/run.py alloprof ext:mem0-multilingual

Setup follows bench/research/competitors.md (mem0 2.2.1 source read on 2026-11-13) and mem0's own raw-add mode (`infer=False`), the same
pattern basic-memory's benchmark harness uses. What this measures: mem0's retrieval path as it ships locally, NOT mem0's managed platform
with LLM fact extraction, which is how its published scores were produced. Telemetry is off and the store lives in a scratch folder.
"""
from __future__ import annotations

import hashlib
import importlib.metadata as md
import os
import shutil
import tempfile
import time
from pathlib import Path

os.environ.setdefault("MEM0_TELEMETRY", "False")  # before importing mem0: no posthog events, no notice fetch

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
WORK = BENCH / "work"


def _bytes(path: Path) -> int:
    if path.is_file():
        return path.stat().st_size
    return sum(f.stat().st_size for f in path.rglob("*") if f.is_file()) if path.is_dir() else 0


class Mem0Engine:
    id = "mem0"
    embedder = "BAAI/bge-small-en-v1.5"  # mem0's documented local small option through fastembed, 384 dimensions
    dims = 384
    latency_scope = "in-process Python call to mem0 Memory.search (embedder loaded once, Qdrant local mode, no LLM)"

    def __init__(self) -> None:
        self.m = None
        self.root: Path | None = None
        self.config = {"product": "mem0", "mode": "raw add (infer=False), no LLM call", "embedder": f"fastembed {self.embedder}, {self.dims}d",
                       "lexical": "Qdrant sparse BM25 (fastembed Qdrant/bm25, English), re-scores dense candidates only",
                       "fusion": "mem0 score_and_rank: (semantic + bm25) / max, candidates from the dense top max(4k, 60)",
                       "threshold": 0.0, "chunking": "none (one memory per document)", "k": 20, "query_clip_bytes": 500,
                       "caveat": "not mem0's managed platform with LLM extraction; English BM25 on French datasets"}

    def versions(self) -> dict:
        return {"mem0ai": md.version("mem0ai"), "fastembed": md.version("fastembed"), "qdrant_client": md.version("qdrant-client"),
                "embedder": self.embedder, "adapter_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def _cfg(self) -> dict:
        root = self.root
        return {
            # mem0 builds an LLM client at init; it is never called with infer=False, and no key is used
            "llm": {"provider": "openai", "config": {"api_key": "sk-unused", "model": "gpt-5-mini"}},
            "embedder": {"provider": "fastembed", "config": {"model": self.embedder, "embedding_dims": self.dims}},
            "vector_store": {"provider": "qdrant", "config": {"collection_name": "bench", "embedding_model_dims": self.dims,
                                                             "path": str(root / "qdrant"), "on_disk": True}},
            "history_db_path": str(root / "history.db"),
        }

    def prepare(self, dataset: str, corpus: dict) -> dict:
        self.close()
        self.root = WORK / dataset / self.id
        if self.root.exists():
            shutil.rmtree(self.root)
        self.root.mkdir(parents=True)
        os.environ["MEM0_DIR"] = str(self.root / "home")
        from mem0 import Memory  # noqa: PLC0415  (after MEM0_DIR is set)
        t = time.perf_counter()
        self.m = Memory.from_config(self._cfg())
        skipped = 0
        for doc_id, (title, text) in corpus.items():
            body = f"{title}\n{text}".strip() if title else text.strip()
            if not body:
                skipped += 1
                continue
            self.m.add([{"role": "user", "content": body}], user_id="bench", metadata={"doc_id": doc_id}, infer=False)
        build_ms = (time.perf_counter() - t) * 1000
        store = _bytes(self.root / "qdrant") + _bytes(self.root / "history.db")
        cache = Path(os.environ.get("FASTEMBED_CACHE_PATH", Path(tempfile.gettempdir()) / "fastembed_cache"))
        return {"documents": len(corpus), "skipped_empty": skipped, "build_ms": build_ms, "embed_ms": None,
                "disk_bytes": store, "store_bytes": store, "qdrant_bytes": _bytes(self.root / "qdrant"),
                "history_db_bytes": _bytes(self.root / "history.db"), "model_cache_bytes": _bytes(cache), "chunks": len(corpus) - skipped}

    def search(self, qid: str, text: str, k: int) -> list[str]:
        q = text.encode("utf-8")[:500].decode("utf-8", errors="ignore")
        r = self.m.search(q, top_k=k, filters={"user_id": "bench"}, threshold=0.0)
        return [x["metadata"]["doc_id"] for x in r["results"]]

    def close(self) -> None:
        if self.m is not None:
            for store in ("vector_store", "_telemetry_vector_store"):
                client = getattr(getattr(self.m, store, None), "client", None)
                if client is not None:
                    try:
                        client.close()
                    except Exception:
                        pass
            self.m = None


class Mem0Multilingual(Mem0Engine):
    """Same, with a multilingual embedder from fastembed's catalogue, for the French and mixed datasets."""
    id = "mem0-multilingual"
    embedder = "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2"
    dims = 384


ENGINES = {e.id: e for e in (Mem0Engine, Mem0Multilingual)}
