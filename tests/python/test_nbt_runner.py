"""Independent wire/comparator regression tests; synthetic, not live parity."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("nbt_runner", ROOT / "tools/parity/run.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class NbtEvidenceTests(unittest.TestCase):
    def test_fixture_audit_and_all_original_types(self):
        data = (ROOT / "tests/fixtures/nbt/corpus.tsv").read_bytes()
        cases = runner.parse_nbt_fixture(data)
        audit = json.loads((ROOT / "tests/fixtures/nbt/audit.json").read_text())
        self.assertEqual(len(cases), audit["cases"])
        self.assertEqual(runner.sha256(data), audit["sha256"])
        self.assertGreaterEqual(len(cases), 5000)
        for case in cases:
            runner.canonical_nbt(case["data"])

    def test_equal_bad_hex_and_record_framing_cannot_pass(self):
        key = [("x", 0, "nbt")]
        for data in (b"", b"x\t0\ta\t4\n", b"x\t0\t0a000000\t+4\n",
                     b"x\t0\t0a000000\t4\r\n", b"x\t0\t0a000000\t4"):
            self.assertGreater(runner.compare_outputs(data, data, key)["mismatch_count"], 0)

    def test_consumed_length_and_shared_semantic_corruption_are_rejected(self):
        cases = runner.parse_nbt_fixture(b"x\t03000000000001\n")
        for data in (b"x\t0\t03000000000001\t6\n", b"x\t0\t03000000000002\t7\n",
                     b"x\t0\t03000000000001ff\t7\n"):
            with self.assertRaises(ValueError):
                runner.inspect_nbt_outputs(data, cases)

    def test_canonical_compounds_ignore_order_and_use_last_duplicate(self):
        left = bytes.fromhex("0a000001000161010100016202010001610300")
        right = bytes.fromhex("0a00000100016202010001610300")
        self.assertEqual(runner.canonical_nbt(left), runner.canonical_nbt(right))

    def test_empty_list_declared_type_is_preserved(self):
        byte_list = runner.canonical_nbt(bytes.fromhex("0900000100000000"))
        int_list = runner.canonical_nbt(bytes.fromhex("0900000300000000"))
        self.assertNotEqual(byte_list, int_list)

    def test_modified_utf8_code_units_nul_surrogates_and_malformed(self):
        for wire, units in (("0800000002c080", [0]), ("080000000100", [0]),
                            ("0800000006eda0bdedb880", [0xd83d, 0xde00])):
            self.assertEqual(runner.canonical_nbt(bytes.fromhex(wire))["value"], [8, units])
        for wire in ("0800000004f09f9880", "0800000001c2", "0800000002c278"):
            with self.assertRaises(ValueError):
                runner.canonical_nbt(bytes.fromhex(wire))

    def test_canonical_retains_ieee_bits_and_normalizes_only_java_nan_write(self):
        for original, written in (("0500007fc01234", "0500007fc00000"),
                                  ("060000fff0000000000001", "0600007ff8000000000000")):
            tree = runner.canonical_nbt(bytes.fromhex(original))
            self.assertNotEqual(tree, runner.canonical_nbt(bytes.fromhex(written)))
            self.assertEqual(runner.java_written_nbt(tree), runner.canonical_nbt(bytes.fromhex(written)))
        negative_zero = runner.canonical_nbt(bytes.fromhex("05000080000000"))
        self.assertEqual(runner.java_written_nbt(negative_zero), negative_zero)

    def test_signed_widths_and_wire_only_int_array(self):
        self.assertEqual(runner.canonical_nbt(bytes.fromhex("01000080"))["value"], [1, -128])
        tree = runner.canonical_nbt(bytes.fromhex("0b000000000002800000007fffffff"))
        self.assertEqual(tree["value"], [11, [-2**31, 2**31-1]])

    def test_unknown_truncated_negative_lengths_and_trailing_are_rejected(self):
        for wire in ("0c0000", "03000000", "070000ffffffff", "0a0000", "0a00000000",
                     "0900000000000001", "090000030000000100"):
            with self.subTest(wire=wire), self.assertRaises(ValueError):
                runner.canonical_nbt(bytes.fromhex(wire))

    def test_saved_nbt_report_dispatch_sensitivity_and_tampering(self):
        # Deliberately synthetic report/binaries: no subprocess parity is claimed.
        data = (ROOT / "tests/fixtures/nbt/corpus.tsv").read_bytes().splitlines()[0] + b"\n"
        cases = runner.parse_nbt_fixture(data)
        wire = cases[0]["data"].hex().replace("7ff8000000000001", "7ff8000000000000")
        output = f"{cases[0]['id']}\t0\t{wire}\t{len(cases[0]['data'])}\n".encode()
        trees = {"input": [{"id": cases[0]["id"], "tree": runner.canonical_nbt(cases[0]["data"])}],
                 "java": runner.inspect_nbt_outputs(output, cases), "rust": runner.inspect_nbt_outputs(output, cases)}
        with tempfile.TemporaryDirectory() as folder:
            directory = Path(folder)
            (directory / "classes").mkdir()
            files = {"input.tsv": data, "java.tsv": output, "rust.tsv": output,
                     "trees.json": json.dumps(trees).encode(), "nbt_probe": b"synthetic binary",
                     "classes/NbtOracle.class": b"synthetic adapter", "client.jar": b"synthetic jar"}
            for name, contents in files.items():
                (directory / name).write_bytes(contents)
            jar_hash = runner.sha256(files["client.jar"])
            (directory / "java.stderr").write_text(f"jar_sha256={jar_hash}\ncases=1 classes=ij,nu\n")
            report = {"status": "pass", "suite": "nbt", "source_hashes": {}, "source_hashes_after": {},
                      "reference_jar_path": str(directory / "client.jar"), "reference_jar_sha256": jar_hash,
                      "reference_invocations": {"named_reads": 1, "named_writes": 1, "classes": ["ij", "nu"], "jar_sha256": jar_hash},
                      "processes": {label: {"returncode": 0, "error": None, "timeout": False}
                                    for label in ("java-run", "rust-run", "java-build", "rust-build")},
                      "log_paths": {"input": str(directory / "input.tsv"), "canonical_trees": str(directory / "trees.json"),
                                    "java-run": {"stdout": str(directory / "java.tsv"), "stderr": str(directory / "java.stderr")},
                                    "rust-run": {"stdout": str(directory / "rust.tsv")}}}
            for key, name in (("input_sha256", "input.tsv"), ("reference_output_sha256", "java.tsv"),
                              ("rust_output_sha256", "rust.tsv"), ("canonical_trees_sha256", "trees.json"),
                              ("rust_binary_sha256", "nbt_probe"), ("adapter_class_sha256", "classes/NbtOracle.class")):
                report[key] = runner.sha256(files[name])
            path = directory / "report.json"

            def verify():
                path.write_text(json.dumps(report))
                return subprocess.run([sys.executable, str(ROOT / "tools/parity/verify_report.py"), str(path)],
                                      capture_output=True, timeout=10)

            self.assertEqual(verify().returncode, 0)
            mutations = json.loads((directory / "nbt-mutations.json").read_text())["mutations"]
            self.assertEqual(len(mutations), 5)
            self.assertTrue(all(item["comparison"]["mismatch_count"] == 1 for item in mutations))
            self.assertEqual((directory / "rust.tsv").read_bytes(), output)
            report["processes"]["java-run"]["timeout"] = True
            self.assertNotEqual(verify().returncode, 0)
            report["processes"]["java-run"]["timeout"] = False
            (directory / "trees.json").write_text("{}")
            report["canonical_trees_sha256"] = runner.sha256(b"{}")
            self.assertNotEqual(verify().returncode, 0)


if __name__ == "__main__":
    unittest.main()
