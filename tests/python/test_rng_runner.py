"""Synthetic comparator/process regression tests, never live parity evidence."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("rng_runner", ROOT / "tools/parity/run.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

SYNTHETIC = b"first\t0\t7\nfirst\t1\t-9\nsecond\t0\t0\n"
KEYS = [("first", 0), ("first", 1), ("second", 0)]


class ComparatorTests(unittest.TestCase):
    def test_exact_valid_output(self):
        self.assertEqual(runner.compare_outputs(SYNTHETIC, SYNTHETIC, KEYS),
                         {"mismatch_count": 0, "first_mismatch": None})

    def test_corrupt_missing_extra_reordered_and_empty(self):
        records = SYNTHETIC.splitlines(keepends=True)
        variants = [SYNTHETIC.replace(b"\t7\n", b"\t8\n"), b"".join(records[:-1]),
                    SYNTHETIC + b"second\t1\t5\n", b"".join(records[::-1]), b"",
                    SYNTHETIC.replace(b"\n", b"\r\n"), SYNTHETIC[:-1],
                    SYNTHETIC.replace(b"\t7\n", b"\t2147483648\n")]
        for actual in variants:
            with self.subTest(actual=actual):
                result = runner.compare_outputs(SYNTHETIC, actual, KEYS)
                self.assertGreater(result["mismatch_count"], 0)
                self.assertIsNotNone(result["first_mismatch"])

    def test_empty_or_invalid_equal_outputs_cannot_pass(self):
        for data in (b"", SYNTHETIC.replace(b"\t7\n", b"\t+7\n")):
            self.assertGreater(runner.compare_outputs(data, data, KEYS)["mismatch_count"], 0)

    def test_fixture_bounds_and_invalid_inputs(self):
        cases = runner.parse_fixture(b"# header\n\na\t-9223372036854775808\t1\nb\t9223372036854775807\t100000\n")
        self.assertEqual(len(cases), 2)
        for data in (b"", b" \n", b"\t\n", b"a\t0\t0\n", b"a\t0\t100001\n", b"a\t9223372036854775808\t1\n",
                     b"a\t0\t1\na\t1\t1\n", b"bad id\t0\t1\n"):
            with self.assertRaises(runner.RunError):
                runner.parse_fixture(data)


class ProcessTests(unittest.TestCase):
    def run_command(self, script, timeout=5):
        with tempfile.TemporaryDirectory() as folder:
            directory = Path(folder)
            result = runner.run_process([sys.executable, "-c", script], cwd=directory,
                                        directory=directory, label="synthetic", timeout=timeout)
            return result, Path(result["stdout"]).read_bytes(), Path(result["stderr"]).read_bytes()

    def test_exit_failure_preserves_raw_logs(self):
        result, stdout, stderr = self.run_command(
            "import sys; print('synthetic'); print('failure', file=sys.stderr); sys.exit(7)")
        self.assertEqual(result["returncode"], 7)
        self.assertEqual(stdout, b"synthetic\n")
        self.assertEqual(stderr, b"failure\n")

    def test_timeout_preserves_partial_output(self):
        result, stdout, _ = self.run_command("import time; print('synthetic', flush=True); time.sleep(5)", 0.2)
        self.assertTrue(result["timeout"])
        self.assertIsNone(result["returncode"])
        self.assertEqual(stdout, b"synthetic\n")

    def test_missing_executable_is_error(self):
        with tempfile.TemporaryDirectory() as folder:
            result = runner.run_process([str(Path(folder) / "absent")], cwd=folder,
                                        directory=Path(folder), label="missing", timeout=1)
        self.assertIsNotNone(result["error"])

    def test_unknown_group_and_strict_statuses(self):
        result = subprocess.run([sys.executable, str(ROOT / "tools/parity/run.py"), "unknown"],
                                capture_output=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        for status in ("fail", "error", "skip", "not_implemented"):
            self.assertEqual(runner.exit_code(status, True), 1)
        self.assertEqual(runner.exit_code("pass", True), 0)


class SyntheticPipelineTests(unittest.TestCase):
    """Simulated build/process data test orchestration; this is not Rust↔Java."""

    def pipeline(self, failure=None, probe=True, mutate=False, output=SYNTHETIC, timeout_label=None):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for path in (runner.ORACLE, runner.FIXTURE, runner.PROBE):
                (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / runner.ORACLE).write_text("// synthetic oracle placeholder\n")
            (root / runner.FIXTURE).write_bytes(b"first\t0\t2\nsecond\t1\t1\n")
            if probe:
                (root / runner.PROBE).write_text("// synthetic probe placeholder\n")
            fake_binary = root / "synthetic_binary"
            fake_binary.write_bytes(b"synthetic executable placeholder")

            def simulated(command, *, cwd, directory, label, timeout, input_data=None):
                stdout, stderr = b"", b""
                if label == "git-revision":
                    stdout = b"synthetic-revision\n"
                elif label.startswith("version-"):
                    stdout = b"synthetic-version\n"
                elif label == "java-build":
                    (directory / "classes/RandomOracle.class").write_bytes(b"synthetic class")
                elif label == "java-run":
                    self.assertEqual(input_data, (directory / "input.tsv").read_bytes())
                    stdout = SYNTHETIC
                elif label == "rust-build":
                    stdout = json.dumps({"reason": "compiler-artifact", "target": {"name": "rng_probe"},
                                         "executable": str(fake_binary)}).encode() + b"\n"
                elif label == "rust-run":
                    self.assertEqual(input_data, (directory / "input.tsv").read_bytes())
                    stdout = output
                    if mutate:
                        (root / runner.PROBE).write_text("// changed during execution\n")
                failed = label == failure
                if failed:
                    stderr = b"synthetic build/process failure"
                out, err = directory / f"{label}.stdout", directory / f"{label}.stderr"
                out.write_bytes(stdout)
                err.write_bytes(stderr)
                timed_out = label == timeout_label
                return {"command": command, "cwd": str(cwd), "stdout": str(out), "stderr": str(err),
                        "returncode": None if timed_out else (1 if failed else 0),
                        "timeout": timed_out, "error": "synthetic timeout" if timed_out else None}

            with patch.object(runner, "run_process", side_effect=simulated):
                report, path = runner.run_rng(root, timeout=2)
            self.assertEqual(json.loads(path.read_text()), report)
            return report

    def test_complete_synthetic_pipeline(self):
        report = self.pipeline()
        self.assertEqual(report["status"], "pass")
        self.assertEqual(report["value_count"], 3)
        self.assertEqual(report["reference_output_sha256"], report["rust_output_sha256"])

    def test_build_and_process_failures_never_pass(self):
        for label in ("java-build", "java-run", "rust-build", "rust-run"):
            with self.subTest(label=label):
                report = self.pipeline(failure=label)
                self.assertEqual(report["status"], "error")
                self.assertIn(label, report["reason"])

    def test_no_probe_executes_java_and_reports_not_implemented(self):
        report = self.pipeline(probe=False)
        self.assertEqual(report["status"], "not_implemented")
        self.assertIn("java-run", report["processes"])
        self.assertNotIn("rust-run", report["processes"])

    def test_timeouts_never_pass(self):
        for label in ("java-build", "java-run", "rust-build", "rust-run"):
            with self.subTest(label=label):
                report = self.pipeline(timeout_label=label)
                self.assertEqual(report["status"], "error")
                self.assertIn("timeout", report["reason"])

    def test_changed_sources_invalidate_result(self):
        report = self.pipeline(mutate=True)
        self.assertEqual(report["status"], "error")
        self.assertIn("changed", report["reason"])

    def test_corruption_is_fail_with_first_mismatch(self):
        report = self.pipeline(output=SYNTHETIC.replace(b"\t7\n", b"\t8\n"))
        self.assertEqual(report["status"], "fail")
        self.assertEqual(report["mismatch_count"], 1)
        self.assertEqual(report["first_mismatch"]["record"], 0)


if __name__ == "__main__":
    unittest.main()
