import copy
import json
import tempfile
import unittest
from pathlib import Path

from evaluation.native_receipts import read_session


class NativeReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / "session.jsonl"
        usage = {"input_tokens": 100, "cached_input_tokens": 80, "output_tokens": 10}
        self.rows = [{"type": "session_meta", "payload": {"id": "session", "agent_path": "child", "parent_thread_id": "parent"}},
                     {"type": "event_msg", "timestamp": "2026-11-17T10:00:00Z", "payload": {"type": "task_started"}},
                     {"type": "response_item", "timestamp": "2026-11-17T10:00:01Z", "payload": {"type": "custom_tool_call"}},
                     {"type": "token_usage_record", "timestamp": "2026-11-17T10:00:02Z", "payload": {"thread_id": "session", "response_id": "response", "usage": usage}},
                     {"type": "event_msg", "timestamp": "2026-11-17T10:00:02Z", "payload": {"type": "token_count", "info": {"total_token_usage": usage, "last_token_usage": usage}}},
                     {"type": "event_msg", "timestamp": "2026-11-17T10:00:03Z", "payload": {"type": "task_complete"}}]

    def read(self):
        self.path.write_text("\n".join(json.dumps(row) for row in self.rows), encoding="utf-8")
        return read_session(self.path, agent_path="child", parent_id="parent")

    def test_provider_response_and_session_totals_reconcile(self):
        result = self.read()
        self.assertEqual(result["calls"][0]["call_id"], "response")
        self.assertEqual(result["elapsed_seconds"], 3)
        self.assertEqual(result["tool_calls"], 1)
        self.assertIsNone(result["ttft_ms"])

    def test_repeated_token_snapshot_does_not_double_count(self):
        self.rows.insert(-1, copy.deepcopy(self.rows[-2]))
        self.assertEqual(len(self.read()["calls"]), 1)

    def test_wrong_parent_and_incomplete_session_refused(self):
        self.rows[0]["payload"]["parent_thread_id"] = "other"
        with self.assertRaises(ValueError): self.read()
        self.rows[0]["payload"]["parent_thread_id"] = "parent"
        self.rows.pop()
        with self.assertRaises(ValueError): self.read()

    def test_duplicate_or_wrong_thread_provider_records_refused(self):
        self.rows.insert(-1, copy.deepcopy(self.rows[3]))
        with self.assertRaises(ValueError): self.read()
        self.rows.pop(-2)
        self.rows[3]["payload"]["thread_id"] = "other"
        with self.assertRaises(ValueError): self.read()

    def test_mismatched_provider_count_refused(self):
        self.rows[3]["payload"]["usage"] = {"input_tokens": 101, "cached_input_tokens": 80, "output_tokens": 10}
        with self.assertRaises(ValueError): self.read()
