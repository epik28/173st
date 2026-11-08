"""FLUID-001 raw-vertex comparator and saved-report verifier tests."""
from pathlib import Path
import json
import tempfile
import unittest
from unittest import mock

from tools.oracle import fluid_corpus
from tools.parity import fluid_run, fluid_review


class FluidRunnerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native = (fluid_corpus.FIXTURES / "native.tsv").read_bytes()
        cls.geometry = fluid_run.geometry_rows(cls.native)

    def test_exact_native_geometry_compares(self):
        self.assertEqual(fluid_run.compare_geometry(self.native, self.geometry),
                         {"mismatch_count": 0, "first_mismatch": None})

    def test_raw_vertex_mutation_is_detected(self):
        changed = bytearray(self.geometry)
        marker = b"vertex\tlevel-9-0\t"
        offset = changed.index(marker) + len(marker)
        changed[offset] = ord("1") if changed[offset] != ord("1") else ord("2")
        result = fluid_run.compare_geometry(self.native, bytes(changed))
        self.assertEqual(result["mismatch_count"], 1)
        self.assertIsNotNone(result["first_mismatch"])

    def test_missing_vertex_is_detected(self):
        rows = self.geometry.splitlines(keepends=True)
        result = fluid_run.compare_geometry(self.native, b"".join(rows[:-1]))
        self.assertEqual(result["mismatch_count"], 1)

    def test_malformed_probe_protocol_is_detected(self):
        result = fluid_run.compare_geometry(self.native, b"case\tmissing\n")
        self.assertEqual(result["mismatch_count"], 1)
        self.assertIn("invalid shape", result["first_mismatch"]["error"])

    def test_saved_report_outside_retained_tree_is_rejected(self):
        with self.assertRaises((ValueError, OSError)):
            fluid_review.verify_report(Path("/etc/passwd"))

    def test_saved_report_and_retained_logs_are_hash_bound(self):
        with tempfile.TemporaryDirectory(prefix="fluid-review-test-") as temp:
            root = Path(temp)
            run = root / "artifacts/parity/fluid/run-fixture"
            run.mkdir(parents=True)
            (root / "tests/fixtures/fluid").mkdir(parents=True)
            (root / "tests/fixtures/fluid/native.tsv").write_bytes(self.native)
            suite = fluid_corpus.SUITE.encode("ascii") + b"\n"
            (root / "tests/fixtures/fluid/input.tsv").write_bytes(suite)
            log_paths = {}
            process = {}
            for name, raw in (("native-run_output", self.native),
                              ("native-repeat_output", self.native)):
                target = run / name
                target.write_bytes(raw)
                log_paths[name] = str(target)
            processes = {}
            for label in ("native-run", "native-repeat"):
                process = {}
                for stream in ("stdout", "stderr"):
                    target = run / f"{label}.{stream}"
                    target.write_bytes(b"")
                    process[stream] = str(target)
                    process[stream + "_sha256"] = fluid_run.sha256(b"")
                process.update({"returncode": 0, "error": None, "timeout": False})
                processes[label] = process
            report = {
                "schema_version": 1, "suite": "fluid",
                "scenario_id": "FLUID-001-water-native-geometry",
                "reference_jar_sha256": fluid_corpus.JAR_SHA,
                "expected_fixture_sha256": fluid_run.sha256(self.native),
                "input_sha256": fluid_run.sha256(suite),
                "source_hashes": {}, "source_hashes_after": {},
                "processes": processes, "log_paths": log_paths,
                "reference_output_sha256": fluid_run.sha256(self.native),
                "case_count": 44, "vertex_count": 1242,
                "native_audit": {"repeatable": True},
                "reference_class_sha256": fluid_corpus.EXPECTED_CLASSES,
                "status": "not_implemented", "reason": "probe absent",
                "rust_binary_sha256": None, "rust_output_sha256": None,
            }
            report_path = run / "report.json"
            report_path.write_text(json.dumps(report), encoding="utf-8")
            with mock.patch.object(fluid_review, "ROOT", root):
                self.assertEqual(fluid_review.verify_report(report_path)["status"], "not_implemented")
                (run / "native-run_output").write_bytes(self.native.replace(b"3f500000", b"3f500001", 1))
                with self.assertRaises(ValueError):
                    fluid_review.verify_report(report_path)


if __name__ == "__main__":
    unittest.main()
