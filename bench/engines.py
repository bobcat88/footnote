"""Engine adapters. Each turns a corpus into an index and answers queries with ranked document ids.

Interface (duck-typed):
    id: str                                   stable name used in receipts
    config: dict                              everything that affects results, recorded in the receipt
    prepare(dataset, corpus) -> dict          build (or reuse) the index; returns {"build_ms", "disk_bytes", ...}
    search(qid, text, k) -> list[str]         ranked document ids (may contain duplicates; the runner dedupes)
    versions() -> dict                        engine/library versions for the receipt

All CLI-based engines share one chunker and one tokenizer (the product's), so differences between them are the query
semantics or the ranking stage, never the corpus handling.
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import sqlite3
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
WORK = HERE / "work"  # no dot in the path: the indexer silently skips dot-folders
EXE = REPO / "target" / "release" / "footnote.exe"
MODEL_DIR = REPO / "artifacts" / "models" / "minilm"
E5_DIR = REPO / "artifacts" / "models" / "e5-small"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def run_cli(db: Path, *args: str, timeout: int = 3600) -> dict:
    out = subprocess.run([str(EXE), "--db", str(db), *args], capture_output=True, text=True, timeout=timeout)
    if out.returncode != 0:
        raise RuntimeError(f"footnote {' '.join(args)} failed: {(out.stderr or out.stdout)[:500]}")
    return json.loads(out.stdout)


def materialize(dataset: str, corpus: dict[str, tuple[str, str]]) -> Path:
    """One .txt per document, named by its id. The title becomes the H1 so the indexer uses it as context."""
    folder = WORK / dataset / "corpus"
    marker = WORK / dataset / "materialized.txt"  # outside the corpus folder: the indexer would index it
    if marker.exists() and marker.read_text() == str(len(corpus)):
        return folder
    if folder.exists():
        shutil.rmtree(folder)
    folder.mkdir(parents=True)
    marker.parent.mkdir(parents=True, exist_ok=True)
    for doc_id, (title, text) in corpus.items():
        body = f"# {title}\n\n{text}\n" if title else f"{text}\n"
        (folder / f"{doc_id}.txt").write_text(body, encoding="utf-8")
    marker.write_text(str(len(corpus)))
    return folder


QUERY_CLIP_BYTES = 500  # the CLI rejects queries over 512 bytes; clip on a character boundary (protocol v1, recorded in receipts)


def clip(text: str) -> str:
    return text.encode("utf-8")[:QUERY_CLIP_BYTES].decode("utf-8", errors="ignore")


def doc_id_of(path: str) -> str:
    return Path(path.replace("\\\\?\\", "")).stem


class CliEngine:
    """Shared build step: the product's own `index` (and, for hybrid, `rebuild-embeddings`)."""

    family = "lex"
    hybrid = False
    model_dir = MODEL_DIR

    def __init__(self) -> None:
        self.db: Path | None = None
        self._k = 20

    def close(self) -> None:
        """Release per-index resources (open connections block deleting the index on Windows)."""

    def versions(self) -> dict:
        v = {"binary_sha256": sha256_file(EXE), "python_sqlite": sqlite3.sqlite_version}
        try:
            v["binary_sqlite"] = run_cli(self.db, "stats")["sqlite_version"] if self.db and self.db.exists() else None
        except Exception:
            v["binary_sqlite"] = None
        if self.hybrid:
            manifests = [json.loads((d / "manifest.json").read_text()) for d in getattr(self, "model_dirs", [self.model_dir])]
            v["model_repository"] = [m["repository"] for m in manifests] if len(manifests) > 1 else manifests[0]["repository"]
            v["model_revision"] = [m["revision"] for m in manifests] if len(manifests) > 1 else manifests[0]["revision"]
            v["model_files_sha256"] = [{n: f["sha256"] for n, f in m["files"].items()} for m in manifests] if len(manifests) > 1                 else {n: f["sha256"] for n, f in manifests[0]["files"].items()}
        return v

    def prepare(self, dataset: str, corpus: dict) -> dict:
        folder = materialize(dataset, corpus)
        base = WORK / dataset / self.family
        base.mkdir(parents=True, exist_ok=True)
        self.db = base / "k.db"
        meta = base / "build.json"
        exe_hash = sha256_file(EXE)
        if meta.exists() and self.db.exists():
            m = json.loads(meta.read_text())
            if m.get("exe_sha256") == exe_hash and m.get("documents") == len(corpus):
                m["reused"] = True
                return m
        if self.db.exists():
            self.db.unlink()
        t = time.perf_counter()
        indexed = run_cli(self.db, "index", "--root", str(folder), "--allow", str(folder))
        if indexed.get("failed"):
            raise RuntimeError(f"{indexed['failed']} documents failed to index: {indexed.get('failures')[:3]}")
        embed_ms = None
        if self.hybrid:
            e = time.perf_counter()
            models = [a for d in getattr(self, "model_dirs", [self.model_dir]) for a in ("--model", str(d))]
            run_cli(self.db, "rebuild-embeddings", *models)
            embed_ms = (time.perf_counter() - e) * 1000
        stats = run_cli(self.db, "stats")
        info = {"exe_sha256": exe_hash, "documents": len(corpus), "build_ms": (time.perf_counter() - t) * 1000,
                "embed_ms": embed_ms, "disk_bytes": self.db.stat().st_size, "chunks": stats["chunks"],
                "sources": stats["sources"], "reused": False}
        meta.write_text(json.dumps(info, indent=2))
        return info


