"""Closed-loop MCP concurrency diagnostic. Protocol: DOCS-FOLLOWUPS-2026-11-15.md.

Run only when other benchmarks have stopped:
    bench/.labenv/Scripts/python.exe -I bench/experiments/docs_mcp_load.py <new-output-dir>

Private copied React database, 3 warm-ups and 30 measured calls per client at each of 1, 5, 10, 20, 40 clients.
Two scenarios: shared daemon with MCP proxies; standalone MCP servers sharing the same database.
"""
from __future__ import annotations

import ctypes
import hashlib
import json
import os
import queue
import random
import shutil
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import yaml

OUT = Path(sys.argv[1]).resolve()
SMOKE = len(sys.argv) > 2 and sys.argv[2] == "--smoke"
sys.argv = sys.argv[:1]
sys.path.insert(0, str(Path(__file__).parent))
import docs_vs_grounded as g  # noqa: E402

LEVELS = (1, 2) if SMOKE else (1, 5, 10, 20, 40)
# A/B runs (17 Nov, #152): the binary and the scenarios can be chosen; defaults are the original run's.
BIN = os.environ.get("FOOTNOTE_LOAD_BIN", g.FOOTNOTE)
SCENARIOS = tuple(os.environ.get("FOOTNOTE_LOAD_SCENARIOS", "shared,standalone").split(","))
WARMUPS = 1 if SMOKE else 3
CALLS = 2 if SMOKE else 30
NO_WINDOW = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0


def digest(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


class Client:
    def __init__(self, db, state, shared, logfile):
        cmd = [BIN, "--db", str(db), *(["mcp"] if shared else ["serve", "--mcp"])]
        self.log = open(logfile, "w", encoding="utf-8")
        self.p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log,
                                  text=True, encoding="utf-8", env=dict(os.environ, FOOTNOTE_STATE_DIR=str(state)),
                                  creationflags=NO_WINDOW)
        self.messages = queue.Queue()
        self.seq = 0
        def reader():
            for line in self.p.stdout:
                self.messages.put(line)
            self.messages.put(None)
        self.thread = threading.Thread(target=reader, daemon=True)
        self.thread.start()
        try:
            self.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                        "clientInfo": {"name": "footnote-load-diagnostic", "version": "1"}})
            self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
            self.p.stdin.flush()
        except Exception:
            self.close()
            raise

    def request(self, method, params):
        self.seq += 1
        ident = self.seq
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": ident, "method": method, "params": params}) + "\n")
        self.p.stdin.flush()
        deadline = time.monotonic() + 120
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"MCP timeout on PID {self.p.pid}")
            try:
                line = self.messages.get(timeout=remaining)
            except queue.Empty as e:
                raise TimeoutError(f"MCP timeout on PID {self.p.pid}") from e
            if line is None:
                raise RuntimeError(f"MCP PID {self.p.pid} closed")
            reply = json.loads(line)
            if reply.get("id") == ident:
                if reply.get("error") or reply.get("result", {}).get("isError"):
                    raise RuntimeError(str(reply.get("error") or reply["result"]))
                return reply["result"]

    def search(self, q):
        result = self.request("tools/call", {"name": "footnote_recall", "arguments": {
            "query": q, "mode": "hybrid", "model": g.MODELS, "limit": 5, "per_source": 1}})
        value = json.loads(result["content"][0]["text"])
        if "results" not in value:
            raise RuntimeError("search returned no results field")
        return len(value["results"])

    def close(self):
        if self.p.stdin and not self.p.stdin.closed:
            self.p.stdin.close()
        try:
            self.p.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.p.kill()
            self.p.wait(timeout=10)
        self.thread.join(timeout=2)
        self.p.stdout.close()
        self.log.close()


