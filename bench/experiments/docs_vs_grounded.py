"""Technical documentation search: Footnote against Grounded Docs (protocol: DOCS-VS-GROUNDED.md, written before this run).

    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_vs_grounded.py [run-dir]

Steps, each skipped when its output exists (the run resumes after a failure): Grounded Docs crawls the five libraries with
EmbeddingGemma 300M served by llama.cpp, then answers the 59 queries (hybrid, then full-text only); the text it stored is exported
page by page; Footnote indexes it (one database per library, and one global database as a pollution check) and answers the same
queries; scoring is Grounded Docs' own (tests/search-eval/lib/metrics.cjs, ported line by line below).
"""
from __future__ import annotations

import json
import math
import os
import random
import re
import sqlite3
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

import yaml

REPO = Path(__file__).resolve().parents[2]
RUN = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(os.environ.get("DOCS_RUN", REPO / "bench/work/gd/run1"))
# Another question set and crawl recipe (bench/datasets/docs-confirm): DOCS_DATASET=<yaml>, DOCS_RECIPE=<json file>.
DATASET = Path(os.environ.get("DOCS_DATASET", REPO / "bench/work/gd/dataset.yaml"))
STORE = RUN / "store"
NPX = ["npx.cmd" if os.name == "nt" else "npx", "-y", "@arabold/docs-mcp-server@3.4.1"]
FOOTNOTE = str(REPO / "target/release/footnote.exe")
MODELS = [str(Path(os.environ["LOCALAPPDATA"]) / "footnote/models" / m) for m in ("bekko-a8m", "gte-small")]
LLAMA = os.environ.get("FOOTNOTE_LLAMA_SERVER", "llama-server")
GGUF = str(REPO / "artifacts/gguf/embeddinggemma-300M-Q8_0.gguf")
PORT = 8091
# Grounded Docs' CI recipe (.github/workflows/eval.yml at v3.4.1).
RECIPE = {
    "react": ("https://react.dev", 300, r"/^https:\/\/react\.dev\/[a-z]{2}(-[a-z]+)?\//"),
    "python": ("https://docs.python.org/3/", 500, r"/^https:\/\/docs\.python\.org\/3\/[a-z]{2}(-[a-z]+)?\//"),
    "fastapi": ("https://fastapi.tiangolo.com/", 200, r"/^https:\/\/fastapi\.tiangolo\.com\/[a-z]{2}(-[a-z]+)?\//"),
    "vite": ("https://vitejs.dev/guide/", 200, None),
    "tailwindcss": ("https://tailwindcss.com/docs", 300, None),
}
if os.environ.get("DOCS_RECIPE"):
    RECIPE = {k: tuple(v) for k, v in json.loads(Path(os.environ["DOCS_RECIPE"]).read_text(encoding="utf-8")).items()}


def log(msg: str) -> None:
    line = f"{time.strftime('%H:%M:%S')} {msg}"
    print(line, flush=True)
    with open(RUN / "run.log", "a", encoding="utf-8") as f:
        f.write(line + "\n")


def gd_env(embeddings: bool) -> dict:
    env = {k: v for k, v in os.environ.items() if not k.startswith(("OPENAI_", "DOCS_MCP_"))}
    env.update(DOCS_MCP_TELEMETRY="false", DOCS_MCP_STORE_PATH=str(STORE))
    if embeddings:  # an OpenAI-compatible local server; without these Grounded Docs searches full-text only
        env.update(OPENAI_API_KEY="local", OPENAI_API_BASE=f"http://127.0.0.1:{PORT}/v1", DOCS_MCP_EMBEDDING_MODEL="embeddinggemma-300m")
    return env


# --- Grounded Docs' metrics, ported from tests/search-eval/lib/metrics.cjs ------------------------------------------------
def normalize_url(u: str) -> str:
    s = u.strip().split("#", 1)[0].split("?", 1)[0]
    if s.endswith("/"):
        s = s[:-1]
    if s.endswith(".md"):
        s = s[:-3]
    elif s.endswith(".html"):
        s = s[:-5]
    return s


def metrics(ranked: list[str], qrels: list[dict]) -> dict:
    rel = {normalize_url(q["url"]): q["grade"] for q in qrels}
    r = [normalize_url(u) for u in ranked]
    mrr = next((1 / (i + 1) for i, u in enumerate(r) if u in rel), 0.0)

    def recall(k):
        return len({u for u in r[:k] if u in rel}) / len(rel) if rel else 0.0

    def dcg(k):
        s, seen = 0.0, set()
        for i, u in enumerate(r[:k]):
            if u in seen:
                continue
            seen.add(u)
            if rel.get(u, 0) > 0:
                s += (2 ** rel[u] - 1) / math.log2(i + 2)
        return s

    ideal = sum((2 ** g - 1) / math.log2(i + 2) for i, g in enumerate(sorted(rel.values(), reverse=True)[:5]))
    return {"ndcg_at_5": dcg(5) / ideal if ideal else 0.0, "mrr": mrr, "recall_at_5": recall(5),
            "hit_at_3": float(any(u in rel for u in r[:3]))}