class ShippedLexical(CliEngine):
    """The CLI exactly as shipped: every query word joined with AND, no stemming, FTS5 unicode61."""

    id = "shipped-lexical"
    latency_scope = "fresh CLI process per query (includes process spawn)"
    config = {"query": "AND of all whitespace tokens (as shipped)", "tokenizer": "unicode61 remove_diacritics 2",
              "ranking": "bm25()", "stemming": None, "chunk_chars": 2000, "k": 20, "query_clip_bytes": QUERY_CLIP_BYTES}

    def search(self, qid: str, text: str, k: int) -> list[str]:
        out = run_cli(self.db, "search", clip(text), "--limit", str(min(k, 20)))
        return [doc_id_of(r["path"]) for r in out["results"]]


class Bm25OrLexical(CliEngine):
    """Same index and tokenizer as the product; only the query changes from AND to OR, ranked by bm25()."""

    id = "bm25-or-lexical"
    latency_scope = "in-process SQLite query on the product index (no process spawn): NOT comparable with CLI engines"
    config = {"query": "OR of all whitespace tokens", "tokenizer": "unicode61 remove_diacritics 2",
              "ranking": "bm25() default k1=1.2 b=0.75, equal column weights", "stemming": None,
              "chunk_chars": 2000, "k": 20, "query_clip_bytes": QUERY_CLIP_BYTES}

    def close(self) -> None:
        if getattr(self, "_con", None) is not None:
            self._con.close()
            self._con = None

    def prepare(self, dataset: str, corpus: dict) -> dict:
        self.close()  # a new index means a new connection: never answer from the previous one
        return super().prepare(dataset, corpus)

    def search(self, qid: str, text: str, k: int) -> list[str]:
        if getattr(self, "_con", None) is None:
            self._con = sqlite3.connect(f"file:{self.db.as_posix()}?mode=ro", uri=True)
        terms = [f'"{t.replace(chr(34), chr(34) * 2)}"' for t in clip(text).split()]
        sql = ("SELECT s.path FROM search_fts f JOIN chunks c ON c.id=f.rowid JOIN sources s ON s.id=c.source_id "
               "WHERE search_fts MATCH ?1 AND s.status='indexed' ORDER BY bm25(search_fts), c.id LIMIT ?2")
        rows = self._con.execute(sql, (" OR ".join(terms), k * 3)).fetchall()
        return [doc_id_of(r[0]) for r in rows]


class ShippedHybrid(CliEngine):
    """The CLI exactly as shipped in hybrid mode: AND lexical top 20 + dense, fused by RRF (k=60)."""

    id = "shipped-hybrid"
    latency_scope = "fresh CLI process per query (includes process spawn and model load in the Python helper)"
    family = "hyb"
    hybrid = True
    config = {"lexical": "AND (as shipped), top 20", "dense": "multilingual MiniLM-L12 int8 384d, brute-force cosine",
              "fusion": "RRF k=60", "model_max_tokens": 128, "chunk_chars": 2000, "k": 20,
              "process": "fresh process per query (Python helper loads the model each time)", "query_clip_bytes": QUERY_CLIP_BYTES}

    def search(self, qid: str, text: str, k: int) -> list[str]:
        out = run_cli(self.db, "search", clip(text), "--mode", "hybrid", "--model", str(self.model_dir),
                      "--limit", str(min(k, 20)))
        return [doc_id_of(r["path"]) for r in out["results"]]


