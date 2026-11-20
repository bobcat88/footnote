"""GPU speed of llama.cpp runtimes on the Radeon RX 6800: b11901 Vulkan (used so far) against b12421 Vulkan and b12421 ROCm/HIP.
Backlog P3 (llama.cpp speed grid, runtime versions). Reported, not judged; valid only on an idle machine.

    bench/.labenv/Scripts/python.exe -I bench/experiments/llamacpp_runtimes.py

Model: Jina v5 nano Q8_0 (the opt-in French/mixed profile), lab settings (last-token pooling, Query:/Document: prefixes, 8 slots,
8192 context on the GPU). Texts: this repository's docs/*.md cut into passages of at most 1,200 characters (fixed, local).
Per runtime: 200 single queries one at a time (p50, p95 ms), then the passages in requests of 16 (passages per second), and the
cosine of each runtime's vectors with b11901's on 100 passages (vectors must agree before speed means anything).
"""
from __future__ import annotations

import json
import statistics
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[2]
OUT = REPO / "bench/work/overnight/llamacpp-runtimes.json"
GGUF = REPO / "artifacts/gguf/v5-nano-retrieval-Q8_0.gguf"
RUNTIMES = {
    "b11901-vulkan": Path(os.environ.get("FOOTNOTE_LLAMA_SERVER", "llama-server")),
    "b12421-vulkan": REPO / "bench/work/runtimes/b12421-vulkan/llama-server.exe",
    "b12421-rocm": REPO / "bench/work/runtimes/b12421-rocm/llama-server.exe",
}
PORT = 8092


def texts() -> tuple[list[str], list[str]]:
    passages, queries = [], []
    for f in sorted((REPO / "docs").rglob("*.md")):
        body = f.read_text(encoding="utf-8", errors="ignore")
        for part in body.split("\n## "):
            part = part.strip()
            if len(part) > 200:
                passages.append(part[:1200])
                queries.append(part.splitlines()[0].strip("# ")[:120])
    return passages[:1200], [q for q in queries if q][:200]


def post(inputs: list[str]) -> list[list[float]]:
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}/v1/embeddings", data=json.dumps({"input": inputs}).encode(),
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return [d["embedding"] for d in json.load(r)["data"]]


def measure(name: str, exe: Path, passages: list[str], queries: list[str]) -> dict:
    if not exe.exists():
        return {"runtime": name, "error": "not installed"}
    cmd = [str(exe), "-m", str(GGUF), "--embedding", "--pooling", "last", "-ngl", "99", "-np", "8", "-c", "8192", "-b", "8192",
           "-ub", "8192", "--host", "127.0.0.1", "--port", str(PORT), "--no-webui"]
    log = open(OUT.with_name(f"server-{name}.log"), "w")
    proc = subprocess.Popen(cmd, cwd=exe.parent, stdout=log, stderr=log)
    try:
        for _ in range(180):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2)
                break
            except Exception:
                if proc.poll() is not None:
                    return {"runtime": name, "error": f"server exited {proc.returncode}; see server-{name}.log"}
                time.sleep(1)
        post(["Query: warm up"])
        lat = []
        for q in queries:
            t = time.perf_counter()
            post([f"Query: {q}"])
            lat.append((time.perf_counter() - t) * 1000)
        docs = [f"Document: {p}" for p in passages]
        t = time.perf_counter()
        vecs = []
        for i in range(0, len(docs), 16):
            vecs += post(docs[i:i + 16])
        secs = time.perf_counter() - t
        lat.sort()
        return {"runtime": name, "query_p50_ms": statistics.median(lat), "query_p95_ms": lat[int(0.95 * len(lat)) - 1],
                "passages": len(docs), "passages_per_s": len(docs) / secs, "vectors": vecs[:100]}
    finally:
        proc.terminate()  # by PID
        proc.wait(timeout=30)


def main() -> None:
    passages, queries = texts()
    print(f"{len(passages)} passages, {len(queries)} queries", flush=True)
    results = [measure(n, e, passages, queries) for n, e in RUNTIMES.items()]
    base = next((r["vectors"] for r in results if r["runtime"] == "b11901-vulkan" and "vectors" in r), None)
    for r in results:
        if "vectors" in r and base:
            a, b = np.array(r["vectors"]), np.array(base)
            cos = (a * b).sum(1) / (np.linalg.norm(a, axis=1) * np.linalg.norm(b, axis=1))
            r["min_cosine_vs_b11901"] = float(cos.min())
        r.pop("vectors", None)
        print(json.dumps(r), flush=True)
    OUT.write_text(json.dumps(results, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