def process_stats(pids):
    """Windows process working set and cumulative CPU seconds, summed across owned PIDs."""
    if os.name != "nt":
        return None
    from ctypes import wintypes
    class Memory(ctypes.Structure):
        _fields_ = [("cb", wintypes.DWORD), ("faults", wintypes.DWORD), ("peak", ctypes.c_size_t),
                    ("working_set", ctypes.c_size_t), ("a", ctypes.c_size_t), ("b", ctypes.c_size_t),
                    ("c", ctypes.c_size_t), ("d", ctypes.c_size_t), ("e", ctypes.c_size_t), ("f", ctypes.c_size_t)]
    k = ctypes.WinDLL("kernel32", use_last_error=True)
    ps = ctypes.WinDLL("psapi", use_last_error=True)
    k.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    k.OpenProcess.restype = wintypes.HANDLE
    k.CloseHandle.argtypes = [wintypes.HANDLE]
    k.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
    ps.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Memory), wintypes.DWORD]
    total_mem, total_cpu, count = 0, 0, 0
    for pid in pids:
        handle = k.OpenProcess(0x0400 | 0x0010, False, pid)
        if not handle:
            continue
        try:
            mem = Memory()
            mem.cb = ctypes.sizeof(mem)
            created, exited, kernel, user = (wintypes.FILETIME() for _ in range(4))
            if ps.GetProcessMemoryInfo(handle, ctypes.byref(mem), mem.cb) and k.GetProcessTimes(
                    handle, ctypes.byref(created), ctypes.byref(exited), ctypes.byref(kernel), ctypes.byref(user)):
                total_mem += mem.working_set
                total_cpu += sum((x.dwHighDateTime << 32) | x.dwLowDateTime for x in (kernel, user)) / 10_000_000
                count += 1
        finally:
            k.CloseHandle(handle)
    return {"working_set_bytes": total_mem, "cpu_seconds": total_cpu, "sampled_pids": count, "expected_pids": len(pids)}


def percentile(samples, p):
    ordered = sorted(samples)
    return ordered[round((len(ordered) - 1) * p)] if ordered else None


def available_memory():
    if os.name != "nt":
        return None
    class Status(ctypes.Structure):
        _fields_ = [("length", ctypes.c_ulong), ("load", ctypes.c_ulong),
                    ("total_phys", ctypes.c_ulonglong), ("avail_phys", ctypes.c_ulonglong),
                    ("total_page", ctypes.c_ulonglong), ("avail_page", ctypes.c_ulonglong),
                    ("total_virtual", ctypes.c_ulonglong), ("avail_virtual", ctypes.c_ulonglong),
                    ("avail_extended", ctypes.c_ulonglong)]
    status = Status()
    status.length = ctypes.sizeof(status)
    if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
        raise OSError("GlobalMemoryStatusEx failed")
    return status.avail_phys


