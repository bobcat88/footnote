import copy
import unittest

from evaluation.receipts import summarize


def sample():
    return {"schema_version": 1, "arm": "flow", "mode": "fixture", "outcome": "accepted", "capture_complete": True, "events_ms": {"submitted": 0, "executor_ready": 10, "first_useful_action": 15, "first_artifact": 70, "accepted": 90, "terminal": 100}, "agents": {"a": "coordinator", "b": "specialist"}, "calls": [{"call_id": "c1", "agent_id": "a", "source": "fixture", "accounting": "per-call", "input_tokens": 100, "cached_input_tokens": 80, "output_tokens": 10}, {"call_id": "c2", "agent_id": "b", "source": "fixture", "accounting": "per-call", "input_tokens": 200, "cached_input_tokens": 100, "output_tokens": 20}]}


class ReceiptTests(unittest.TestCase):
    def test_includes_coordinator_and_child_without_double_counting_cache(self):
        result = summarize(sample())
        self.assertEqual(result["run_total_tokens"], 330)
        self.assertEqual(result["per_agent"]["a"]["total_tokens"], 110)
        self.assertEqual(result["durations_ms"]["time_to_start"], 10)
        self.assertEqual(result["durations_ms"]["time_to_accepted_delivery"], 90)

    def test_unknown_and_incomplete_capture_never_become_zero(self):
        value = sample()
        value["calls"][1]["output_tokens"] = None
        self.assertIsNone(summarize(value)["run_total_tokens"])
        value = sample()
        value["capture_complete"] = False
        self.assertIsNone(summarize(value)["run_total_tokens"])

    def test_duplicate_and_cumulative_usage_refused(self):
        value = sample()
        value["calls"].append(copy.deepcopy(value["calls"][0]))
        with self.assertRaises(ValueError): summarize(value)
        value = sample()
        value["calls"][0]["accounting"] = "cumulative"
        with self.assertRaises(ValueError): summarize(value)

    def test_failed_run_retains_consumption_and_terminal_time(self):
        value = sample()
        value["outcome"] = "timeout"
        value["events_ms"]["accepted"] = None
        result = summarize(value)
        self.assertEqual(result["run_total_tokens"], 330)
        self.assertIsNone(result["durations_ms"]["time_to_accepted_delivery"])
        self.assertEqual(result["durations_ms"]["terminal"], 100)

    def test_mode_and_invalid_counts_refused(self):
        for key, bad in (("input_tokens", True), ("output_tokens", -1), ("cached_input_tokens", 101), ("source", "real")):
            value = sample()
            value["calls"][0][key] = bad
            with self.subTest(key=key), self.assertRaises(ValueError): summarize(value)

    def test_invalid_times_and_false_acceptance_refused(self):
        for key, bad in (("executor_ready", -1), ("accepted", 101), ("terminal", None), ("first_artifact", float("nan"))):
            value = sample()
            value["events_ms"][key] = bad
            with self.subTest(key=key), self.assertRaises(ValueError): summarize(value)
        value = sample()
        value["outcome"] = "failed"
        with self.assertRaises(ValueError): summarize(value)


if __name__ == "__main__":
    unittest.main()
