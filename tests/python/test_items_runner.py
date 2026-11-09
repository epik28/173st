"""Strict ITEMS-001 typed comparator and retained report review tests."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from tools.oracle import items_corpus
from tools.parity import items_run, items_review


class ItemsRunnerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        fixtures = items_corpus.FIXTURES
        cls.native = (fixtures / "native.tsv").read_bytes()
        cls.protocol, cls.keys = items_run.normalize_stream(cls.native, native=True)

    def test_native_rows_normalize_only_display_names(self):
        self.assertEqual(items_run.compare_items(self.native, self.protocol),
                         {"mismatch_count": 0, "first_mismatch": None})
        self.assertIn(b"1,10,0,64", self.protocol)
        self.assertNotIn(b"tile.stone", self.protocol)

    def test_single_stack_field_mutation_is_detected(self):
        changed = self.protocol.replace(b"1,10,0,64", b"1,11,0,64", 1)
        self.assertEqual(items_run.compare_items(self.native, changed)["mismatch_count"], 1)

    def test_missing_and_extra_rows_are_detected(self):
        rows = self.protocol.splitlines(keepends=True)
        self.assertEqual(items_run.compare_items(self.native, b"".join(rows[:-1]))["mismatch_count"], 1)
        self.assertEqual(items_run.compare_items(self.native, self.protocol + rows[-1])["mismatch_count"], 1)

    def test_reordered_rows_are_detected(self):
        rows = self.protocol.splitlines(keepends=True)
        changed = b"".join([rows[1], rows[0], *rows[2:]])
        self.assertEqual(items_run.compare_items(self.native, changed)["mismatch_count"], 1)

    def test_wrong_types_and_noncanonical_rows_are_rejected(self):
        row = self.protocol.splitlines(keepends=True)[1]
        malformed = row.replace(b"\t0\t", b"\tfalse\t", 1)
        with self.assertRaisesRegex(ValueError, "selected index"):
            items_run.normalize_stream(malformed)
        with self.assertRaisesRegex(ValueError, "canonical LF"):
            items_run.normalize_stream(row.rstrip(b"\n") + b"\r\n")

    def test_limits_identity_count_and_damage_are_validated(self):
        for old, new in ((b"1,10,0,64", b"1,10,0,16"),
                         (b"1,10,0,64", b"1,65,0,64"),
                         (b"1,10,0,64", b"1,10,-1,64"),
                         (b"1,10,0,64", b"99,10,0,64")):
            with self.subTest(new=new), self.assertRaises(ValueError):
                items_run.normalize_stream(self.protocol.replace(old, new, 1))

    def test_slot_count_and_selected_slot_relation_are_checked(self):
        row = self.protocol.splitlines(keepends=True)[1]
        fields = row.rstrip(b"\n").split(b"\t")
        fields[5] = b"|".join(fields[5].split(b"|")[:-1])
        with self.assertRaisesRegex(ValueError, "36 main slots"):
            items_run.normalize_stream(b"\t".join(fields) + b"\n")
        fields = row.rstrip(b"\n").split(b"\t")
        fields[4] = b"-"
        with self.assertRaisesRegex(ValueError, "selected stack differs"):
            items_run.normalize_stream(b"\t".join(fields) + b"\n")

    def test_duplicate_state_and_bounds_are_rejected(self):
        row = self.protocol.splitlines(keepends=True)[0]
        with self.assertRaisesRegex(ValueError, "duplicate"):
            items_run.normalize_stream(row + row)
        oversized = b"x" * (items_run.MAX_INPUT_BYTES + 1)
        with self.assertRaisesRegex(ValueError, "byte limit"):
            items_run.normalize_stream(oversized)

    def test_native_hotbar_out_of_range_observations_are_preserved_as_empty_lookup(self):
        rows = self.protocol.splitlines(keepends=True)
        lower = next(row for row in rows if row.startswith(b"hotbar_bounds\tindex_minus_one\t"))
        upper = next(row for row in rows if row.startswith(b"hotbar_bounds\tindex_nine\t"))
        for row, expected in ((lower, b"\t-1\t-\t"), (upper, b"\t9\t-\t")):
            self.assertIn(expected, row)
            self.assertEqual(items_run.normalize_stream(row)[0], row)

    def test_native_display_name_is_required_but_omitted_from_comparison(self):
        bad = self.native.replace(b"tile.stone", b"", 1)
        self.assertEqual(items_run.compare_items(bad, self.protocol)["mismatch_count"], 1)

    def test_probe_failure_and_timeout_cannot_be_recorded_as_success(self):
        with tempfile.TemporaryDirectory(prefix="items-process-test-") as temp:
            with self.assertRaises(items_run.parity.RunError):
                items_run._record(Path(temp), {"processes": {}}, "missing", ["missing-binary"],
                                  .1, Path(temp))
            timeout_result = {"error": "timeout after 0.1s", "returncode": None,
                              "timeout": True, "stdout": str(Path(temp) / "out")}
            with mock.patch.object(items_run.parity, "run_process", return_value=timeout_result):
                with self.assertRaisesRegex(items_run.parity.RunError, "timeout"):
                    items_run._record(Path(temp), {"processes": {}}, "timeout", ["slow"],
                                      .1, Path(temp))

    def test_absent_and_wrong_client_jar_are_errors(self):
        for jar_exists, contents in ((False, b""), (True, b"wrong-jar")):
            with self.subTest(jar_exists=jar_exists), tempfile.TemporaryDirectory(prefix="items-jar-test-") as temp:
                root = Path(temp)
                (root / "config").mkdir()
                (root / "config/local.toml").write_text(
                    '[paths]\nclient_jar = "client.jar"\nartifacts = "artifacts"\n')
                if jar_exists:
                    (root / "client.jar").write_bytes(contents)
                report, path = items_run.run_items(root=root, timeout=.1)
                self.assertEqual(report["status"], "error")
                self.assertTrue(report["reason"])
                self.assertTrue(path.is_file())

    def test_retained_report_rejects_path_escape_and_hash_mutation(self):
        with self.assertRaises((ValueError, OSError)):
            items_review.verify_report(Path("/etc/passwd"))
        with tempfile.TemporaryDirectory(prefix="items-review-test-") as temp:
            root = Path(temp)
            expected_root = root / "artifacts/parity/items/run-test"
            expected_root.mkdir(parents=True)
            fixture_root = root / "tests/fixtures/items"
            fixture_root.mkdir(parents=True)
            for name in ("input.tsv", "native.tsv", "audit.json"):
                (fixture_root / name).write_bytes((items_corpus.FIXTURES / name).read_bytes())
            log_paths, processes = {}, {}
            audit = json.loads((fixture_root / "audit.json").read_text())
            for name in ("native-run", "native-repeat"):
                output_path = expected_root / f"{name}.native.tsv"
                output_path.write_bytes(self.native)
                audit_path = expected_root / f"{name}.audit.json"
                audit_path.write_text(json.dumps(audit))
                log_paths[name + "_output"] = str(output_path)
                log_paths[name + "_audit"] = str(audit_path)
                process = {}
                for stream, data in (("stdout", self.native),
                                     ("stderr", json.dumps(audit).encode())):
                    stream_path = expected_root / f"{name}.{stream}"
                    stream_path.write_bytes(data)
                    process[stream] = str(stream_path)
                    process[stream + "_sha256"] = items_run.sha256(data)
                process.update(returncode=0, error=None, timeout=False)
                processes[name] = process
            report = {"schema_version": 1, "suite": "items",
                      "scenario_id": "ITEMS-001-main-inventory-native",
                      "reference_jar_sha256": items_corpus.PINNED_JAR,
                      "expected_fixture_sha256": items_run.sha256(self.native),
                      "input_sha256": items_run.sha256((fixture_root / "input.tsv").read_bytes()),
                      "source_hashes": {}, "source_hashes_after": {}, "processes": processes,
                      "log_paths": log_paths, "reference_output_sha256": items_run.sha256(self.native),
                      "reference_class_sha256": audit["classes"], "case_count": 6, "row_count": 21,
                      "status": "not_implemented", "reason": "probe absent",
                      "native_audit": {key: audit[key] for key in
                                       ("bootstrap", "bootstrap_stdout_sha256", "direct_calls", "native_methods", "rows")}
                      | {"repeatable": True},
                      "rust_binary_sha256": None, "rust_output_sha256": None}
            path = expected_root / "report.json"
            path.write_text(json.dumps(report), encoding="utf-8")
            with mock.patch.object(items_review, "ROOT", root):
                self.assertEqual(items_review.verify_report(path)["status"], "not_implemented")
                (expected_root / "native-run.native.tsv").write_bytes(self.native.replace(b"1,10,0,64", b"1,11,0,64", 1))
                with self.assertRaises(ValueError):
                    items_review.verify_report(path)


if __name__ == "__main__":
    unittest.main()
