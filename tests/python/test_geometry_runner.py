"""Protocol and retained-evidence regressions independent of local game installation."""
import copy
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
import geometry_run as g
from geometry_review import retained, verify_geometry


def number(value):
    return struct.pack(">d", value).hex()


def row(name="one", op="vec_new", values=(0., -0., 1.)):
    return ("\t".join([name, op, *(number(x) for x in values)]) + "\n").encode()


class GeometryRunnerTests(unittest.TestCase):
    def test_committed_fixture_is_typed_and_audited(self):
        cases = g.parse_input((ROOT / g.FIXTURES / "input.tsv").read_bytes())
        reference = (ROOT / g.FIXTURES / "reference.tsv").read_bytes()
        self.assertGreaterEqual(len(cases), 5000)
        self.assertEqual({c["op"] for c in cases}, set(g.SPECS))
        self.assertEqual(g.compare(reference, reference, cases)["mismatch_count"], 0)
        g.validate_pinned(ROOT, json.loads((ROOT / g.FIXTURES / "audit.json").read_text()))

    def test_raw_zero_and_first_differing_record_are_preserved(self):
        cases = g.parse_input(row())
        a = b"one\t0000000000000000,0000000000000000,3ff0000000000000\n"
        b = a.replace(b"one\t0000000000000000", b"one\t8000000000000000")
        mismatch = g.compare(a, b, cases)
        self.assertEqual(mismatch["mismatch_count"], 1)
        self.assertEqual(mismatch["first_mismatch"]["case"], "one")

    def test_input_rejects_framing_bits_domain_duplicates_and_limits(self):
        invalid = [b"", row()[:-1], row() + b"\n", row() + row(),
                   row(op="unknown"), row(values=(float("nan"), 0., 0.)),
                   row(values=(float("inf"), 0., 0.)), row(values=(30_000_001., 0., 0.)),
                   row().replace(b"3ff0000000000000", b"3FF0000000000000"),
                   b"one\tvec_new\tinvalid\n", b"# comment\n", b"x" * (32 * 1024 * 1024 + 1)]
        for raw in invalid:
            with self.subTest(size=len(raw)), self.assertRaises(g.GeometryError):
                g.parse_input(raw)
        self.assertEqual(len(g.parse_input(row(values=(-30_000_000., 30_000_000., -0.)))), 1)

    def test_output_rejects_order_shape_faces_and_missing_lf(self):
        cases = g.parse_input(row(op="box_ray", values=(0.,) * 12))
        for data in (b"one\t6,0000000000000000,0000000000000000,0000000000000000\n",
                     b"other\tnone\n", b"one\tnone", b"one\t0,0,0,0\n", b"one\tnone\none\tnone\n"):
            with self.subTest(data=data), self.assertRaises(g.GeometryError):
                g.parse_output(data, cases, "test")
        self.assertEqual(g.parse_output(b"one\tnone\n", cases, "test"), ["none"])

    def test_native_audit_rejects_changed_provenance_methods_counts_and_code_source(self):
        cases = g.parse_input(row())
        classes = {name: "a" * 64 for name in ("eq", "bt", "vf", "in")}
        pinned = {"class_sha256": classes, "method_mapping": {name: {"name": name} for name in g.SPECS}}
        jar = Path("/tmp/geometry-test-client.jar")
        audit = {"schema_version": 1, "suite": "geometry", "jar_sha256": g.JAR_SHA,
                 "class_sha256": classes, "method_mapping": pinned["method_mapping"],
                 "case_count": 1, "invocations": {"vec_new": 1},
                 "code_sources": {name: jar.as_uri() for name in classes}}
        with patch.object(g, "native_hashes", return_value=classes):
            g.validate_audit(audit, cases, pinned, jar)
            for key, value in (("jar_sha256", "b" * 64), ("method_mapping", {}),
                               ("case_count", 2), ("invocations", {"vec_new": 0}),
                               ("code_sources", {name: "file:///other.jar" for name in classes})):
                bad = {**audit, key: value}
                with self.subTest(key=key), self.assertRaises(g.GeometryError):
                    g.validate_audit(bad, cases, pinned, jar)

    def test_missing_original_jar_is_error_with_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "config").mkdir()
            (root / "config/local.toml").write_text('[paths]\nartifacts="artifacts"\nclient_jar="missing.jar"\n')
            report, path = g.run_geometry_suite(root=root)
            self.assertEqual(report["status"], "error")
            self.assertTrue(path.is_file())
            self.assertIn("missing.jar", report["reason"])

    def test_saved_verifier_rejects_missing_process_and_escaping_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp) / "run"
            directory.mkdir()
            outside = Path(tmp) / "outside"
            outside.write_bytes(b"bytes")
            with self.assertRaisesRegex(g.GeometryError, "escapes"):
                retained(outside, directory)
            with self.assertRaisesRegex(SystemExit, "git-revision"):
                verify_geometry(directory / "report.json", {"suite": "geometry", "status": "pass", "log_paths": {}, "processes": {}})

    def test_source_snapshot_binds_crate_manifest_and_new_build_script(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate = root / "crates/beta-core"
            crate.mkdir(parents=True)
            manifest = crate / "Cargo.toml"
            manifest.write_text('[package]\nname="core"\n')
            original = g.source_snapshot(root)
            self.assertIn("crates/beta-core/Cargo.toml", original)
            manifest.write_text(manifest.read_text() + "# changed\n")
            self.assertNotEqual(original, g.source_snapshot(root))
            snapshot = g.source_snapshot(root)
            (crate / "build.rs").write_text("fn main() {}\n")
            self.assertNotEqual(snapshot, g.source_snapshot(root))


@unittest.skipUnless(os.environ.get("BETA_LIVE_GEOMETRY") == "1",
                     "opt-in native geometry evidence mutation test (BETA_LIVE_GEOMETRY=1)")
class LiveGeometryEvidenceTests(unittest.TestCase):
    def test_live_saved_report_and_adversarial_mutations(self):
        report, path = g.run_geometry_suite()
        self.assertEqual(report["status"], "pass", report["reason"])
        self.assertEqual(verify_geometry(path, report, protocol=True), 0)
        mutations = [lambda r: r["processes"]["rust-run"].update(returncode=1),
                     lambda r: r["processes"].pop("java-repeat"),
                     lambda r: r["processes"]["java-run"].update(command=["/bin/false"]),
                     lambda r: r.update(rust_binary_sha256="0" * 64),
                     lambda r: r.update(adapter_class_hashes={}),
                     lambda r: r.update(source_hashes={}),
                     lambda r: r.update(mismatch_count=1),
                     lambda r: r.update(operation_counts={}),
                     lambda r: r.update({"java-run-audit-sha256": "0" * 64}),
                     lambda r: r["log_paths"]["rust-run"].update(stdout="/missing")]
        for mutation in mutations:
            changed = copy.deepcopy(report)
            mutation(changed)
            with self.subTest(mutation=mutation), self.assertRaises(SystemExit):
                verify_geometry(path, changed, protocol=True)
        for name in ("rust_binary", "java-run-audit", "input"):
            artifact = Path(report["log_paths"][name])
            original = artifact.read_bytes()
            try:
                artifact.write_bytes(original + b"changed")
                with self.subTest(artifact=name), patch("geometry_review.subprocess.run") as execute:
                    with self.assertRaises(SystemExit):
                        verify_geometry(path, report, protocol=True)
                    execute.assert_not_called()
            finally:
                artifact.write_bytes(original)
        with patch("geometry_review.subprocess.run", return_value=subprocess.CompletedProcess([], 2, b"", b"")):
            with self.assertRaisesRegex(SystemExit, "omitted diagnostics"):
                verify_geometry(path, report, protocol=True)


if __name__ == "__main__":
    unittest.main()
