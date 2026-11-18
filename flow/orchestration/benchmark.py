"""Measure local fixture coordination overhead, never model or product performance."""

import argparse
import hashlib
import json
import os
import platform
import sqlite3
import statistics
import tempfile
import time
from pathlib import Path

from .demo import run_demo


def benchmark(skills_root, runs=10, warmups=2):
    if type(runs) is not int or not 1 <= runs <= 100 or type(warmups) is not int or not 0 <= warmups <= 10:
        raise ValueError("runs must be 1..100 and warmups 0..10")
    samples = []
    events = []
    sizes = []
    with tempfile.TemporaryDirectory(prefix="footnote-flow-benchmark-") as temp:
        root = Path(temp).resolve()
        if not root.is_relative_to(Path(tempfile.gettempdir()).resolve()):
            raise ValueError("benchmark temporary directory is outside the expected root")
        for index in range(warmups + runs):
            destination = root / f"run-{index}"
            started = time.perf_counter()
            result = run_demo(destination, skills_root)
            elapsed = (time.perf_counter() - started) * 1000
            if not result["snapshot"]["complete"] or result["normalizer_calls"] != 1:
                raise ValueError("fixture correctness failed; timing is inadmissible")
            if index >= warmups:
                samples.append(elapsed)
                events.append(len(result["snapshot"]["events"]))
                sizes.append((destination / "project.sqlite").stat().st_size)
    environment = {"platform": platform.platform(), "python": platform.python_version(), "sqlite": sqlite3.sqlite_version, "machine": platform.machine(), "cpu": platform.processor(), "logical_cpus": os.cpu_count(), "host_fingerprint": hashlib.sha256(platform.node().encode()).hexdigest()}
    sources = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(Path(__file__).parent.glob("*.py")) if not path.name.startswith("test_")}
    pack_hash = hashlib.sha256((Path(skills_root).resolve().parent / "pack-manifest.json").read_bytes()).hexdigest()
    revision = hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest()
    comparison = {"schema_version": 1, "workload_id": "ringmaster-summary-fixture:" + sources["demo.py"] + ":pack:" + pack_hash, "environment_id": hashlib.sha256(json.dumps(environment, sort_keys=True).encode()).hexdigest(), "metric": "fixture_workflow_elapsed", "unit": "ms", "lower_is_better": True, "revision": revision, "correctness_passed": True, "samples": samples}
    return {"schema_version": 1, "scope": "single-process warm-import fixture coordination plus SQLite/artifact I/O; excludes interpreter startup, real models, runtime sandbox, Footnote and external services", "environment": environment, "sources": sources, "capability_pack_sha256": pack_hash, "comparison_receipt": comparison, "warmups": warmups, "samples_ms": samples, "median_ms": statistics.median(samples), "min_ms": min(samples), "max_ms": max(samples), "event_counts": events, "database_bytes": sizes, "correctness_passed": True, "real_integration": False, "tail_latency": "not established by this small pilot", "budget_verdict": "measurement-only; no universal performance budget", "limitations": ["background machine load was not controlled", "storage/power-loss behavior was not independently certified", "host fingerprint is a local descriptor, not hardware attestation"]}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skills-root", required=True)
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--warmups", type=int, default=2)
    parser.add_argument("--output", required=True, help="new JSON receipt path; existing files are not overwritten")
    args = parser.parse_args(argv)
    output = Path(args.output)
    if output.exists():
        raise ValueError("benchmark output already exists")
    report = benchmark(args.skills_root, args.runs, args.warmups)
    with output.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"runs": args.runs, "median_ms": report["median_ms"], "correctness_passed": True, "scope": "local fixture only"}))


if __name__ == "__main__":
    main()