# --- steps ----------------------------------------------------------------------------------------------------------------
def llama_server() -> subprocess.Popen:
    cmd = [LLAMA, "-m", GGUF, "--embedding", "-ngl", "99", "-np", "4", "-c", "8192", "-b", "8192", "-ub", "8192",
           "--host", "127.0.0.1", "--port", str(PORT), "--no-webui"]
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=open(RUN / "llama-server.log", "w"))
    for _ in range(120):
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2)
            log(f"llama-server ready, pid {proc.pid}")
            return proc
        except Exception:
            if proc.poll() is not None:
                raise RuntimeError("llama-server exited; see llama-server.log")
            time.sleep(1)
    raise RuntimeError("llama-server did not start")


def scraped(lib: str) -> int:
    db = STORE / "documents.db"
    if not db.exists():
        return 0
    c = sqlite3.connect(db)
    n = c.execute("SELECT count(*) FROM pages p JOIN versions v ON v.id=p.version_id JOIN libraries l ON l.id=v.library_id "
                  "WHERE l.name=? AND v.status='completed'", (lib,)).fetchone()[0]
    c.close()
    return n


def scrape() -> None:
    for lib, (url, pages, exclude) in RECIPE.items():
        if scraped(lib):
            log(f"scrape {lib}: already done ({scraped(lib)} pages)")
            continue
        args = [*NPX, "scrape", lib, url, "--max-pages", str(pages), "--no-telemetry"]
        if exclude:
            args += ["--exclude-pattern", exclude]
        t = time.time()
        out = subprocess.run(args, env=gd_env(True), capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=5400)
        log(f"scrape {lib}: exit {out.returncode}, {time.time() - t:.0f} s, {scraped(lib)} pages; {out.stdout.strip()[-200:]}")
        if out.returncode != 0:
            (RUN / f"scrape-{lib}.err").write_text(out.stderr[-20000:], encoding="utf-8")
            raise RuntimeError(f"scrape {lib} failed")


def gd_search(queries: list[dict], embeddings: bool) -> dict:
    out = {}
    for i, q in enumerate(queries):
        args = [*NPX, "search", q["library"], q["query"], "--limit", "5", "--output", "json", "--no-telemetry"]
        r = subprocess.run(args, env=gd_env(embeddings), capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=300)
        try:
            out[str(i)] = [h["url"] for h in json.loads(r.stdout)]
        except Exception:
            out[str(i)] = []
            log(f"gd search {i} failed: {r.stderr.strip()[-300:]}")
    return out