class ShippedHybridE5(ShippedHybrid):
    """Same pipeline as `shipped-hybrid`; only the embedding model changes (multilingual-e5-small, 512-token window)."""

    id = "shipped-hybrid-e5"
    family = "hyb-e5"
    model_dir = E5_DIR
    config = {**ShippedHybrid.config, "dense": "multilingual-e5-small int8 384d (query:/passage: prefixes), brute-force cosine",
              "model_max_tokens": 512}


class ShippedEnsemble(CliEngine):
    """The product as an agent uses it: ONE resident `mcp` session per index (JSON-RPC over stdio), hybrid mode with the default
    model pair. The first call pays the model load; every other query is what the agent sees. The state directory (audit log, MAC key,
    token) is a scratch folder, never the user's real one."""

    id = "shipped-ensemble-mcp"
    latency_scope = "one resident `mcp` session (models loaded once, helpers resident): JSON-RPC over stdio, per-query wall time"
    family = "ens"
    hybrid = True
    model_dirs = [REPO / "artifacts" / "models" / "bekko-a8m", REPO / "artifacts" / "models" / "gte-small"]
    config = {"lexical": "any word (OR), Tantivy BM25 en_stem, accents folded", "dense": "bekko-a8m + gte-small, 384d int8, blended 50/50",
              "fusion": "convex combination alpha 0.55 (z-scores over top 100)", "strategy": "full dense scan up to 50000 entries, else BM25 shortlist 1000",
              "chunk_chars": 2000, "k": 20, "process": "resident mcp session", "query_clip_bytes": QUERY_CLIP_BYTES}

    def __init__(self) -> None:
        super().__init__()
        self._proc = None
        self._n = 0

    def close(self) -> None:
        if self._proc is not None:
            try:
                self._proc.stdin.close()
                self._proc.wait(timeout=10)
            except Exception:
                self._proc.kill()
            self._proc = None

    def prepare(self, dataset: str, corpus: dict) -> dict:
        self.close()
        return super().prepare(dataset, corpus)

    def _rpc(self, method: str, params: dict | None = None, notify: bool = False):
        self._n += 1
        msg = {"jsonrpc": "2.0", "method": method, **({"params": params} if params is not None else {})}
        if not notify:
            msg["id"] = self._n
        self._proc.stdin.write(json.dumps(msg) + "\n")
        self._proc.stdin.flush()
        if notify:
            return None
        while True:
            line = self._proc.stdout.readline()
            if not line:
                raise RuntimeError("mcp session ended")
            reply = json.loads(line)
            if reply.get("id") == self._n:
                if "error" in reply:
                    raise RuntimeError(f"mcp error: {reply['error']}")
                return reply["result"]

    def _start(self) -> None:
        state = WORK / "state" / self.family
        state.mkdir(parents=True, exist_ok=True)
        env = {**os.environ, "BORG_STATE_DIR": str(state)}
        self._proc = subprocess.Popen([str(EXE), "--db", str(self.db), "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                      stderr=subprocess.DEVNULL, text=True, encoding="utf-8", bufsize=1, env=env)
        self._rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1"}})
        self._rpc("notifications/initialized", notify=True)

    def search(self, qid: str, text: str, k: int) -> list[str]:
        if self._proc is None:
            self._start()
        args = {"query": clip(text), "mode": "hybrid", "limit": min(k, 20), "model": [str(d) for d in self.model_dirs]}
        if getattr(self, "per_source", None):
            args["per_source"] = self.per_source
        result = self._rpc("tools/call", {"name": "footnote_recall", "arguments": args})
        payload = json.loads(result["content"][0]["text"]) if "content" in result else result["structuredContent"]
        return [doc_id_of(r["path"]) for r in payload["results"]]


ENGINES = {e.id: e for e in (ShippedLexical, Bm25OrLexical, ShippedHybrid, ShippedHybridE5, ShippedEnsemble)}
