"""Identical independent acceptance checks for live calibration, not confirmation."""

import importlib.util
import sys
import unittest
from pathlib import Path


def suite(workspace):
    path = Path(workspace) / "sizes.py"
    spec = importlib.util.spec_from_file_location("candidate_sizes", path)
    module = importlib.util.module_from_spec(spec)
    exec(compile(path.read_bytes(), str(path), "exec"), module.__dict__)

    class Acceptance(unittest.TestCase):
        def test_decimal_and_binary_units(self):
            for text, expected in [("0B", 0), ("1KB", 1000), ("1MB", 1000000), ("1GB", 1000000000), ("1KiB", 1024), ("1MiB", 1048576), ("1GiB", 1073741824)]:
                with self.subTest(text=text): self.assertEqual(module.parse_byte_size(text), expected)

        def test_exact_fractional_conversion(self):
            for text, expected in [("0.5 KiB", 512), ("0.001KB", 1), ("1.125 KiB", 1152), ("9007199254740993 B", 9007199254740993), ("0.000000001 GB", 1)]:
                with self.subTest(text=text):
                    actual = module.parse_byte_size(text)
                    self.assertIs(type(actual), int)
                    self.assertEqual(actual, expected)

        def test_whitespace(self):
            for text in (" 1 KB ", "\t1\tKB\n", "1KB"):
                with self.subTest(text=text): self.assertEqual(module.parse_byte_size(text), 1000)

        def test_nonintegral_bytes_refused(self):
            for text in ("0.1 B", "0.0001 KB", "0.1 KiB"):
                with self.subTest(text=text), self.assertRaises(ValueError): module.parse_byte_size(text)

        def test_malformed_and_nonstring_refused(self):
            for text in (None, True, 1024, b"1KB", "", "KB", "1", "-1KB", "+1KB", "1e3B", "1kb", "1TB", "NaN B", "inf B", "1 KB junk", "1_000B", "1 0KB", "1..0KB"):
                with self.subTest(text=text), self.assertRaises(ValueError): module.parse_byte_size(text)

        def test_delivery_and_public_tests_exist(self):
            self.assertTrue((Path(workspace) / "delivery.txt").read_text(encoding="utf-8").strip())
            self.assertIn("test", (Path(workspace) / "test_sizes.py").read_text(encoding="utf-8"))

    return unittest.defaultTestLoader.loadTestsFromTestCase(Acceptance)


if __name__ == "__main__":
    result = unittest.TextTestRunner(verbosity=2).run(suite(sys.argv[1]))
    raise SystemExit(0 if result.wasSuccessful() else 1)
