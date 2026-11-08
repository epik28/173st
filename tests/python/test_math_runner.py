"""Parser and exact comparator tests for MATH-001 live reports."""
import hashlib
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
import math_run


class MathRunnerTests(unittest.TestCase):
    def test_committed_fixture_parses_and_has_full_lut_coverage(self):
        data = (ROOT / "tests/fixtures/math/input.tsv").read_bytes()
        cases = math_run.parse_math_input(data)
        audit = json.loads((ROOT / "tests/fixtures/math/audit.json").read_text())
        self.assertEqual(len(cases), 131408)
        self.assertEqual(hashlib.sha256(data).hexdigest(), audit["input_sha256"])
        self.assertEqual(sum(case["op"] == "sin" for case in cases), 65536)
        self.assertEqual(sum(case["op"] == "cos" for case in cases), 65536)
        self.assertEqual(len({case["id"] for case in cases}), len(cases))

    def test_parser_rejects_malformed_types_duplicate_ids_and_bad_utf8(self):
        invalid = (
            b"id\tunknown\t0\n",
            b"id\tsin\txyz\n",
            b"id\tabs_max_f64\t0000000000000000\n",
            b"id\tfloor_div_i32\t2147483648\t1\n",
            b"id\tnull_or_empty\tff\n",
            b"id\tsin\t00000000\nid\tcos\t00000000\n",
            b"id\tsin\t00000000",
        )
        for data in invalid:
            with self.subTest(data=data), self.assertRaises(math_run.MathRunError):
                math_run.parse_math_input(data)

    def test_comparator_is_exact_and_reports_first_mismatch(self):
        data = b"a\tsin\t00000000\nb\tfloor_div_i32\t-3\t2\n"
        cases = math_run.parse_math_input(data)
        java = b"a\t00000000\nb\t-2\n"
        self.assertEqual(math_run.compare_math_outputs(java, java, cases), {
            "mismatch_count": 0, "first_mismatch": None})
        changed = b"a\t00000001\nb\t-2\n"
        result = math_run.compare_math_outputs(java, changed, cases)
        self.assertEqual(result["mismatch_count"], 1)
        self.assertEqual(result["first_mismatch"], {
            "record": 0, "case": "a", "operation": "sin",
            "reference": "00000000", "rust": "00000001"})

    def test_comparator_rejects_missing_reordered_and_wrong_typed_rows(self):
        cases = math_run.parse_math_input(b"a\tsin\t00000000\nb\tfloor_f32\t3f800000\n")
        expected = b"a\t00000000\nb\t1\n"
        for actual in (b"a\t00000000\n", b"b\t1\na\t00000000\n",
                       b"a\t00000000\nb\t1.0\n", b"a\t00000000\nb\t2147483648\n"):
            with self.subTest(actual=actual), self.assertRaises(math_run.MathRunError):
                math_run.compare_math_outputs(expected, actual, cases)


if __name__ == "__main__":
    unittest.main()
