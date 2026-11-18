"""Common parent-only functional checks for the integration proof."""

import copy
import importlib.util
import sys
import unittest
from pathlib import Path


def checks(workspace, followup=False):
    path = Path(workspace) / "intervals.py"
    spec = importlib.util.spec_from_file_location("candidate", path)
    module = importlib.util.module_from_spec(spec)
    exec(compile(path.read_bytes(), str(path), "exec"), module.__dict__)

    class Checks(unittest.TestCase):
        def test_overlap(self):
            self.assertEqual(module.merge_intervals([(3, 7), (1, 5), (10, 12)]), [(1, 7), (10, 12)])

        def test_touching_boundaries_preserved(self):
            self.assertEqual(module.merge_intervals([(0, 1), (1, 2)]), [(0, 1), (1, 2)])

        def test_empty_nested_and_negative(self):
            self.assertEqual(module.merge_intervals([]), [])
            self.assertEqual(module.merge_intervals([(-2, 2), (-1, 1)]), [(-2, 2)])

        def test_validation(self):
            for value in (None, [(1, 1)], [(3, 1)], [(True, 3)], [(1, 2.0)], [(1, 2, 3)], ["12"]):
                with self.subTest(value=value), self.assertRaises(ValueError): module.merge_intervals(value)

        def test_inputs_unchanged(self):
            value = [[4, 7], [1, 5]]
            before = copy.deepcopy(value)
            self.assertEqual(module.merge_intervals(value), [(1, 7)])
            self.assertEqual(value, before)

        @unittest.skipUnless(followup, "follow-up contract only")
        def test_opt_in_touching(self):
            self.assertEqual(module.merge_intervals([(0, 1), (1, 2), (2, 3)], join_touching=True), [(0, 3)])
            self.assertEqual(module.merge_intervals([(0, 1), (1, 2)], join_touching=False), [(0, 1), (1, 2)])
            with self.assertRaises(ValueError): module.merge_intervals([(0, 1)], join_touching=1)

    return unittest.defaultTestLoader.loadTestsFromTestCase(Checks)


if __name__ == "__main__":
    result = unittest.TextTestRunner(verbosity=2).run(checks(sys.argv[1], "--followup" in sys.argv))
    raise SystemExit(0 if result.wasSuccessful() else 1)
