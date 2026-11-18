"""A real helper change in an isolated copy, using a deterministic fixture worker."""

import argparse
import difflib
import hashlib
import json
from pathlib import Path

from .contracts import encoded
from .demo import task
from .fixtures import FixtureTokenLayer, fixture_report
from .governor import Governor
from .repository_tests import PythonUnittest


UNIQUE_KEYS = '''def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON keys are ambiguous")
        result[key] = value
    return result


'''

REGRESSION = '''
class DuplicateKeyTests(unittest.TestCase):
    def test_duplicate_keys_are_rejected_at_every_depth_and_by_cli(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "receipt.json"
            for raw in ('{"correctness_passed":false,"correctness_passed":true}',
                        '{"metadata":{"unit":"ms","unit":"s"}}'):
                with self.subTest(raw=raw):
                    path.write_text(raw, encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, "duplicate JSON keys"):
                        load(path)
                    with contextlib.redirect_stderr(io.StringIO()):
                        self.assertEqual(main([str(path), str(path)]), 2)
            path.write_text('{"metadata":{"unit":"ms"},"unit":"s"}', encoding="utf-8")
            self.assertEqual(load(path)["metadata"]["unit"], "ms")

'''


def fixed_source(baseline):
    old = 'return json.loads(data.decode("utf-8-sig"))'
    if baseline.count(old) != 1 or baseline.count("def load(path):") != 1:
        raise ValueError("frozen baseline does not match the intended patch")
    return baseline.replace("def load(path):", UNIQUE_KEYS + "def load(path):").replace(old, 'return json.loads(data.decode("utf-8-sig"), object_pairs_hook=unique_object)')


class PatchExecutor:
    mode = "fixture"
    actor = "helper-editor"

    def execute(self, brief, activation):
        root = Path(brief["workspace"])
        baseline = (root / "input/compare_receipts.py").read_text(encoding="utf-8")
        (root / "source/compare_receipts.py").write_text(fixed_source(baseline), encoding="utf-8", newline="\n")
        return fixture_report(brief, artifacts=["source/compare_receipts.py"], memories=[{"text": "This helper now rejects duplicate JSON keys, including nested keys; this is local parser behavior, not a production vulnerability claim.", "evidence": ["source/compare_receipts.py"]}])


def run_change(destination, skills_root):
    destination = Path(destination).resolve()
    if destination.exists():
        raise ValueError("choose a new disposable destination")
    root = destination / "workspace"
    for name in ("input", "source", "tests"):
        (root / name).mkdir(parents=True)
    fixtures = Path(__file__).resolve().parents[1] / "evidence/code-change-inputs"
    baseline = (fixtures / "compare_receipts.py").read_text(encoding="utf-8")
    original_tests = (fixtures / "test_compare_receipts.py").read_text(encoding="utf-8")
    tests = original_tests.replace('if __name__ == "__main__":', REGRESSION + 'if __name__ == "__main__":')
    if tests == original_tests:
        raise ValueError("test insertion point missing")
    for name, content in (("input/compare_receipts.py", baseline), ("input/test_compare_receipts.py", original_tests), ("source/compare_receipts.py", baseline), ("tests/test_compare_receipts.py", tests)):
        (root / name).write_text(content, encoding="utf-8", newline="\n")
    verifier = PythonUnittest(source_directory="source", tests_path="tests/test_compare_receipts.py", tests_sha256=hashlib.sha256((root / "tests/test_compare_receipts.py").read_bytes()).hexdigest())
    baseline_request = {"workspace": str(root), "verification_id": "baseline-regression", "verifier": verifier.descriptor}
    before = verifier.run(baseline_request)
    if before["passed"] or before["details"]["exit_code"] != 1 or "AssertionError: ValueError not raised" not in before["details"]["transcript"]:
        raise ValueError("regression did not fail on the baseline for the expected reason")
    governor = Governor(destination / "project.sqlite", root, skills_root, verifiers={verifier.descriptor["id"]: verifier})
    try:
        contract = task("reject-ambiguous-json", "helper-editor", ["source/compare_receipts.py"], [{"kind": "contains", "path": "source/compare_receipts.py", "value": "object_pairs_hook=unique_object"}], skills=["python-performance"], context=["input/compare_receipts.py", "input/test_compare_receipts.py", "tests/test_compare_receipts.py"])
        contract.update(objective="Reject duplicate receipt JSON keys in the existing comparator without orchestration dependencies", constraints=["Use the frozen existing helper and tests", "Do not modify tests or imports outside the owned helper"], verifier=verifier.descriptor)
        governor.plan({"schema_version": 1, "goal": "Deliver a verified change to an existing capability helper in an isolated copy", "decisions": ["Fixture worker and token adapter; real Python suite execution", "Canonical helper is changed only after inspecting this accepted artifact"], "tasks": [contract]})
        governor.dispatch(contract["id"], PatchExecutor(), FixtureTokenLayer())
        if governor.verify(contract["id"]) != "accepted":
            raise ValueError("changed helper failed the registered repository test suite")
        governor.admit_memory(contract["id"], 0, actor="fixture-governor", kind="observation", rationale="Record the tested parser change with its artifact hash")
        fixed = (root / "source/compare_receipts.py").read_text(encoding="utf-8")
        patch = "".join(difflib.unified_diff(baseline.splitlines(keepends=True), fixed.splitlines(keepends=True), fromfile="baseline/compare_receipts.py", tofile="candidate/compare_receipts.py"))
        (destination / "change.diff").write_text(patch, encoding="utf-8")
        result = {"mode": "fixture-worker-real-tests", "real_integration": False, "baseline_sha256": hashlib.sha256((root / "input/compare_receipts.py").read_bytes()).hexdigest(), "candidate_sha256": hashlib.sha256((root / "source/compare_receipts.py").read_bytes()).hexdigest(), "baseline_regression": before, "snapshot": governor.snapshot()}
        (destination / "result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        return result
    finally:
        governor.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination")
    parser.add_argument("--skills-root", required=True)
    args = parser.parse_args()
    result = run_change(args.destination, args.skills_root)
    print(encoded({"complete": result["snapshot"]["complete"], "mode": result["mode"], "real_integration": False, "candidate_sha256": result["candidate_sha256"]}))


if __name__ == "__main__":
    main()
