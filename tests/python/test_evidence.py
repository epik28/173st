"""Fast saved-evidence checks plus opt-in live report mutation regressions."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
import java_review
import math_review
from java_run import run_java_suite
from math_run import run_math


def _sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


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


class EvidenceHelperTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="evidence-unit-")
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def _process_record(self, label="probe"):
        stdout = self.root / f"{label}.stdout"
        stderr = self.root / f"{label}.stderr"
        stdout.write_bytes(b"out\n")
        stderr.write_bytes(b"")
        process = {"returncode": 0, "timeout": False, "error": None,
                   "stdout": str(stdout), "stderr": str(stderr),
                   "stdout_sha256": _sha(stdout), "stderr_sha256": _sha(stderr)}
        return {"processes": {label: process},
                "log_paths": {label: {"stdout": str(stdout), "stderr": str(stderr)}}}

    def test_math_and_java_process_records_bind_status_logs_paths_and_hashes(self):
        for module in (math_review, java_review):
            with self.subTest(module=module.__name__):
                report = self._process_record()
                self.assertIs(module._saved_processes(report, ("probe",)), report["processes"])
                report["processes"]["probe"]["returncode"] = 9
                with self.assertRaisesRegex(SystemExit, "did not complete successfully"):
                    module._saved_processes(report, ("probe",))

                report = self._process_record()
                Path(report["processes"]["probe"]["stderr"]).write_bytes(b"tampered")
                with self.assertRaisesRegex(SystemExit, "stderr log hash mismatch"):
                    module._saved_processes(report, ("probe",))

                report = self._process_record()
                report["log_paths"]["probe"]["stdout"] = "/wrong/path"
                with self.assertRaisesRegex(SystemExit, "path disagrees"):
                    module._saved_processes(report, ("probe",))

                report = self._process_record()
                report["processes"].pop("probe")
                with self.assertRaisesRegex(SystemExit, "record probe is missing"):
                    module._saved_processes(report, ("probe",))

    def test_math_and_java_artifact_hashes_and_run_path_are_bound(self):
        for module, class_name, hash_key in (
                (math_review, "MathOracle.class", "adapter_class_sha256"),
                (java_review, "JavaCompatOracle.class", "oracle_class_sha256")):
            with self.subTest(module=module.__name__):
                classes = self.root / module.__name__
                classes.mkdir()
                adapter = classes / class_name
                adapter.write_bytes(b"adapter")
                binary = self.root / f"{module.__name__}-probe"
                binary.write_bytes(b"executable")
                report = {hash_key: _sha(adapter), "rust_binary_sha256": _sha(binary)}
                log = {"java_classes": str(classes), "rust_binary": str(binary)}
                processes = {"rust-run": {"command": [str(binary)]}}
                module._verify_artifacts(report, log, processes)
                binary.write_bytes(b"changed")
                with self.assertRaisesRegex(SystemExit, "Rust executable hash mismatch"):
                    module._verify_artifacts(report, log, processes)
                binary.write_bytes(b"executable")
                adapter.write_bytes(b"changed")
                with self.assertRaisesRegex(SystemExit, "Java adapter class hash mismatch"):
                    module._verify_artifacts(report, log, processes)

    def test_revision_and_comparison_summary_are_checked(self):
        for module in (math_review, java_review):
            with self.subTest(module=module.__name__):
                revision = self.root / f"{module.__name__}.revision"
                dirty = self.root / f"{module.__name__}.dirty"
                revision.write_text("abc123\n")
                dirty.write_bytes(b"")
                report = {"rust_revision": "abc123", "rust_dirty": False}
                processes = {"git-revision": {"stdout": str(revision)},
                             "git-dirty": {"stdout": str(dirty)}}
                module._verify_revision(report, processes)
                report["rust_dirty"] = True
                with self.assertRaisesRegex(SystemExit, "revision is dirty"):
                    module._verify_revision(report, processes)
                report = {"mismatch_count": 0, "first_mismatch": None}
                module._verify_comparison_summary(report, {
                    "mismatch_count": 0, "first_mismatch": None})
                with self.assertRaisesRegex(SystemExit, "summary disagrees"):
                    module._verify_comparison_summary(report, {
                        "mismatch_count": 1, "first_mismatch": {"record": 0}})


@unittest.skipUnless(os.environ.get("BETA_LIVE_EVIDENCE") == "1",
                     "set BETA_LIVE_EVIDENCE=1 to run live math/java reports and protocol mutations")
class LiveSavedEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.math_report, cls.math_report_path = run_math()
        cls.java_report, cls.java_report_path = run_java_suite()
        if cls.math_report.get("status") != "pass":
            raise RuntimeError(f"live math report failed: {cls.math_report.get('reason')}")
        if cls.java_report.get("status") != "pass":
            raise RuntimeError(f"live Java report failed: {cls.java_report.get('reason')}")

    def _assert_rejected(self, suite, mutate, message):
        original = self.math_report_path if suite == "math" else self.java_report_path
        verifier = math_review.verify_math if suite == "math" else java_review.verify_java
        temp, report_path, report = _copy_report(original)
        try:
            mutate(report_path, report)
            with self.assertRaisesRegex(SystemExit, message):
                verifier(report_path, report, protocol=True)
        finally:
            temp.cleanup()

    def test_live_reports_pass_saved_protocol_verification(self):
        self.assertEqual(math_review.verify_math(self.math_report_path,
                                                 self.math_report, protocol=True), 0)
        self.assertEqual(java_review.verify_java(self.java_report_path,
                                                 self.java_report, protocol=True), 0)

    def test_math_rejects_process_mutations(self):
        self._assert_rejected("math", lambda p, r: r["processes"]["java-run"].update(returncode=7),
                              "process java-run did not complete")
        self._assert_rejected("math", lambda p, r: r["processes"].pop("java-build"),
                              "process record java-build is missing")
        self._assert_rejected("math", lambda p, r: Path(r["processes"]["java-run"]["stderr"]).write_bytes(b"changed"),
                              "stderr log hash mismatch")
        self._assert_rejected("math", lambda p, r: r["log_paths"]["java-run"].update(stdout="/missing"),
                              "stdout path disagrees")

    def test_math_rejects_artifact_revision_and_summary_mutations(self):
        self._assert_rejected("math", lambda p, r: Path(r["log_paths"]["rust_binary"]).write_bytes(b"bad"),
                              "Rust executable hash mismatch")
        self._assert_rejected("math", lambda p, r: (Path(r["log_paths"]["java_classes"]) /
                                                       "MathOracle.class").write_bytes(b"bad"),
                              "Java adapter class hash mismatch")
        self._assert_rejected("math", lambda p, r: r.update(mismatch_count=1),
                              "mismatch summary disagrees")
        self._assert_rejected("math", lambda p, r: r.update(rust_revision="0" * 40),
                              "git revision disagrees")

    def test_java_rejects_process_mutations(self):
        self._assert_rejected("java", lambda p, r: r["processes"]["rust-run"].update(timeout=True),
                              "process rust-run did not complete")
        self._assert_rejected("java", lambda p, r: r["processes"].pop("version-java"),
                              "process record version-java is missing")
        self._assert_rejected("java", lambda p, r: Path(r["processes"]["java-run"]["stdout"]).unlink(),
                              "stdout log is missing")
        self._assert_rejected("java", lambda p, r: r["log_paths"]["rust-run"].update(stderr="/wrong"),
                              "stderr path disagrees")

    def test_java_rejects_artifact_revision_and_summary_mutations(self):
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
