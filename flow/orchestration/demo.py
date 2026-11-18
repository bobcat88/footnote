"""Disposable project fixture: change, recover, verify, review, remember, hand over."""

import argparse
import json
import sqlite3
import sys
from pathlib import Path

from .fixtures import FixtureTokenLayer
from .governor import Governor


UNKNOWN_USAGE = {"source": "unknown", "input_tokens": None, "cached_input_tokens": None, "output_tokens": None}


def task(ident, owner, owns, acceptance, *, dependencies=None, skills=None, context=None, review=False):
    return {"id": ident, "objective": f"Complete the {ident} step of the disposable RingMaster latency-summary change", "owner": owner, "owns": owns, "acceptance": acceptance, "skills": skills or [], "context_files": context or [], "constraints": ["Fixture only; no real project changes or model calls", "Preserve summary labels; these values are not raw timing samples"], "permitted_actions": ["Read declared fixture inputs", "Write declared owned fixture artifacts"], "stop_condition": "Return a report after the artifact is written; do not delegate", "dependencies": dependencies or [], "max_attempts": 2, "review_required": review}


class NormalizeExecutor:
    mode = "fixture"
    actor = "normalizer"

    def __init__(self, interrupt=True):
        self.interrupt = interrupt
        self.report = None
        self.calls = 0

    def execute(self, brief, activation):
        self.calls += 1
        root = Path(brief["workspace"])
        source = json.loads((root / "input/legacy-summary.json").read_text())
        result = {"unit": "ms", "summary": {name.removesuffix("_us") + "_ms": value / 1000 for name, value in source["summary"].items()}}
        (root / "output").mkdir(exist_ok=True)
        (root / "output/summary.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        self.report = {"task_id": brief["task_id"], "attempt_id": brief["attempt_id"], "summary": "Converted labelled microsecond summaries to milliseconds without treating them as samples", "artifacts": ["output/summary.json"], "unresolved": [], "usage": dict(UNKNOWN_USAGE), "candidate_memories": [{"text": "The disposable RingMaster fixture summary was normalized from microseconds to milliseconds; these are summary statistics, not raw samples.", "evidence": ["output/summary.json"]}], "runtime_turn_id": "fixture-normalization"}
        if self.interrupt:
            raise SystemExit("deliberate fixture interruption after artifact write")
        return self.report


class HandoverExecutor:
    mode = "fixture"
    actor = "writer"

    def execute(self, brief, activation):
        root = Path(brief["workspace"])
        summary = json.loads((root / "output/summary.json").read_text())
        content = "Fixture handover\nUnit: ms\n" + json.dumps(summary, sort_keys=True) + "\nNo real Footnote integration or product benchmark claim.\n"
        (root / "handover.txt").write_text(content, encoding="utf-8")
        return {"task_id": brief["task_id"], "attempt_id": brief["attempt_id"], "summary": "Prepared the bounded fixture handover", "artifacts": ["handover.txt"], "unresolved": [], "usage": dict(UNKNOWN_USAGE), "candidate_memories": [], "runtime_turn_id": "fixture-handover"}


def run_demo(destination, skills_root):
    destination = Path(destination).resolve()
    if destination.exists():
        raise ValueError("demo destination already exists; choose a new disposable directory")
    workspace = destination / "workspace"
    (workspace / "input").mkdir(parents=True)
    # Values are a labelled fixture based on the documented RingMaster event-log table.
    # They are intentionally kept as summary statistics, never benchmark observations.
    (workspace / "input/legacy-summary.json").write_text(json.dumps({"unit": "us", "summary": {"median_us": 25, "p95_us": 61, "max_us": 10843}}), encoding="utf-8")
    database = destination / "project.sqlite"
    governor = Governor(database, workspace, skills_root)
    normalizer = NormalizeExecutor()
    try:
        expected = {"unit": "ms", "summary": {"median_ms": 0.025, "p95_ms": 0.061, "max_ms": 10.843}}
        plan = {"schema_version": 1, "goal": "Normalize a disposable existing-project latency-summary fixture and deliver its checked handover", "decisions": ["Preserve summary semantics; no benchmark superiority claim", "Use fixture runtime/token-layer adapters; real integration is deferred"], "tasks": [task("normalize", "normalizer", ["output/summary.json"], [{"kind": "json_equals", "path": "output/summary.json", "value": expected}], skills=["rust-performance"], context=["input/legacy-summary.json"]), task("handover", "writer", ["handover.txt"], [{"kind": "contains", "path": "handover.txt", "value": "Fixture handover"}, {"kind": "contains", "path": "handover.txt", "value": "Unit: ms"}], dependencies=["normalize"], context=["output/summary.json"], review=True)]}
        governor.plan(plan)
        try:
            governor.dispatch("normalize", normalizer, FixtureTokenLayer())
        except SystemExit:
            pass
        interrupted = governor.snapshot()
    finally:
        governor.close()
    governor = Governor(database, workspace, skills_root)
    try:
        attempt_id = interrupted["tasks"][0]["latest_attempt"]
        governor.mark_uncertain(attempt_id, actor="fixture-operator", reason="Observed fixture callback termination after output write; reconcile its report without another dispatch")
        governor.submit_report(attempt_id, normalizer.report, actor="normalizer")
        if governor.verify("normalize") != "accepted":
            raise ValueError("normalization fixture failed verification")
        governor.dispatch("handover", HandoverExecutor(), FixtureTokenLayer())
        if governor.verify("handover") != "verified":
            raise ValueError("handover fixture did not reach review")
        (workspace / "review.txt").write_text("Fixture critic confirmed summary labels, ms units and explicit integration limitations.", encoding="utf-8")
        governor.review("handover", reviewer="fixture-critic", approve=True, evidence_path="review.txt", rationale="Fixture review checks expected units and limitation text")
        memory_id = governor.admit_memory("normalize", 0, actor="fixture-governor", kind="observation", rationale="Keep the scoped fixture conversion and provenance, not a general product claim")
        result = {"mode": "fixture", "real_integration": False, "normalizer_calls": normalizer.calls, "memory_id": memory_id, "snapshot": governor.snapshot()}
        if not result["snapshot"]["complete"] or normalizer.calls != 1:
            raise ValueError("fixture did not complete without repeating the normalization effect")
        (destination / "result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        return result
    finally:
        governor.close()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", help="new disposable directory; never an existing project")
    parser.add_argument("--skills-root", required=True, help="exported capability pack's skills directory")
    args = parser.parse_args(argv)
    try:
        result = run_demo(args.destination, args.skills_root)
    except (OSError, ValueError, sqlite3.Error) as error:
        print(f"Fixture failed: {error}", file=sys.stderr)
        return 2
    print(json.dumps({"mode": "fixture", "complete": result["snapshot"]["complete"], "normalizer_calls": result["normalizer_calls"], "real_integration": False}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


