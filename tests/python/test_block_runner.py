"""BLOCK-001 ray-only runner schema, limits, provenance and mutation checks."""

from pathlib import Path
import tempfile
import sys
import unittest
from unittest import mock

from tools.oracle import block_corpus
from tools.parity import block_run, block_review


def probe_output(native=None):
    if native is None:
        native = block_run.ROOT.joinpath("tests/fixtures/block/native.tsv").read_bytes()
    refs = block_corpus.parse(native)
    rows = [block_run.JAVA_HEADER + b"\n", block_run.PROBE_HEADER + b"\n"]
    names = ("id", "kind", "x", "y", "z", "side", "px", "py", "pz", "px_bits", "py_bits", "pz_bits")
    rows.extend(("\t".join(row[k] for k in names) + "\n").encode("ascii") for row in refs)
    return b"".join(rows)


class BlockRunnerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native = (block_run.ROOT / "tests/fixtures/block/native.tsv").read_bytes()
        cls.probe = probe_output(cls.native)

    def test_pinned_java_case_input_has_scene_and_ray_data(self):
        raw = (block_run.ROOT / "tests/fixtures/block/input.tsv").read_bytes()
        parsed = block_corpus.parse_inputs(raw)
        self.assertEqual(len(parsed), 17)
        self.assertEqual(parsed[0]["block_id"], "1")
        self.assertEqual(parsed[7]["block_x"], "-1")
        self.assertEqual(parsed[11]["shape"], "slab")
        self.assertEqual(parsed[13]["stop_liquid"], "1")
        self.assertEqual(block_run.probe_input(), raw)

    def test_ray_only_schema_compares_exact_hit_fields_and_raw_words(self):
        self.assertEqual(block_run.compare_ray_outputs(self.native, self.probe),
                         {"mismatch_count": 0, "first_mismatch": None})
        changed = self.probe.replace(b"\t4\t0.0\t0.5", b"\t5\t0.0\t0.5", 1)
        self.assertEqual(block_run.compare_ray_outputs(self.native, changed)["first_mismatch"]["case_id"], "face_west")
        changed = self.probe.replace(b"0000000000000000", b"0000000000000001", 1)
        self.assertIn("disagrees", block_run.compare_ray_outputs(self.native, changed)["first_mismatch"]["error"])
        changed = self.probe.replace(b"\t0.0\t0.5\t0.5\t0000000000000000",
                                     b"\t5e-324\t0.5\t0.5\t0000000000000001", 1)
        self.assertEqual(block_run.compare_ray_outputs(self.native, changed)["first_mismatch"]["case_id"], "face_west")
        changed = self.probe.replace(b"\t-1\t0\t0\t4\t-1.0", b"\t1\t0\t0\t4\t-1.0", 1)
        self.assertEqual(block_run.compare_ray_outputs(self.native, changed)["mismatch_count"], 1)

    def test_probe_parser_rejects_extra_action_fields_order_and_limits(self):
        extra_action = self.probe.replace(b"face_west\tblock", b"face_west\tblock\tneighbor_id=1", 1)
        for field in (b"neighbor_id=1", b"neighbor_meta=0", b"stack_count=1", b"stack_damage=0"):
            action_row = self.probe.replace(b"face_west\tblock", b"face_west\tblock\t" + field, 1)
            with self.subTest(field=field), self.assertRaises(ValueError):
                block_run.parse_probe(action_row)
        reordered = self.probe.replace(b"face_west", b"face_east", 1)
        for raw in (extra_action, reordered, self.probe + b"extra\n", self.probe + b" " * block_run.MAX_OUTPUT_BYTES):
            with self.subTest(size=len(raw)), self.assertRaises(ValueError):
                block_run.parse_probe(raw)

    def test_input_parser_rejects_malformed_extra_and_over_limit_cases(self):
        raw = (block_run.ROOT / "tests/fixtures/block/input.tsv").read_bytes()
        mutations = (raw.replace(b"face_west\t-2.0", b"face_west\tNaN", 1),
                    raw.replace(b"\t1\t0\tcube", b"\t999\t0\tcube", 1),
                    raw + raw.splitlines(keepends=True)[2],
                    raw + b" " * 4096)
        for damaged in mutations:
            with self.subTest(size=len(damaged)), self.assertRaises(ValueError):
                block_corpus.parse_inputs(damaged)
        for damaged in (raw.rstrip(b"\n"), raw.replace(b"\n", b"\r\n")):
            with self.subTest(line_ending=damaged[-8:]), self.assertRaises(ValueError):
                block_corpus.parse_inputs(damaged)

    def test_process_timeout_is_retained_and_reported(self):
        with tempfile.TemporaryDirectory(prefix="block-timeout-") as temp:
            directory = Path(temp)
            report = {"processes": {}}
            with self.assertRaises(block_run.BlockRunError):
                block_run._record(directory, report, "timeout",
                                  [sys.executable, "-c", "import time; time.sleep(0.2)"],
                                  0.01, directory)
            record = report["processes"]["timeout"]
            self.assertTrue(record["timeout"])
            self.assertIn("timeout after", record["error"])
            self.assertTrue(Path(record["stdout"]).is_file())
            self.assertTrue(Path(record["stderr"]).is_file())

    def test_wrong_jar_is_error_before_native_or_probe_claims(self):
        with tempfile.TemporaryDirectory(prefix="block-wrong-jar-") as temp:
            root = Path(temp)
            (root / "wrong.jar").write_bytes(b"wrong")
            (root / "config").mkdir()
            (root / "config/local.toml").write_text('[paths]\nclient_jar = "wrong.jar"\n')
            with mock.patch.object(block_run.parity, "artifacts_root", return_value=root / "artifacts"):
                report, _ = block_run.run_block(root=root)
            self.assertEqual(report["status"], "error")
            self.assertIn("SHA-256", report["reason"])
            self.assertEqual(report["action_status"], "not_implemented")
            self.assertFalse(report["actions_compared"])

    def test_report_reviewer_rejects_outside_tree(self):
        with self.assertRaises((ValueError, OSError)):
            block_review.verify_report("/etc/passwd")


if __name__ == "__main__":
    unittest.main()
