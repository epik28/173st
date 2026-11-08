"""Structural checks for the committed native Beta math corpus."""
import hashlib
import json
from pathlib import Path
import unittest

from tools.oracle.math_corpus import corpus

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/math"


class MathCorpusTests(unittest.TestCase):
    def test_corpus_is_deterministic_and_addresses_every_sine_table_entry(self):
        rows, covered = corpus()
        self.assertEqual(len(rows), 131408)
        self.assertEqual(covered, set(range(65536)))
        self.assertEqual(("\n".join(rows) + "\n").encode(), (FIXTURE / "input.tsv").read_bytes())
        ids = [row.split("\t", 1)[0] for row in rows]
        self.assertEqual(len(ids), len(set(ids)))

    def test_native_outputs_and_audit_are_bound_to_the_corpus(self):
        audit = json.loads((FIXTURE / "audit.json").read_text())
        input_data = (FIXTURE / "input.tsv").read_bytes()
        output_data = (FIXTURE / "reference.tsv").read_bytes()
        self.assertEqual(audit["kind"], "native Beta 1.7.3 MathHelper oracle")
        self.assertEqual(audit["class_name"], "in")
        self.assertEqual(audit["class_sha256"], "8693f3179fb0ff1df382e1d3388d23ab688ad3e26415454655da88f10b054f63")
        self.assertEqual(hashlib.sha256(input_data).hexdigest(), audit["input_sha256"])
        self.assertEqual(hashlib.sha256(output_data).hexdigest(), audit["output_sha256"])
        self.assertEqual(audit["output_sha256"], audit["repeat_output_sha256"])
        self.assertEqual(len(input_data.splitlines()), audit["operations"])
        self.assertEqual(len(output_data.splitlines()), audit["operations"])
        self.assertEqual(audit["trig_lut_indices_per_method"], {"sin": 65536, "cos": 65536})
        for expected in ("error_arithmetic",):
            self.assertIn(expected, output_data.decode("ascii"))

    def test_protocol_outputs_have_unique_ids_and_valid_raw_float_bits(self):
        lines = (FIXTURE / "reference.tsv").read_text().splitlines()
        self.assertEqual(len(lines), 131408)
        ids = [line.split("\t", 1)[0] for line in lines]
        self.assertEqual(len(ids), len(set(ids)))
        values = {line.split("\t", 1)[0]: line.split("\t", 1)[1] for line in lines}
        for op in ("sin", "cos"):
            for i in range(65536):
                self.assertRegex(values[f"{op}-lut-{i:05d}"], r"^[0-9a-f]{8}$")


if __name__ == "__main__":
    unittest.main()
