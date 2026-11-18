import copy
from contextlib import closing
import hashlib
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path

from orchestration import Governor
from orchestration.fixtures import FixtureTokenLayer, fixture_report


def task(ident="first", **updates):
    value = {"id": ident, "objective": "Produce the required fixture artifact", "owner": "worker", "owns": [ident + ".txt"], "acceptance": [{"kind": "contains", "path": ident + ".txt", "value": "ready"}], "skills": ["example"], "context_files": [], "constraints": ["No external effects"], "permitted_actions": ["write the owned fixture artifact"], "stop_condition": "Return one report or a visible failure", "dependencies": [], "max_attempts": 2, "review_required": False}
    value.update(updates)
    return value


def plan(*tasks):
    return {"schema_version": 1, "goal": "Complete the bounded fixture project", "decisions": ["Fixture execution only"], "tasks": list(tasks) or [task()]}


class Executor:
    mode = "fixture"
    actor = "worker"

    def __init__(self, output="ready", *, crash=False, report_change=None):
        self.output = output
        self.crash = crash
        self.report_change = report_change
        self.calls = 0
        self.last_report = None

    def execute(self, brief, activation):
        self.calls += 1
        destination = Path(brief["workspace"]) / brief["contract"]["owns"][0]
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(self.output, encoding="utf-8")
        self.last_report = fixture_report(brief, artifacts=[brief["contract"]["owns"][0]], memories=[{"text": "The fixture artifact was checked", "evidence": [brief["contract"]["owns"][0]]}])
        if self.report_change:
            self.report_change(self.last_report)
        if self.crash:
            raise SystemExit("fixture interrupted after producing artifact")
        return self.last_report


class GovernorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.workspace = self.root / "workspace"
        self.workspace.mkdir()
        self.skills = self.root / "pack/skills"
        skill = self.skills / "example/SKILL.md"
        skill.parent.mkdir(parents=True)
        skill.write_text("# Example fixture skill\n", encoding="utf-8")
        manifest = {"schema_version": 1, "skills": ["example"], "files": {"skills/example/SKILL.md": hashlib.sha256(skill.read_bytes()).hexdigest()}}
        (self.skills.parent / "pack-manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        self.database = self.root / "project.sqlite"
        self.g = self.open()

    def open(self):
        governor = Governor(self.database, self.workspace, self.skills)
        self.addCleanup(governor.close)
        return governor

    def finish(self, ident="first", executor=None):
        attempt = self.g.dispatch(ident, executor or Executor(), FixtureTokenLayer())
        state = self.g.verify(ident)
        return attempt, state

    def test_complete_project_and_memory_are_not_self_approved(self):
        self.g.plan(plan())
        executor = Executor()
        attempt = self.g.dispatch("first", executor, FixtureTokenLayer())
        self.assertFalse(self.g.snapshot()["complete"])
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "reported")
        with self.assertRaises(ValueError):
            self.g.admit_memory("first", 0, actor="governor", kind="fact", rationale="not verified yet")
        self.assertEqual(self.g.verify("first"), "accepted")
        with self.assertRaises(ValueError):
            self.g.admit_memory("first", 0, actor="worker", kind="fact", rationale="self approval")
        ident = self.g.admit_memory("first", 0, actor="governor", kind="observation", rationale="Useful bounded fixture result")
        self.assertTrue(ident)
        self.assertTrue(self.g.snapshot()["complete"])
        with self.assertRaises(ValueError):
            self.g.admit_memory("first", 0, actor="governor", kind="fact", rationale="duplicate")
        with self.assertRaises(ValueError):
            self.g.submit_report(attempt, executor.last_report, actor="worker")
        with self.assertRaises(ValueError):
            self.g.verify("first")

    def test_missing_or_fixture_token_layer_cannot_authorize_real_dispatch(self):
        self.g.plan(plan())
        executor = Executor()
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor)
        executor.mode = "real"
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        self.assertEqual(executor.calls, 0)
        self.assertEqual(self.g.snapshot()["tasks"][0]["attempts"], 0)

    def test_activation_binds_identity_and_refusal_has_bounded_attempts(self):
        self.g.plan(plan(task(max_attempts=1)))
        class StaleActivation(FixtureTokenLayer):
            def activate(self, brief):
                receipt = super().activate(brief)
                receipt["brief_sha256"] = "stale"
                return receipt
        executor = Executor()
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, StaleActivation())
        self.assertEqual(executor.calls, 0)
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "escalated")
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, FixtureTokenLayer())

    def test_reports_cannot_grant_acceptance_or_fake_runtime_usage(self):
        self.g.plan(plan())
        executor = Executor(report_change=lambda report: report.update(accepted=True))
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "uncertain")
        self.assertFalse(self.g.snapshot()["complete"])

    def test_dependencies_and_transitive_stale_evidence_block_progress(self):
        self.g.plan(plan(task(), task("second", dependencies=["first"]), task("third", dependencies=["second"])))
        with self.assertRaises(ValueError):
            self.g.dispatch("second", Executor(), FixtureTokenLayer())
        self.finish("first")
        self.finish("second")
        (self.workspace / "first.txt").write_text("changed", encoding="utf-8")
        with self.assertRaises(ValueError):
            self.g.dispatch("third", Executor(), FixtureTokenLayer())
        with self.assertRaises(ValueError):
            self.g.admit_memory("second", 0, actor="governor", kind="fact", rationale="stale upstream")
        self.assertFalse(self.g.snapshot()["tasks"][1]["evidence_fresh"])

    def test_verifier_checks_actual_bound_artifact_and_attempt_limit(self):
        self.g.plan(plan())
        self.assertEqual(self.finish(executor=Executor("wrong"))[1], "revision")
        self.assertEqual(self.finish(executor=Executor("wrong"))[1], "escalated")
        with self.assertRaises(ValueError):
            self.g.dispatch("first", Executor(), FixtureTokenLayer())

    def test_changed_or_unresolved_report_is_not_accepted(self):
        self.g.plan(plan())
        self.g.dispatch("first", Executor(), FixtureTokenLayer())
        (self.workspace / "first.txt").write_text("ready but changed", encoding="utf-8")
        self.assertEqual(self.g.verify("first"), "revision")
        executor = Executor(report_change=lambda report: report.update(unresolved=["required check missing"]))
        self.g.dispatch("first", executor, FixtureTokenLayer())
        self.assertEqual(self.g.verify("first"), "escalated")

    def test_report_paths_and_actor_are_checked(self):
        self.g.plan(plan())
        executor = Executor(report_change=lambda report: report.update(artifacts=["../outside.txt"]))
        (self.root / "outside.txt").write_text("ready")
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        attempt = self.g.snapshot()["tasks"][0]["latest_attempt"]
        report = copy.deepcopy(executor.last_report)
        report["artifacts"] = ["first.txt"]
        report["candidate_memories"] = []
        with self.assertRaises(ValueError):
            self.g.submit_report(attempt, report, actor="other-worker")
        self.g.submit_report(attempt, report, actor="worker")
        self.assertEqual(self.g.verify("first"), "accepted")

    def test_review_is_distinct_and_bound_to_fresh_evidence(self):
        self.g.plan(plan(task(review_required=True)))
        self.assertEqual(self.finish()[1], "verified")
        review = self.workspace / "review.txt"
        review.write_text("Fixture review checks the expected ready artifact")
        with self.assertRaises(ValueError):
            self.g.review("first", reviewer="worker", approve=True, evidence_path="review.txt", rationale="self-review")
        self.g.review("first", reviewer="critic", approve=True, evidence_path="review.txt", rationale="Expected artifact verified")
        self.assertTrue(self.g.snapshot()["complete"])
        review.write_text("changed")
        self.assertFalse(self.g.snapshot()["complete"])

    def test_plan_validation_is_atomic_and_contract_is_frozen(self):
        invalid = [plan(task(), task()), plan(task(dependencies=["missing"])), plan(task(dependencies=["first"])), plan(task(acceptance=[])), plan(task(owns=["../escape"])), plan(task(acceptance=[{"kind": "command", "path": "first.txt", "value": "dangerous"}]))]
        for candidate in invalid:
            with self.subTest(candidate=candidate), self.assertRaises(ValueError):
                self.g.plan(candidate)
            self.assertEqual(self.g.snapshot()["tasks"], [])
        self.g.plan(plan())
        with self.assertRaises(ValueError):
            self.g.plan(plan(task("replacement")))

    def test_capability_manifest_and_selected_skill_are_bound(self):
        with self.assertRaises(ValueError):
            self.g.plan(plan(task(skills=["missing"])))
        self.g.plan(plan())
        (self.skills / "example/SKILL.md").write_text("changed")
        with self.assertRaises(ValueError):
            self.g.dispatch("first", Executor(), FixtureTokenLayer())
        self.assertEqual(self.g.snapshot()["tasks"][0]["attempts"], 0)

    def test_interruption_and_reopen_never_automatically_reexecute(self):
        self.g.plan(plan())
        executor = Executor(crash=True)
        with self.assertRaises(SystemExit):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        self.g.close()
        self.g = self.open()
        current = self.g.snapshot()["tasks"][0]
        self.assertEqual(current["state"], "running")
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        self.g.mark_uncertain(current["latest_attempt"], actor="operator", reason="Fixture callback ended; transport report was lost")
        self.g.submit_report(current["latest_attempt"], executor.last_report, actor="worker")
        self.assertEqual(self.g.verify("first"), "accepted")
        self.assertEqual(executor.calls, 1)

    def test_concurrent_dispatch_has_one_executor_effect(self):
        self.g.plan(plan())
        started, release = threading.Event(), threading.Event()
        errors = []
        class WaitingExecutor(Executor):
            def execute(self, brief, activation):
                started.set()
                if not release.wait(5):
                    raise RuntimeError("test synchronization timeout")
                return super().execute(brief, activation)
        executor = WaitingExecutor()
        def run():
            governor = None
            try:
                governor = Governor(self.database, self.workspace, self.skills)
                governor.dispatch("first", executor, FixtureTokenLayer())
            except Exception as error:
                errors.append(error)
            finally:
                if governor:
                    governor.close()
        thread = threading.Thread(target=run)
        thread.start()
        try:
            self.assertTrue(started.wait(5))
            with self.assertRaises(ValueError):
                self.g.dispatch("first", Executor(), FixtureTokenLayer())
        finally:
            release.set()
            thread.join(5)
        self.assertFalse(thread.is_alive())
        self.assertEqual(errors, [])
        self.assertEqual(executor.calls, 1)
        self.assertEqual(self.g.snapshot()["tasks"][0]["attempts"], 1)

    def test_unknown_schema_corruption_and_state_location_are_refused(self):
        with self.assertRaises(ValueError):
            Governor(self.workspace / "unsafe.sqlite", self.workspace, self.skills)
        self.g.close()
        with closing(sqlite3.connect(self.database)) as connection:
            connection.execute("PRAGMA user_version=99")
            connection.commit()
        with self.assertRaises(ValueError):
            self.open()
        corrupt = self.root / "corrupt.sqlite"
        corrupt.write_bytes(b"not sqlite; preserve these bytes")
        with self.assertRaises(sqlite3.DatabaseError):
            Governor(corrupt, self.workspace, self.skills)
        self.assertEqual(corrupt.read_bytes(), b"not sqlite; preserve these bytes")

    def test_invalid_persisted_task_is_not_silently_reinitialized(self):
        self.g.plan(plan())
        self.g.db.execute("UPDATE tasks SET state='invented'")
        self.g.close()
        with self.assertRaises(ValueError):
            self.open()

    def test_json_verifier_preserves_types_and_memory_supersession_history(self):
        spec = task(owns=["result.json"], acceptance=[{"kind": "json_equals", "path": "result.json", "value": {"ok": True}}])
        self.g.plan(plan(spec))
        bad = Executor('{"ok":1}')
        self.assertEqual(self.finish(executor=bad)[1], "revision")
        good = Executor('{"ok":true}', report_change=lambda report: report["candidate_memories"].append({"text": "Superseding scoped decision", "evidence": ["result.json"]}))
        self.assertEqual(self.finish(executor=good)[1], "accepted")
        first = self.g.admit_memory("first", 0, actor="governor", kind="observation", rationale="bounded observation")
        second = self.g.admit_memory("first", 1, actor="governor", kind="decision", rationale="retain the better scoped decision", supersedes=first)
        memories = self.g.snapshot()["memories"]
        self.assertEqual([(m["id"], m["active"]) for m in memories], [(first, 0), (second, 1)])

    def test_actual_process_exit_preserves_dispatch_and_recovery_without_second_effect(self):
        self.g.plan(plan())
        project = Path(__file__).resolve().parent.parent
        script = "\n".join([
            "import sys,json,os",
            f"sys.path.insert(0,{str(project)!r})",
            "from pathlib import Path",
            "from orchestration import Governor",
            "from orchestration.fixtures import FixtureTokenLayer",
            f"g=Governor({str(self.database)!r},{str(self.workspace)!r},{str(self.skills)!r})",
            "class CrashExecutor:",
            "    mode='fixture'",
            "    actor='worker'",
            "    def execute(self,brief,activation):",
            f"        Path({str(self.workspace / 'first.txt')!r}).write_text('ready')",
            f"        Path({str(self.root / 'effect-count.txt')!r}).write_text('one effect')",
            "        os._exit(23)",
            "g.dispatch('first',CrashExecutor(),FixtureTokenLayer())",
        ])
        result = subprocess.run([sys.executable, "-c", script], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 23, result.stderr.decode(errors="replace"))
        current = self.g.snapshot()["tasks"][0]
        self.assertEqual(current["state"], "running")
        attempt_id = current["latest_attempt"]
        with self.assertRaises(ValueError):
            self.g.dispatch("first", Executor(), FixtureTokenLayer())
        self.g.mark_uncertain(attempt_id, actor="operator", reason="Observed child process terminal exit 23; inspect existing artifact")
        brief = json.loads(self.g._attempt(attempt_id)["brief"])
        self.g.submit_report(attempt_id, fixture_report(brief, artifacts=["first.txt"]), actor="worker")
        self.assertEqual(self.g.verify("first"), "accepted")
        self.assertEqual((self.root / "effect-count.txt").read_text(), "one effect")

    def test_active_and_uncertain_ownership_blocks_another_task(self):
        shared = task("second", owns=["first.txt"], acceptance=[{"kind": "contains", "path": "first.txt", "value": "ready"}])
        self.g.plan(plan(task(), shared))
        executor = Executor(crash=True)
        with self.assertRaises(SystemExit):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        with self.assertRaises(ValueError):
            self.g.dispatch("second", Executor(), FixtureTokenLayer())
        attempt_id = self.g.snapshot()["tasks"][0]["latest_attempt"]
        self.g.mark_uncertain(attempt_id, actor="operator", reason="Outcome needs reconciliation")
        with self.assertRaises(ValueError):
            self.g.dispatch("second", Executor(), FixtureTokenLayer())
        self.assertEqual(self.g.snapshot()["tasks"][1]["attempts"], 0)

    def test_usage_keeps_cache_subset_and_unknowns_and_reconciles_predictions(self):
        self.g.plan(plan())
        class PredictingFixture(FixtureTokenLayer):
            def activate(self, brief):
                receipt = super().activate(brief)
                receipt.update(predicted_input_tokens=90, predicted_output_tokens=20)
                return receipt
        executor = Executor(report_change=lambda report: report.update(usage={"source": "fixture", "input_tokens": 100, "cached_input_tokens": 80, "output_tokens": 15}))
        self.g.dispatch("first", executor, PredictingFixture())
        usage = self.g.snapshot()["tasks"][0]["usage"]
        self.assertEqual((usage["total_tokens"], usage["uncached_input_tokens"]), (115, 20))
        self.assertEqual((usage["input_prediction_error"], usage["output_prediction_error"]), (10, -5))
        self.assertIsNone(usage["cost"])

    def test_unknown_usage_stays_null_and_impossible_cache_is_rejected(self):
        self.g.plan(plan())
        executor = Executor(report_change=lambda report: report.update(usage={"source": "fixture", "input_tokens": 10, "cached_input_tokens": 11, "output_tokens": 1}))
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        attempt_id = self.g.snapshot()["tasks"][0]["latest_attempt"]
        executor.last_report["usage"] = {"source": "unknown", "input_tokens": None, "cached_input_tokens": None, "output_tokens": None}
        self.g.submit_report(attempt_id, executor.last_report, actor="worker")
        usage = self.g.snapshot()["tasks"][0]["usage"]
        self.assertIsNone(usage["total_tokens"])
        self.assertIsNone(usage["input_prediction_error"])

    def test_verifier_requires_the_criterion_artifact_in_the_report(self):
        self.g.plan(plan(task(owns=["output"], acceptance=[{"kind": "contains", "path": "output/required.txt", "value": "ready"}])))
        (self.workspace / "output").mkdir()
        (self.workspace / "output/required.txt").write_text("ready")
        class OtherArtifact:
            mode = "fixture"
            actor = "worker"
            def execute(inner, brief, activation):
                (self.workspace / "output/other.txt").write_text("ready")
                return fixture_report(brief, artifacts=["output/other.txt"])
        self.g.dispatch("first", OtherArtifact(), FixtureTokenLayer())
        self.assertEqual(self.g.verify("first"), "revision")

    def test_duplicate_json_keys_are_not_accepted_as_unambiguous_data(self):
        self.g.plan(plan(task(owns=["data.json"], acceptance=[{"kind": "json_equals", "path": "data.json", "value": {"ok": True}}])))
        self.assertEqual(self.finish(executor=Executor('{"ok":false,"ok":true}'))[1], "revision")

    def test_reopen_refuses_fabricated_accepted_state_without_an_attempt(self):
        self.g.plan(plan())
        self.g.db.execute("UPDATE tasks SET state='accepted'")
        self.g.close()
        with self.assertRaises(ValueError):
            self.open()

    def test_reopen_validates_persisted_report_and_identity(self):
        self.g.plan(plan())
        self.finish()
        attempt = self.g._latest("first")
        report = json.loads(attempt["report"])
        report["usage"]["input_tokens"] = -1
        self.g.db.execute("UPDATE attempts SET report=? WHERE id=?", (json.dumps(report), attempt["id"]))
        self.g.close()
        with self.assertRaises(ValueError):
            self.open()

    def test_existing_empty_database_is_not_silently_initialized(self):
        database = self.root / "truncated.sqlite"
        database.touch()
        with self.assertRaises(ValueError):
            Governor(database, self.workspace, self.skills)
        self.assertEqual(database.read_bytes(), b"")

    def test_context_modified_during_activation_refuses_executor_launch(self):
        source = self.workspace / "input.txt"
        source.write_text("original")
        self.g.plan(plan(task(context_files=["input.txt"])))
        class ChangingContext(FixtureTokenLayer):
            def activate(inner, brief):
                receipt = super().activate(brief)
                source.write_text("changed")
                return receipt
        executor = Executor()
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, ChangingContext())
        self.assertEqual(executor.calls, 0)
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "revision")

    def test_immutable_context_change_after_report_blocks_verification(self):
        source = self.workspace / "input.txt"
        source.write_text("original")
        self.g.plan(plan(task(context_files=["input.txt"])))
        self.g.dispatch("first", Executor(), FixtureTokenLayer())
        source.write_text("changed")
        self.assertEqual(self.g.verify("first"), "revision")

    def test_context_change_after_acceptance_blocks_completion_and_memory(self):
        source = self.workspace / "input.txt"
        source.write_text("original")
        self.g.plan(plan(task(context_files=["input.txt"])))
        self.finish()
        source.write_text("changed")
        self.assertFalse(self.g.snapshot()["complete"])
        with self.assertRaises(ValueError):
            self.g.admit_memory("first", 0, actor="governor", kind="fact", rationale="stale input")

    def test_pack_modified_during_activation_refuses_executor_launch(self):
        self.g.plan(plan())
        class ChangingPack(FixtureTokenLayer):
            def activate(inner, brief):
                receipt = super().activate(brief)
                (self.skills / "example/SKILL.md").write_text("changed")
                return receipt
        executor = Executor()
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, ChangingPack())
        self.assertEqual(executor.calls, 0)

    def test_dependency_modified_during_activation_refuses_executor_launch(self):
        self.g.plan(plan(task(), task("second", dependencies=["first"])))
        self.finish()
        class ChangingDependency(FixtureTokenLayer):
            def activate(inner, brief):
                receipt = super().activate(brief)
                (self.workspace / "first.txt").write_text("changed upstream")
                return receipt
        executor = Executor()
        with self.assertRaises(ValueError):
            self.g.dispatch("second", executor, ChangingDependency())
        self.assertEqual(executor.calls, 0)

    def test_transforming_an_owned_input_requires_separate_immutable_context(self):
        with self.assertRaises(ValueError):
            self.g.plan(plan(task(context_files=["first.txt"])))
        self.assertEqual(self.g.snapshot()["tasks"], [])

    def test_uncertainty_during_activation_is_not_reset_into_a_retry(self):
        self.g.plan(plan())
        class ObservedUncertainty(FixtureTokenLayer):
            def activate(inner, brief):
                receipt = super().activate(brief)
                self.g.mark_uncertain(brief["attempt_id"], actor="observer", reason="External state needs reconciliation")
                return receipt
        executor = Executor()
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, ObservedUncertainty())
        self.assertEqual(executor.calls, 0)
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "uncertain")

    def test_benchmark_comparison_refuses_a_different_capability_pack(self):
        from orchestration.benchmark import benchmark
        from capabilities.scripts.compare_receipts import compare
        root = self.skills.parent
        skill = self.skills / "rust-performance/SKILL.md"
        skill.parent.mkdir()
        skill.write_text("# Bounded benchmark fixture skill\n")
        manifest = json.loads((root / "pack-manifest.json").read_text())
        manifest["skills"].append("rust-performance")
        manifest["files"]["skills/rust-performance/SKILL.md"] = hashlib.sha256(skill.read_bytes()).hexdigest()
        (root / "pack-manifest.json").write_text(json.dumps(manifest))
        before = benchmark(self.skills, runs=1, warmups=0)
        extra = root / "extra.txt"
        extra.write_text("additional manifest work")
        manifest["files"]["extra.txt"] = hashlib.sha256(extra.read_bytes()).hexdigest()
        (root / "pack-manifest.json").write_text(json.dumps(manifest))
        after = benchmark(self.skills, runs=1, warmups=0)
        with self.assertRaises(ValueError):
            compare(before["comparison_receipt"], after["comparison_receipt"])

    def test_fixture_execution_cannot_label_counts_as_real_runtime_usage(self):
        self.g.plan(plan())
        executor = Executor(report_change=lambda report: report.update(usage={"source": "runtime", "input_tokens": 10, "cached_input_tokens": 0, "output_tokens": 1}))
        with self.assertRaises(ValueError):
            self.g.dispatch("first", executor, FixtureTokenLayer())
        self.assertEqual(self.g.snapshot()["tasks"][0]["state"], "uncertain")


if __name__ == "__main__":
    unittest.main()
