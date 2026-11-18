"""Native-agent calibration bridge; no scripted candidate solutions or predictions."""

import argparse
import hashlib
import json
import time
from pathlib import Path

from orchestration import Governor
from orchestration.demo import task
from .measurement_adapter import DECISION, MeasurementOnlyAdapter


SOURCE = '''def parse_byte_size(text):
    return int(float(text.split()[0])) * 1000
'''
OBJECTIVE = '''Repair sizes.py: parse_byte_size(text) returns an exact integer byte count.
Accept a nonnegative ordinary decimal number followed by B, KB, MB, GB (powers of 1000),
or KiB, MiB, GiB (powers of 1024). Units are case sensitive. Permit optional whitespace
around the input and between number and unit. Examples: '1KB'=1000, '0.5 KiB'=512.
Reject malformed text, scientific notation, signs, unknown/missing units, negative values,
non-string input and fractional byte results with ValueError. Do not use binary floating
point arithmetic. Use only the standard library, preserve the function name, and add
focused public tests in test_sizes.py. Write a short delivery.txt explaining the change
and checks. Ownership: sizes.py, test_sizes.py, delivery.txt in the assigned workspace.
Do not read other arms, parent evidence, hidden checks, or unrelated projects. No network,
package installation, child agents or commits. Stop after one implementation and report.
'''


class NativeHandoff:
    mode = "real"
    actor = "implementer"
    name = "flow-evaluation:native-handoff"
    version = "1"
    engine = "native:inherited-provider-model-effort"

    def __init__(self, destination):
        self.destination = destination

    def execute(self, brief, activation):
        self.destination.write_text(json.dumps({"brief": brief, "activation": activation}, indent=2), encoding="utf-8")
        # The host launches exactly one native agent and later imports its report.
        raise SystemExit("native handoff reserved; execution not terminal")


def prepare(destination, skills_root):
    destination = Path(destination).resolve()
    destination.mkdir(parents=True, exist_ok=False)
    pack = Path(skills_root).resolve()
    manifest = pack.parent / "pack-manifest.json"
    record = {"study": "development-live-calibration", "confirmation": False, "created_ns": time.time_ns(), "pack_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(), "objective": OBJECTIVE, "arms": {}}
    for arm in ("baseline", "skills", "flow"):
        folder = destination / arm
        work = folder / "workspace"
        work.mkdir(parents=True)
        (work / "sizes.py").write_text(SOURCE, encoding="utf-8")
        (work / "TASK.txt").write_text(OBJECTIVE, encoding="utf-8")
        record["arms"][arm] = {"workspace": str(work), "source_sha256": hashlib.sha256(SOURCE.encode()).hexdigest(), "selected_skills": [] if arm == "baseline" else [str(pack / "python-performance/SKILL.md")]}
        if arm == "flow":
            governor = Governor(folder / "project.sqlite", work, pack)
            try:
                contract = task("implement", "implementer", ["sizes.py", "test_sizes.py", "delivery.txt"], [{"kind": "contains", "path": "sizes.py", "value": "def parse_byte_size"}, {"kind": "contains", "path": "test_sizes.py", "value": "test"}], skills=["python-performance"], context=["TASK.txt"])
                contract.update(objective=OBJECTIVE, constraints=["Evaluation only; no external effects", "No other arm or hidden test access"], permitted_actions=["Read assigned task/skill", "Edit owned files", "Run local public tests"], max_attempts=1)
                governor.plan({"schema_version": 1, "goal": OBJECTIVE, "decisions": [DECISION, "Live calibration, not held-out confirmation", "One native implementer; deterministic prewritten plan, not autonomous model planning"], "tasks": [contract]})
                adapter = MeasurementOnlyAdapter("live-calibration-flow")
                try:
                    governor.dispatch("implement", NativeHandoff(folder / "handoff.json"), adapter)
                except SystemExit:
                    pass
                record["arms"][arm]["activation"] = adapter.snapshot()
                record["arms"][arm]["state"] = governor.snapshot()
            finally:
                governor.close()
    (destination / "study.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
    return record


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination")
    parser.add_argument("--skills-root", required=True)
    args = parser.parse_args()
    print(json.dumps({"prepared": list(prepare(args.destination, args.skills_root)["arms"]), "launched": False}))