def run_level(name, n, db, qs):
    folder = OUT / f"{name}-{n}"
    folder.mkdir()
    state = folder / "state"
    state.mkdir()
    daemon = None
    clients = []
    try:
        if name == "shared":
            daemon = subprocess.Popen([BIN, "--db", str(db), "serve", "--mcp", "--daemon", "--no-audit"],
                                      stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                      env=dict(os.environ, FOOTNOTE_STATE_DIR=str(state)), creationflags=NO_WINDOW)
            deadline = time.monotonic() + 20
            while True:
                check = subprocess.run([BIN, "--db", str(db), "memory-recall", "--resident", "--", "probe"],
                                       env=dict(os.environ, FOOTNOTE_STATE_DIR=str(state)), capture_output=True, timeout=10,
                                       creationflags=NO_WINDOW)
                if check.returncode == 0:
                    break
                if daemon.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError("owned shared daemon did not become ready")
                time.sleep(0.1)
        for i in range(n):
            clients.append(Client(db, state, name == "shared", folder / f"client-{i}.log"))
        if name == "shared" and any("no shared daemon" in (folder / f"client-{i}.log").read_text(encoding="utf-8") for i in range(n)):
            raise RuntimeError("one or more clients fell back to independent servers")

        def calls(i, num, measured):
            out = []
            for j in range(num):
                started = time.perf_counter()
                try:
                    count = clients[i].search(qs[(i * CALLS + j) % len(qs)])
                    out.append({"client": i, "request": j, "ms": (time.perf_counter() - started) * 1000,
                                "ok": True, "hits": count})
                except Exception as e:
                    out.append({"client": i, "request": j, "ms": (time.perf_counter() - started) * 1000,
                                "ok": False, "error": str(e)[:400]})
                    if not measured:
                        break
            return out

        # Avoid 40 models loading simultaneously: that transient spike is not the requested steady-state workload.
        for i in range(n):
            free = available_memory()
            if free is not None and free < 5 * 1024**3:
                raise RuntimeError(f"stopped warm-up at {i}/{n}: less than 5 GiB physical memory available")
            warmed = calls(i, WARMUPS, False)
            if any(not row["ok"] for row in warmed):
                raise RuntimeError(f"warmup failed: {warmed}")
        pids = [client.p.pid for client in clients] + ([daemon.pid] if daemon else [])
        initial = process_stats(pids)
        samples = []
        stop = threading.Event()
        def sampler():
            while not stop.wait(0.2):
                sample = process_stats(pids)
                if sample:
                    samples.append(sample)
        sampler_thread = threading.Thread(target=sampler)
        sampler_thread.start()
        start = time.perf_counter()
        try:
            with ThreadPoolExecutor(max_workers=n) as pool:
                groups = list(pool.map(lambda i: calls(i, CALLS, True), range(n)))
        finally:
            elapsed = time.perf_counter() - start
            stop.set()
            sampler_thread.join()
        final = process_stats(pids)
        samples += [s for s in (initial, final) if s]
        rows = [r for group in groups for r in group]
        ok = [r["ms"] for r in rows if r["ok"]]
        bad = [r for r in rows if not r["ok"]]
        report = {"scenario": name, "concurrency": n, "attempts": len(rows), "successes": len(ok), "errors": len(bad),
                  "wall_seconds": elapsed, "successes_per_second": len(ok) / elapsed,
                  "latency_ms": {f"p{int(p * 100)}": percentile(ok, p) for p in (.5, .95, .99)},
                  "working_set_peak_sum_bytes": max((s["working_set_bytes"] for s in samples), default=None),
                  "cpu_seconds": final["cpu_seconds"] - initial["cpu_seconds"] if initial and final else None,
                  "resource_samples_complete": all(s["sampled_pids"] == s["expected_pids"] for s in samples),
                  "pids": pids, "requests_per_client": CALLS, "warmups_per_client": WARMUPS}
        (folder / "requests.json").write_text(json.dumps(rows, indent=2), encoding="utf-8")
        (folder / "resources.json").write_text(json.dumps(samples, indent=2), encoding="utf-8")
        (folder / "result.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report), flush=True)
        return report
    finally:
        for client in clients:
            client.close()
        if daemon:
            daemon.terminate()
            try:
                daemon.wait(timeout=10)
            except subprocess.TimeoutExpired:
                daemon.kill()
                daemon.wait(timeout=10)


def main():
    OUT.mkdir(parents=True, exist_ok=False)
    source = g.REPO / "bench/work/gd/run1/footnote/react.db"
    source_index = Path(str(source) + ".tantivy")
    target = OUT / "react.db"
    shutil.copy2(source, target)
    shutil.copytree(source_index, Path(str(target) + ".tantivy"))
    if digest(source) != digest(target):
        raise RuntimeError("copied database differs from source")
    dataset = g.REPO / "bench/work/gd/dataset.yaml"
    qs = [e["vars"]["query"] for e in yaml.safe_load(dataset.read_text(encoding="utf-8"))["entries"]
          if e["vars"]["library"] == "react"]
    random.Random(20261115).shuffle(qs)
    inputs = {"binary_sha256": digest(Path(BIN)), "database_sha256": digest(target), "dataset_sha256": digest(dataset),
              "models": {m: digest(Path(m) / "manifest.json") for m in g.MODELS}, "levels": LEVELS, "query_count": len(qs),
              "warmups": WARMUPS, "calls_per_client": CALLS, "idle_confirmed": False, "smoke": SMOKE,
              "resource_note": "sum of Windows working sets includes shared pages and proxy processes; CPU in core-seconds"}
    (OUT / "inputs.json").write_text(json.dumps(inputs, indent=2), encoding="utf-8")
    all_results = []
    for scenario in SCENARIOS:
        for n in LEVELS:
            all_results.append(run_level(scenario, n, target, qs))
    (OUT / "summary.json").write_text(json.dumps(all_results, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
