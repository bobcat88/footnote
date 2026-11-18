import hashlib
import json
import subprocess
import sys
import unittest

from orchestration import Governor
from orchestration.contracts import encoded
from orchestration.fixtures import FixtureTokenLayer
from orchestration.repository_tests import PythonUnittest
from orchestration.test_governor import Executor, plan, task
from orchestration import test_governor as helpers


class Verifier:
    descriptor = {"id": "trusted-suite", "version": "1", "sha256": "a" * 64}

    def __init__(self, action=None):
        self.calls = 0
        self.action = action
        self.request = self.result = None

    def run(self, request):
        self.calls += 1
        self.request = request
        self.result = {"verification_id": request["verification_id"], "request_sha256": hashlib.sha256(encoded(request).encode()).hexdigest(), "passed": True, "details": {"fixture": True}}
        if self.action:
            self.action(self)
        return self.result


class NamedVerifierTests(unittest.TestCase):
    setUp = helpers.GovernorTests.setUp
    open = helpers.GovernorTests.open

    def prepare(self, verifier=None, **changes):
        self.verifier = verifier or Verifier()
        self.g.verifiers["trusted-suite"] = self.verifier
        self.g.plan(plan(task(verifier=self.verifier.descriptor, **changes)))
        self.executor = Executor()
        self.attempt = self.g.dispatch("first", self.executor, FixtureTokenLayer())

    def test_registered_suite_and_receipt_survive_reopen(self):
        self.prepare()
        self.assertEqual(self.g.verify("first"), "accepted")
        self.assertEqual(self.verifier.calls, 1)
        self.g.close()
        self.g = self.open()
        self.assertTrue(self.g.snapshot()["complete"])
        with self.assertRaises(ValueError):
            self.g.verify("first")

    def test_wrong_or_missing_host_registration_consumes_no_verifier_run(self):
        with self.assertRaises(ValueError):
            self.g.plan(plan(task(verifier=Verifier.descriptor)))
        self.prepare()
        self.g.verifiers.clear()
        with self.assertRaises(ValueError):
            self.g.verify("first")
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "reported")
        self.assertEqual(self.verifier.calls, 0)

    def test_callback_runs_outside_transaction_and_duplicate_claim_is_refused(self):
        def action(verifier):
            other = Governor(self.database, self.workspace, self.skills, verifiers=self.g.verifiers)
            try:
                self.assertEqual(other.snapshot()["tasks"][0]["state"], "verifying")
                with other.store.write():
                    other.store.event(None, "concurrent_host_event", {})
                with self.assertRaises(ValueError):
                    other.verify("first")
            finally:
                other.close()
        self.prepare(Verifier(action))
        self.assertEqual(self.g.verify("first"), "accepted")

    def test_failed_suite_consumes_bounded_attempt(self):
        self.prepare(Verifier(lambda verifier: verifier.result.update(passed=False)), max_attempts=1)
        self.assertEqual(self.g.verify("first"), "escalated")

    def test_artifact_or_context_mutation_during_tests_cannot_be_accepted(self):
        (self.workspace / "input.txt").write_text("baseline")
        self.prepare(Verifier(lambda verifier: (self.workspace / "input.txt").write_text("changed")), context_files=["input.txt"])
        self.assertEqual(self.g.verify("first"), "revision")
        self.assertFalse(self.g.snapshot()["complete"])

    def test_interrupted_verifier_is_reconciled_without_rerun_or_worker_resubmit(self):
        def interrupt(verifier):
            raise SystemExit("test process completed; host interrupted")
        self.prepare(Verifier(interrupt))
        with self.assertRaises(SystemExit):
            self.g.verify("first")
        self.g.close()
        self.g = self.open()
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "verification_uncertain")
        with self.assertRaises(ValueError):
            self.g.submit_report(self.attempt, self.executor.last_report, actor="worker")
        with self.assertRaises(ValueError):
            self.g.dispatch("first", self.executor, FixtureTokenLayer())
        with self.assertRaises(ValueError):
            self.g.verify("first")
        self.assertEqual(self.g.reconcile_verification(self.verifier.request["verification_id"], self.verifier.result, actor="trusted-host", rationale="Observed terminal suite and recovered its bound receipt"), "accepted")
        self.assertEqual(self.verifier.calls, 1)
        with self.assertRaises(ValueError):
            self.g.reconcile_verification(self.verifier.request["verification_id"], self.verifier.result, actor="trusted-host", rationale="duplicate")

    def test_stale_verifier_receipt_remains_uncertain(self):
        self.prepare(Verifier(lambda verifier: verifier.result.update(request_sha256="stale")))
        with self.assertRaises(ValueError):
            self.g.verify("first")
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "verification_uncertain")

    def test_schema_one_upgrade_preserves_existing_state(self):
        self.g.plan(plan())
        with self.g.store.write():
            self.g.db.execute("DROP TABLE verifications")
            self.g.db.execute("PRAGMA user_version=1")
        self.g.close()
        self.g = self.open()
        self.assertEqual(self.g.db.execute("PRAGMA user_version").fetchone()[0], 2)
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "pending")

    def test_actual_host_exit_keeps_verifier_reserved_until_terminal_receipt_reconciliation(self):
        self.prepare()
        self.g.close()
        receipt = self.root / "terminal-receipt.json"
        script = '''import json, os, sys
from pathlib import Path
from orchestration import Governor
from orchestration.test_verifiers import Verifier
def exit_after_result(verifier):
    Path(sys.argv[4]).write_text(json.dumps({"request": verifier.request, "result": verifier.result}))
    os._exit(23)
verifier = Verifier(exit_after_result)
governor = Governor(sys.argv[1], sys.argv[2], sys.argv[3], verifiers={"trusted-suite": verifier})
governor.verify("first")
'''
        completed = subprocess.run(["rtk", "proxy", sys.executable, "-c", script, str(self.database), str(self.workspace), str(self.skills), str(receipt)], capture_output=True, timeout=20)
        self.assertEqual(completed.returncode, 23, completed.stderr)
        self.g = self.open()
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "verifying")
        recovered = json.loads(receipt.read_text())
        ident = recovered["request"]["verification_id"]
        self.g.mark_verification_uncertain(ident, actor="test-host", reason="Observed terminal process exit 23, receipt recovered")
        self.assertEqual(self.g.reconcile_verification(ident, recovered["result"], actor="test-host", rationale="Same verification attempt, no second test run"), "accepted")

    def test_named_verification_then_independent_review_survives_reopen(self):
        self.prepare(review_required=True)
        self.assertEqual(self.g.verify("first"), "verified")
        (self.workspace / "review.txt").write_text("bounded independent fixture review")
        self.g.review("first", reviewer="critic", approve=True, evidence_path="review.txt", rationale="Checked artifact")
        self.g.close()
        self.g = self.open()
        self.assertTrue(self.g.snapshot()["complete"])

    def test_zero_exit_during_import_cannot_claim_a_completed_suite(self):
        (self.workspace / "source").mkdir()
        tests = self.workspace / "suite.py"
        cases = [("raise SystemExit(0)", False), ("import os; os._exit(0)", False), ("import unittest", False), ("import unittest\nclass T(unittest.TestCase):\n def test_pass(self): self.assertEqual(1, 1)", True), ("import unittest\nclass T(unittest.TestCase):\n def test_fail(self): self.assertEqual(1, 2)", False)]
        for source, expected in cases:
            with self.subTest(source=source):
                tests.write_text(source, encoding="utf-8")
                verifier = PythonUnittest(source_directory="source", tests_path="suite.py", tests_sha256=hashlib.sha256(tests.read_bytes()).hexdigest())
                result = verifier.run({"verification_id": "suite-check", "workspace": str(self.workspace)})
                self.assertIs(result["passed"], expected)


if __name__ == "__main__":
    unittest.main()
