"""Cognee as a bench engine: its LLM-free indexing pipeline (classify, chunk, store and embed chunks), then chunk search.

    bench/.venv-cognee/Scripts/python.exe -I bench/run.py scifact ext:cognee
    bench/.venv-cognee/Scripts/python.exe -I bench/run.py scifact ext:cognee-lexical
    bench/.venv-cognee/Scripts/python.exe -I bench/run.py alloprof ext:cognee-multilingual

Setup follows bench/research/competitors.md and Cognee 1.6.8's own source: `get_indexing_tasks()` ("no task calls the LLM") run through the public
`cognee.run_custom_pipeline`, documents added as `DataItem`s with pinned ids, search with `SearchType.CHUNKS` (dense, LanceDB) or `CHUNKS_LEXICAL`
(Cognee's in-process lexical scorer). What this does NOT exercise: Cognee's knowledge graph and LLM summaries, which are what Cognee is known for.
Cognee does not fuse its dense and lexical results, so each is reported as its own system. Telemetry is off, keys are removed from the environment.
"""
from __future__ import annotations

import asyncio
import hashlib
import importlib.metadata as md
import os
import shutil
import tempfile
import time
import uuid
from pathlib import Path

HERE = Path(__file__).resolve().parent
WORK = HERE.parent / "work"
NS = uuid.UUID("00000000-0000-0000-0000-00000000b0b0")


def _bytes(path: Path) -> int:
    if path.is_file():
        return path.stat().st_size
    return sum(f.stat().st_size for f in path.rglob("*") if f.is_file()) if path.is_dir() else 0


class CogneeEngine:
    id = "cognee"
    search_type = "CHUNKS"
    embedder = "BAAI/bge-small-en-v1.5"  # Cognee's documented keyless default (fastembed), 384 dimensions
    chunk_tokens = 512
    latency_scope = "in-process Python call to cognee.search (async, one event loop for the run; embedder loaded once; no LLM)"

    def __init__(self) -> None:
        self.loop = asyncio.new_event_loop()
        self.root: Path | None = None
        self.by_hash: dict[str, str] = {}
        self.config = {"product": "cognee", "pipeline": "get_indexing_tasks via run_custom_pipeline (classify, chunk, embed), no LLM",
                       "search": f"SearchType.{self.search_type}", "embedder": f"fastembed {self.embedder}, 384d", "vector_store": "LanceDB (default)",
                       "chunking": f"Cognee TextChunker, {self.chunk_tokens} tokens", "k": 20, "query_clip_bytes": 500,
                       "caveat": "the knowledge graph and LLM summaries are not built or searched; no fusion of dense and lexical in Cognee"}

    def versions(self) -> dict:
        return {"cognee": md.version("cognee"), "fastembed": md.version("fastembed"), "lancedb": md.version("lancedb"),
                "embedder": self.embedder, "adapter_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def prepare(self, dataset: str, corpus: dict) -> dict:
        self.root = WORK / dataset / self.id
        if self.root.exists():
            shutil.rmtree(self.root)
        self.root.mkdir(parents=True)
        for v in ("LLM_API_KEY", "OPENAI_API_KEY"):
            os.environ.pop(v, None)
        os.environ.update({"AUTO_FEEDBACK": "false", "TELEMETRY_DISABLED": "true",
                           "SYSTEM_ROOT_DIRECTORY": str(self.root / "system"), "DATA_ROOT_DIRECTORY": str(self.root / "data"),
                           "EMBEDDING_PROVIDER": "fastembed", "EMBEDDING_MODEL": self.embedder, "EMBEDDING_DIMENSIONS": "384",
                           "GRAPH_EXTRACTOR": "gliner_demo", "GLINER_AUTO_INSTALL": "false"})
        import cognee  # noqa: PLC0415  (after the environment is set)
        from cognee.api.v1.cognify.cognify import get_indexing_tasks  # noqa: PLC0415
        from cognee.tasks.ingestion.data_item import DataItem  # noqa: PLC0415
        self.cognee = cognee
        items, skipped = [], 0
        for doc_id, (title, text) in corpus.items():
            body = f"{title}\n{text}".strip() if title else text.strip()
            if not body:
                skipped += 1
                continue
            self.by_hash[hashlib.md5(body.encode("utf-8")).hexdigest()] = doc_id
            items.append(DataItem(data=body, label=doc_id, data_id=uuid.uuid5(NS, doc_id), literal_text=True))
        self.by_uuid = {str(uuid.uuid5(NS, d)): d for d in corpus}
        t = time.perf_counter()

        async def build():
            await cognee.add(items, dataset_name="bench")
            tasks = await get_indexing_tasks(chunk_size=self.chunk_tokens)
            await cognee.run_custom_pipeline(tasks=tasks, dataset="bench", skip_connection_test=True)
        self.loop.run_until_complete(build())
        build_ms = (time.perf_counter() - t) * 1000
        system, data = self.root / "system", self.root / "data"
        lance = next((p for p in system.rglob("*.lance") if p.is_dir()), None)
        cache = Path(os.environ.get("FASTEMBED_CACHE_PATH", Path(tempfile.gettempdir()) / "fastembed_cache"))
        return {"documents": len(corpus), "skipped_empty": skipped, "build_ms": build_ms, "embed_ms": None,
                "disk_bytes": _bytes(system) + _bytes(data), "store_bytes": _bytes(system) + _bytes(data),
                "system_bytes": _bytes(system), "raw_copy_bytes": _bytes(data),
                "lancedb_bytes": _bytes(lance.parent) if lance else None, "model_cache_bytes": _bytes(cache), "chunks": None}

    def _doc(self, hit: dict) -> str | None:
        if hit.get("document_id") in self.by_uuid:
            return self.by_uuid[hit["document_id"]]
        h = hit.get("source_content_hash") or (hit.get("document_name") or "").removeprefix("text_").split(".")[0]
        return self.by_hash.get(h)

    def search(self, qid: str, text: str, k: int) -> list[str]:
        from cognee import SearchType  # noqa: PLC0415
        q = text.encode("utf-8")[:500].decode("utf-8", errors="ignore")
        res = self.loop.run_until_complete(self.cognee.search(query_text=q, query_type=getattr(SearchType, self.search_type),
                                                              datasets=["bench"], top_k=k * 3))
        out, seen = [], set()
        for block in res:
            for hit in block.get("search_result", []) if isinstance(block, dict) else []:
                d = self._doc(hit) if isinstance(hit, dict) else None
                if d and d not in seen:
                    seen.add(d)
                    out.append(d)
        return out[:k]

    def close(self) -> None:
        pass


class CogneeLexical(CogneeEngine):
    id = "cognee-lexical"
    search_type = "CHUNKS_LEXICAL"


class CogneeMultilingual(CogneeEngine):
    id = "cognee-multilingual"
    embedder = "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2"


ENGINES = {e.id: e for e in (CogneeEngine, CogneeLexical, CogneeMultilingual)}
