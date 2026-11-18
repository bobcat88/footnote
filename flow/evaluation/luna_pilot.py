"""Prepare a bounded four-arm development calibration; launches no models."""

import json
import random
import subprocess
import sys
import time
from pathlib import Path

from integration.footnote import ENGINE, PACK, Footnote, prepare, sha, validate_context
from integration.proof import CONTRACT, SEED, setup
from orchestration import Governor
from orchestration.demo import task
from .live_smoke import NativeHandoff
from .measurement_adapter import DECISION, MeasurementOnlyAdapter


OBJECTIVE = """Repair merge_intervals in intervals.py using CONTRACT.txt and DECISION.txt.
Also add keyword-only join_touching=False: True merges touching intervals transitively;
False preserves boundaries. Reject non-boolean join_touching with ValueError.
Use only the standard library. Preserve input and return a list of tuple pairs.
Add focused unittest tests in test_intervals.py and a brief delivery.txt.
Owned outputs: intervals.py, test_intervals.py, delivery.txt only.
One delivery attempt, no parent-requested repair or stronger-model fallback.
No network, installs, commits, child agents, other arms, hidden checks or unrelated files.
Aim for at most 8 model responses and 5 minutes; these are requested limits, not a
provider-enforced token cap. Stop after local verification. Keep the final answer short.
"""


def run(destination, arms=("baseline", "skills", "footnote", "flow"), study="luna-medium-development-v1",
        model="gpt-6-luna", effort="medium", controls=True, folders=None):
    root = Path(destination).resolve()
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    binary = ENGINE
    pack = PACK
    arms = list(arms)
    random.Random(6117).shuffle(arms)
    record = {"study": study, "model": model,
              "options": {"reasoningEffort": effort}, "created_ns": time.time_ns(),
              "order": arms, "attempts_per_arm": 1, "maximum_runs": len(arms),
              "requested_model_response_limit": 8, "requested_minutes": 5,
              "hard_token_limit": None, "confirmation": False,
              "objective": OBJECTIVE, "arms": {},
              "limitations": ["One synthetic task, one repetition per arm",
                              "Current Flow prototype, not completed production integration",
                              "Common parent preparation cost recorded separately",
                              "No price estimate or dollar billing claim"]}
    for arm in arms:
        started = time.perf_counter_ns()
        folder = Path(folders[arm]) if folders else root / arm
        setup(folder, binary)
        work = folder / "workspace"
        for name, content in (("TASK.txt", OBJECTIVE), ("CONTRACT.txt", CONTRACT), ("DECISION.txt", SEED)):
            (work / name).write_text(content, encoding="utf-8", newline="\n")
        # Equal initial knowledge; only its delivery differs between treatments.
        item = {"workspace": str(work), "initial_source_sha256": sha(work / "intervals.py"),
                "initial_knowledge": {name: sha(work / name) for name in ("TASK.txt", "CONTRACT.txt", "DECISION.txt")},
                "selected_skill": None if arm in ("baseline", "footnote_only") else str(pack / "python-performance/SKILL.md")}
        if arm in ("footnote_only", "footnote", "flow"):
            client = Footnote(folder / "bin/footnote.exe", folder)
            context = prepare(client, project="flow-interval-proof", workspace=work,
                              symbol="merge_intervals", query="merge_intervals",
                              documents=[folder / "docs/interval-contract.txt"])
            validate_context(context)
            (work / "footnote-context.json").write_text(json.dumps(context, indent=2, ensure_ascii=False), encoding="utf-8", newline="\n")
            item["context_bytes"] = context["context_bytes"]
            item["retrieval_calls"] = client.calls
        if arm == "flow":
            governor = Governor(folder / "project.sqlite", work, pack)
            try:
                contract = task("implement", "implementer", ["intervals.py", "test_intervals.py", "delivery.txt"],
                                [{"kind": "contains", "path": "intervals.py", "value": "def merge_intervals"}],
                                skills=["python-performance"], context=["TASK.txt", "CONTRACT.txt", "DECISION.txt", "footnote-context.json"])
                contract.update(objective=OBJECTIVE, max_attempts=1,
                                constraints=["Isolated evaluation", "External acceptance must pass before host acceptance"],
                                permitted_actions=["Read assigned inputs", "Edit owned files", "Run local tests"])
                governor.plan({"schema_version": 1, "goal": OBJECTIVE, "decisions": [DECISION], "tasks": [contract]})
                adapter = MeasurementOnlyAdapter(study.removesuffix("-v1") + "-flow")
                executor = NativeHandoff(folder / "handoff.json")
                executor.engine = "t3:codex:" + model + ":" + effort
                try:
                    governor.dispatch("implement", executor, adapter)
                except SystemExit:
                    pass
                item["measurement_activation"] = adapter.snapshot()
                item["governor_snapshot"] = governor.snapshot()
            finally:
                governor.close()
        else:
            adapter = MeasurementOnlyAdapter(study.removesuffix("-v1") + "-" + arm)
            brief = {"mode": "real", "task_id": "implement", "attempt_id": arm + "-1",
                     "project_decisions": [DECISION], "runtime": {"adapter": "flow-evaluation:t3"},
                     "objective": OBJECTIVE, "workspace": str(work)}
            adapter.activate(brief)
            item["measurement_activation"] = adapter.snapshot()
        item["prepare_ms"] = (time.perf_counter_ns() - started) / 1_000_000
        record["arms"][arm] = item
    hashes = [item["initial_source_sha256"] for item in record["arms"].values()]
    assert len(set(hashes)) == 1
    assert len({json.dumps(item["initial_knowledge"], sort_keys=True) for item in record["arms"].values()}) == 1
    if not controls:
        record["controls"] = None
        (root / "study.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
        return record
    # The independent suite rejects the broken start and accepts a retained known-good control.
    controls = {}
    for label, work, expected in (("broken", root / "baseline/workspace", 1),
                                  ("known_valid", project / "evidence/integrated-loop-v1", 0)):
        result = subprocess.run(["rtk", "proxy", sys.executable, "-m", "integration.check_intervals", str(work), "--followup"], capture_output=True, text=True, timeout=30)
        (root / ("control-" + label + ".txt")).write_text(result.stdout + result.stderr, encoding="utf-8")
        if result.returncode != expected:
            raise ValueError("independent checker control failed: " + label)
        controls[label] = {"exit_code": result.returncode, "expected": expected}
    record["checker_sha256"] = sha(project / "integration/check_intervals.py")
    record["controls"] = controls
    (root / "study.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
    print(json.dumps({"prepared": arms, "maximum_runs": len(arms), "controls": controls, "launched": False}))
    return record


if __name__ == "__main__":
    run(sys.argv[1])
