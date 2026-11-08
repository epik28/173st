"""Regression tests for the direct JVM compatibility corpus."""
import hashlib
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/oracle"))
import java_compat_corpus


class JavaCompatCorpusTests(unittest.TestCase):
    def test_committed_cases_cover_cast_edges_random_bits_and_utf16_hash(self):
        audit = json.loads((ROOT / "tests/fixtures/java/audit.json").read_text())
        data = (ROOT / "tests/fixtures/java/input.tsv").read_bytes()
        reference = (ROOT / "tests/fixtures/java/reference.tsv").read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), audit["input_sha256"])
        self.assertEqual(hashlib.sha256(reference).hexdigest(), audit["reference_sha256"])
        self.assertEqual(audit["case_count"], 18493)
        self.assertEqual(audit["string_cases"], 2057)
        self.assertTrue(all(audit["operations"][name] >= 4096 for name in (
            "f32_to_i32", "f64_to_i32", "f32_to_i64", "f64_to_i64")))
        self.assertIn("😀".encode().hex(), data.decode("ascii"))
        self.assertTrue(reference.endswith(b"\n"))
        self.assertEqual(len(data.splitlines()), len(reference.splitlines()))

    def test_generator_is_deterministic(self):
        first = java_compat_corpus.input_bytes(java_compat_corpus.build_cases())
        second = java_compat_corpus.input_bytes(java_compat_corpus.build_cases())
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
