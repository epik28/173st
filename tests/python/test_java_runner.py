"""Parser and comparator tests for the JAVA-001 runner."""
import hashlib
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
import java_run


class JavaRunnerTests(unittest.TestCase):
    def test_fixture_parses_with_expected_operation_coverage(self):
        data = (ROOT / "tests/fixtures/java/input.tsv").read_bytes()
        cases = java_run.parse_input(data)
        audit = json.loads((ROOT / "tests/fixtures/java/audit.json").read_text())
        self.assertEqual(len(cases), audit["case_count"])
        self.assertEqual(hashlib.sha256(data).hexdigest(), audit["input_sha256"])
        self.assertEqual(sum(case["op"] == "string_hash_code" for case in cases), 2057)

    def test_parser_rejects_bad_bits_utf8_duplicate_and_framing(self):
        for data in (b"a\tunknown\t00\n", b"a\tf32_to_i32\txyz\n",
                     b"a\tstring_hash_code\tff\n",
                     b"a\tf32_to_i32\t00000000\na\tf64_to_i32\t0000000000000000\n",
                     b"a\tf32_to_i32\t00000000"):
            with self.subTest(data=data), self.assertRaises(java_run.JavaRunError):
                java_run.parse_input(data)

    def test_comparator_is_exact_and_reports_first_mismatch(self):
        cases = java_run.parse_input(b"a\tf32_to_i32\t00000000\nb\tstring_hash_code\t\n")
        reference = b"a\t0\nb\t0\n"
        self.assertEqual(java_run.compare(reference, reference, cases),
                         {"mismatch_count": 0, "first_mismatch": None})
        mismatch = java_run.compare(reference, b"a\t1\nb\t0\n", cases)
        self.assertEqual(mismatch["mismatch_count"], 1)
        self.assertEqual(mismatch["first_mismatch"]["case"], "a")

    def test_output_rejects_width_overflow_wrong_order_and_wrong_count(self):
        cases = java_run.parse_input(b"a\tf32_to_i32\t00000000\nb\tf64_to_i64\t0000000000000000\n")
        for output in (b"a\t0\n", b"b\t0\na\t0\n", b"a\t2147483648\nb\t0\n"):
            with self.subTest(output=output), self.assertRaises(java_run.JavaRunError):
                java_run.parse_output(output, cases, "test")


if __name__ == "__main__":
    unittest.main()