def export() -> dict:
    """Each page's stored text, its chunks in order, as one Markdown file; the manifest maps files back to URLs."""
    c = sqlite3.connect(STORE / "documents.db")
    manifest = {}
    for lib in RECIPE:
        folder = RUN / "corpus" / lib
        folder.mkdir(parents=True, exist_ok=True)
        rows = c.execute("SELECT p.id, p.url, d.content FROM documents d JOIN pages p ON p.id=d.page_id JOIN versions v "
                         "ON v.id=p.version_id JOIN libraries l ON l.id=v.library_id WHERE l.name=? ORDER BY p.id, d.sort_order",
                         (lib,)).fetchall()
        pages: dict[int, tuple[str, list[str]]] = {}
        for pid, url, content in rows:
            pages.setdefault(pid, (url, []))[1].append(content)
        for pid, (url, parts) in pages.items():
            path = folder / f"{pid}.md"
            path.write_text("\n\n".join(parts), encoding="utf-8")
            manifest[key(str(path.resolve()))] = url
        log(f"export {lib}: {len(pages)} pages")
    (RUN / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
    return manifest


def fn(db: Path, *args: str) -> dict:
    r = subprocess.run([FOOTNOTE, "--db", str(db), *args], capture_output=True, text=True, encoding="utf-8", timeout=3600)
    v = json.loads(r.stdout or "{}")
    if "error" in v:
        raise RuntimeError(f"footnote {args[0]}: {v['error']}")
    return v


def fn_build(db: Path, root: Path) -> None:
    if db.exists():
        return
    t = time.time()
    fn(db, "index", "--root", str(root), "--allow", str(root))
    fn(db, "rebuild-embeddings", *[a for m in MODELS for a in ("--model", m)])
    log(f"footnote build {db.name}: {time.time() - t:.0f} s")


def key(path: str) -> str:
    """Footnote stores canonical Windows paths (verbatim prefix); the manifest uses plain lower-case forward-slash paths."""
    return re.sub(r"^[\\/]{1,2}\?[\\/]", "", path).replace("\\", "/").lower()


def fn_search(queries: list[dict], manifest: dict, db_for, hybrid: bool) -> dict:
    out = {}
    for i, q in enumerate(queries):
        args = ["search", q["query"], "--limit", "5"]
        if hybrid:
            args += ["--mode", "hybrid", *[a for m in MODELS for a in ("--model", m)]]
        hits = fn(db_for(q["library"]), *args)["results"]
        out[str(i)] = [manifest.get(key(h["path"]), h["path"]) for h in hits]
    return out


def bootstrap(a: list[float], b: list[float], n: int = 10_000, seed: int = 20261115) -> tuple[float, float, float]:
    rng = random.Random(seed)
    d = [x - y for x, y in zip(a, b)]
    means = sorted(sum(d[rng.randrange(len(d))] for _ in d) / len(d) for _ in range(n))
    return sum(d) / len(d), means[int(0.025 * n)], means[int(0.975 * n) - 1]


def cached(name: str, make):
    path = RUN / f"{name}.json"
    if path.exists():
        return json.loads(path.read_text(encoding="utf-8"))
    v = make()
    path.write_text(json.dumps(v), encoding="utf-8")
    return v


def main() -> None:
    RUN.mkdir(parents=True, exist_ok=True)
    data = yaml.safe_load(DATASET.read_text(encoding="utf-8"))
    queries = [e["vars"] for e in data["entries"]]
    log(f"run {RUN}: {len(queries)} queries")
    need_server = not all(scraped(lib) for lib in RECIPE) or not (RUN / "gd-hybrid.json").exists()
    server = llama_server() if need_server else None
    try:
        scrape()
        gd_hybrid = cached("gd-hybrid", lambda: gd_search(queries, True))
    finally:
        if server:
            server.terminate()  # by PID, never by name
            server.wait(timeout=30)
            log(f"llama-server {server.pid} stopped")
    gd_fts = cached("gd-fts", lambda: gd_search(queries, False))
    manifest = json.loads((RUN / "manifest.json").read_text()) if (RUN / "manifest.json").exists() else export()
    per_lib = {lib: RUN / "footnote" / f"{lib}.db" for lib in RECIPE}
    for lib, db in per_lib.items():
        db.parent.mkdir(parents=True, exist_ok=True)
        fn_build(db, RUN / "corpus" / lib)
    glob = RUN / "footnote" / "_all.db"
    fn_build(glob, RUN / "corpus")
    runs = {
        "grounded_hybrid": gd_hybrid,
        "grounded_fulltext": gd_fts,
        "footnote_hybrid": cached("fn-hybrid", lambda: fn_search(queries, manifest, per_lib.get, True)),
        "footnote_keyword": cached("fn-keyword", lambda: fn_search(queries, manifest, per_lib.get, False)),
        "footnote_hybrid_global": cached("fn-hybrid-global", lambda: fn_search(queries, manifest, lambda _: glob, True)),
    }
    per_query = {name: [metrics(r[str(i)], q["qrels"]) for i, q in enumerate(queries)] for name, r in runs.items()}
    summary = {name: {k: sum(m[k] for m in ms) / len(ms) for k in ms[0]} for name, ms in per_query.items()}
    nd = {name: [m["ndcg_at_5"] for m in ms] for name, ms in per_query.items()}
    diff, lo, hi = bootstrap(nd["footnote_hybrid"], nd["grounded_hybrid"])
    verdict = "better" if lo > 0 else "equivalent" if lo > -0.05 else "worse"
    g_diff, g_lo, g_hi = bootstrap(nd["footnote_hybrid_global"], nd["footnote_hybrid"])
    coverage = {lib: scraped(lib) for lib in RECIPE}
    result = {"summary": summary, "primary": {"ndcg5_diff": diff, "ci95": [lo, hi], "verdict": verdict},
              "global_minus_per_library": {"ndcg5_diff": g_diff, "ci95": [g_lo, g_hi]}, "pages": coverage,
              "per_query": per_query, "queries": [q["query"] for q in queries]}
    (RUN / "result.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
    log(f"PRIMARY footnote_hybrid - grounded_hybrid nDCG@5 {diff:+.4f} [{lo:+.4f}, {hi:+.4f}] -> {verdict}")
    log(f"global DB - per-library DBs nDCG@5 {g_diff:+.4f} [{g_lo:+.4f}, {g_hi:+.4f}]")
    for name, s in summary.items():
        log(f"{name:24} " + " ".join(f"{k} {v:.3f}" for k, v in s.items()))


if __name__ == "__main__":
    main()
