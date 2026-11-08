"""Regression tests for retained MATH/JAVA saved evidence integrity."""
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
from java_review import verify_java
from java_run import run_java_suite
from math_review import verify_math
from math_run import run_math


def _copy_report(report_path):
    report_path = Path(report_path).resolve()
    source_dir = report_path.parent
    temp = tempfile.TemporaryDirectory(prefix="evidence-review-")
    destination = Path(temp.name) / source_dir.name
    shutil.copytree(source_dir, destination)
    copied_report = destination / report_path.name
    report = json.loads(copied_report.read_text())

    def rebase(value):
        if isinstance(value, str):
            try:
                path = Path(value).resolve()
                return str(destination / path.relative_to(source_dir)) if path.is_relative_to(source_dir) else value
            except (OSError, ValueError):
                return value
        return value

    report["log_paths"] = {key: rebase(value) if isinstance(value, str) else {
        subkey: rebase(subvalue) for subkey, subvalue in value.items()
    } for key, value in report["log_paths"].items()}
    for process in report["processes"].values():
        for key in ("stdout", "stderr", "cwd"):
            if key in process:
                process[key] = rebase(process[key])
        process["command"] = [rebase(part) for part in process["command"]]
    copied_report.write_text(json.dumps(report, indent=2) + "\n")
    return temp, copied_report, report


class SavedEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        local = ROOT / "config/local.toml"
        cls.created_local = not local.exists()
        if cls.created_local:
            example = (ROOT / "config/local.example.toml").read_text()
            example += '\nclient_jar = "../.local/minecraft/client-b1.7.3.jar"\n'
            (ROOT / "config").mkdir(exist_ok=True)
            local.write_text(example)
        try:
            cls.math_report, cls.math_report_path = run_math()
            cls.java_report, cls.java_report_path = run_java_suite()
        except Exception:
            if cls.created_local:
                local.unlink(missing_ok=True)
            raise

    @classmethod
    def tearDownClass(cls):
        if cls.created_local:
            (ROOT / "config/local.toml").unlink(missing_ok=True)

    def _assert_rejected(self, suite, mutate, message):
        original = self.math_report_path if suite == "math" else self.java_report_path
        verifier = verify_math if suite == "math" else verify_java
        temp, report_path, report = _copy_report(original)
        try:
            mutate(report_path, report)
            with self.assertRaisesRegex(SystemExit, message):
                verifier(report_path, report, protocol=True)
        finally:
            temp.cleanup()

    def test_live_reports_pass_saved_protocol_verification(self):
        self.assertEqual(verify_math(self.math_report_path, self.math_report, protocol=True), 0)
        self.assertEqual(verify_java(self.java_report_path, self.java_report, protocol=True), 0)

    def test_math_rejects_false_process_status_missing_log_and_bad_paths(self):
        self._assert_rejected("math", lambda p, r: r["processes"]["java-run"].update(returncode=7),
                              "process java-run did not complete")
        self._assert_rejected("math", lambda p, r: r["processes"].pop("java-build"),
                              "process record java-build is missing")
        self._assert_rejected("math", lambda p, r: Path(r["processes"]["java-run"]["stderr"]).write_bytes(b"changed"),
                              "stderr log hash mismatch")
        self._assert_rejected("math", lambda p, r: r["processes"]["java-run"].update(stdout="/missing"),
                              "stdout path disagrees")
        self._assert_rejected("math", lambda p, r: r["log_paths"]["java-run"].update(stdout="/missing"),
                              "stdout path disagrees")

    def test_math_rejects_binary_adapter_revision_and_mismatch_mutations(self):
        def alter(path, report, field):
            Path(report["log_paths"][field]).write_bytes(b"mutated")

        self._assert_rejected("math", lambda p, r: alter(p, r, "rust_binary"),
                              "Rust executable hash mismatch")
        self._assert_rejected("math", lambda p, r: (Path(r["log_paths"]["java_classes"]) /
                                                       "MathOracle.class").write_bytes(b"mutated"),
                              "Java adapter class hash mismatch")
        self._assert_rejected("math", lambda p, r: r.update(mismatch_count=1),
                              "mismatch summary disagrees")
        self._assert_rejected("math", lambda p, r: r.update(rust_revision="0" * 40),
                              "git revision disagrees")

    def test_java_rejects_false_process_status_missing_logs_and_bad_paths(self):
        self._assert_rejected("java", lambda p, r: r["processes"]["rust-run"].update(timeout=True),
                              "process rust-run did not complete")
        self._assert_rejected("java", lambda p, r: r["processes"].pop("version-java"),
                              "process record version-java is missing")
        self._assert_rejected("java", lambda p, r: Path(r["processes"]["java-run"]["stdout"]).unlink(),
                              "stdout log is missing")
        self._assert_rejected("java", lambda p, r: r["log_paths"]["rust-run"].update(stderr="/wrong"),
                              "stderr path disagrees")

    def test_java_rejects_binary_adapter_revision_and_mismatch_mutations(self):
        self._assert_rejected("java", lambda p, r: Path(r["log_paths"]["rust_binary"]).write_bytes(b"bad"),
                              "Rust executable hash mismatch")
        self._assert_rejected("java", lambda p, r: (Path(r["log_paths"]["java_classes"]) /
                                                       "JavaCompatOracle.class").write_bytes(b"bad"),
                              "Java adapter class hash mismatch")
        self._assert_rejected("java", lambda p, r: r.update(first_mismatch={"record": 0}),
                              "mismatch summary disagrees")
        self._assert_rejected("java", lambda p, r: r.update(rust_dirty=True),
                              "source revision is dirty")


if __name__ == "__main__":
    unittest.main()
