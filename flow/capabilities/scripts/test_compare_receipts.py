import contextlib
import copy
import io
import json
import math
import tempfile
import unittest
from pathlib import Path

from compare_receipts import compare, load, main


def receipt(samples, **changes):
    result = {
        "schema_version": 1,
        "workload_id": "same-input-sha256",
        "environment_id": "same-host-runtime-cache-policy",
        "metric": "elapsed",
        "unit": "ms",
        "lower_is_better": True,
        "revision": "baseline-sha",
        "correctness_passed": True,
        "samples": samples,
    }
    result.update(changes)
    return result


class ComparisonTests(unittest.TestCase):
    def test_default_is_measurement_not_pass_claim(self):
        report = compare(receipt([10, 20, 30]), receipt([12, 22, 32]))
        self.assertEqual(report["verdict"], "measurement-only")
        self.assertEqual(report["regression_pct"], 10)
        self.assertEqual(report["statistical_significance"], "not evaluated")

    def test_direction_and_threshold(self):
        for lower, after, expected in [(True, 12, "over-budget"), (False, 12, "within-budget"), (False, 8, "over-budget")]:
            with self.subTest(lower=lower, after=after):
                report = compare(receipt([10], lower_is_better=lower), receipt([after], lower_is_better=lower), 10)
                self.assertEqual(report["verdict"], expected)

    def test_workload_environment_metric_unit_direction_must_match(self):
        for field, value in [("workload_id", "other"), ("environment_id", "other"), ("metric", "other"), ("unit", "seconds"), ("lower_is_better", False)]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                compare(receipt([1]), receipt([1], **{field: value}))

    def test_rejects_failed_unknown_or_falsey_correctness(self):
        for value in (False, None, 1, "true"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                compare(receipt([1]), receipt([1], correctness_passed=value))

    def test_invalid_samples_and_thresholds(self):
        for samples in ([], [True], [-1], [math.nan], [math.inf], ["1"]):
            with self.subTest(samples=samples), self.assertRaises(ValueError):
                compare(receipt([1]), receipt(samples))
        for limit in (-1, math.nan, math.inf, True):
            with self.subTest(limit=limit), self.assertRaises(ValueError):
                compare(receipt([1]), receipt([1]), limit)

    def test_zero_baseline_cannot_pass_percentage_budget(self):
        self.assertEqual(compare(receipt([0]), receipt([1]), 10)["verdict"], "inconclusive-zero-baseline")

    def test_extreme_values_do_not_produce_nonfinite_reports(self):
        for before, after in [([1e308, 1e308], [1]), ([1e-300], [1e300])]:
            with self.subTest(before=before), self.assertRaises(ValueError):
                compare(receipt(before), receipt(after))

    def test_inputs_are_not_modified(self):
        before, after = receipt([3, 1, 2]), receipt([2, 1, 3])
        original = copy.deepcopy((before, after))
        compare(before, after)
        self.assertEqual((before, after), original)

    def test_cli_codes_and_bounded_file_input(self):
        with tempfile.TemporaryDirectory() as folder:
            first, second = Path(folder) / "before.json", Path(folder) / "after.json"
            first.write_text(json.dumps(receipt([10])), encoding="utf-8")
            second.write_text(json.dumps(receipt([20])), encoding="utf-8")
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(main([str(first), str(second)]), 0)
                self.assertEqual(main([str(first), str(second), "--max-regression-pct", "5"]), 1)
                second.write_text("not json", encoding="utf-8")
                self.assertEqual(main([str(first), str(second)]), 2)
                second.write_text("[" * 2000 + "]" * 2000, encoding="utf-8")
                self.assertEqual(main([str(first), str(second)]), 2)
            second.write_bytes(b" " * 1_048_577)
            with self.assertRaises(ValueError):
                load(second)



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

if __name__ == "__main__":
    unittest.main()
