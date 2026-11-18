import unittest

from evaluation.measurement_adapter import DECISION, MeasurementOnlyAdapter
from evaluation.receipts import summarize
from orchestration import test_governor as helpers


def brief():
    return {"mode": "real", "project_decisions": [DECISION], "runtime": {"adapter": "flow-evaluation:calibration"}, "task_id": "first", "attempt_id": "attempt-1"}


def usage(**updates):
    value = {"source": "runtime", "accounting": "per-call", "input_tokens": 100, "cached_input_tokens": 80, "output_tokens": 10}
    value.update(updates)
    return value


class AdapterTests(unittest.TestCase):
    def test_labels_and_null_predictions(self):
        adapter = MeasurementOnlyAdapter("calibration")
        receipt = adapter.activate(brief())
        self.assertFalse(receipt["prediction_enabled"])
        self.assertIsNone(receipt["predicted_input_tokens"])
        self.assertIsNone(receipt["predicted_output_tokens"])
        self.assertFalse(adapter.snapshot()["capture_complete"])
        self.assertEqual(adapter.snapshot()["calls"], [])

    def test_production_fixture_and_duplicate_activation_refused(self):
        for key, value in (("mode", "fixture"), ("project_decisions", []), ("runtime", {"adapter": "production"})):
            wrong = brief()
            wrong[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                MeasurementOnlyAdapter("calibration").activate(wrong)
        adapter = MeasurementOnlyAdapter("calibration")
        adapter.activate(brief())
        with self.assertRaises(ValueError): adapter.activate(brief())

    def test_usage_requires_activation_runtime_and_unique_call(self):
        adapter = MeasurementOnlyAdapter("calibration")
        arguments = dict(attempt_id="attempt-1", agent_id="worker", call_id="call-1", usage=usage())
        with self.assertRaises(ValueError): adapter.record_call(**arguments)
        adapter.activate(brief())
        record = adapter.record_call(**arguments)
        self.assertEqual(record["input_tokens"], 100)
        with self.assertRaises(ValueError): adapter.record_call(**arguments)
        for changes in ({"source": "fixture"}, {"accounting": "cumulative"}, {"input_tokens": True}, {"cached_input_tokens": 101}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                adapter.record_call(**{**arguments, "call_id": "bad", "usage": usage(**changes)})

    def test_unknown_counts_preserved_and_export_cannot_mutate_capture(self):
        adapter = MeasurementOnlyAdapter("calibration")
        adapter.activate(brief())
        adapter.record_call(attempt_id="attempt-1", agent_id="worker", call_id="call-1", usage=usage(output_tokens=None))
        snapshot = adapter.snapshot()
        snapshot["calls"][0]["input_tokens"] = 999
        self.assertEqual(adapter.snapshot()["calls"][0]["input_tokens"], 100)
        result = summarize({"schema_version": 1, "arm": "flow", "mode": "real", "outcome": "failed", "events_ms": {"submitted": 0, "terminal": 10}, "agents": {"worker": "specialist"}, "calls": adapter.snapshot()["calls"], "capture_complete": True})
        self.assertIsNone(result["run_total_tokens"])


class GovernorAdapterTests(unittest.TestCase):
    setUp = helpers.GovernorTests.setUp
    open = helpers.GovernorTests.open

    def test_governor_accepts_authorized_boundary_without_claiming_model_execution(self):
        project = helpers.plan()
        project["decisions"].append(DECISION)
        self.g.plan(project)
        # Real-mode contract calibration only: this callback makes no model call.
        class CalibrationExecutor(helpers.Executor):
            mode = "real"
            name = "flow-evaluation:contract-calibration-no-model"
            version = "1"
            engine = "calibration:no-model"
        adapter = MeasurementOnlyAdapter("contract-calibration")
        self.g.dispatch("first", CalibrationExecutor(), adapter)
        self.assertEqual(self.g.verify("first"), "accepted")
        event = next(e for e in self.g.snapshot()["events"] if e["kind"] == "token_activated")
        self.assertIn('"purpose":"measurement-only"', event["payload"])
        self.assertEqual(adapter.snapshot()["calls"], [])
