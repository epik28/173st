import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/light_material"
SCRIPT = ROOT / "tools/oracle/light_material_audit.py"
JAR = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"

spec = importlib.util.spec_from_file_location("light_material_audit", SCRIPT)
audit = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = audit
spec.loader.exec_module(audit)


class LightMaterialAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.output = (FIXTURE / "native.tsv").read_bytes()
        cls.manifest = json.loads((FIXTURE / "audit.json").read_text())
        cls.rows = audit.parse_registry(cls.output)

    def test_fixture_covers_registered_and_unregistered_materials(self):
        self.assertEqual(len(self.rows), 256)
        rows = {row[0]: row[1:] for row in self.rows}
        self.assertEqual(rows[1], (1, 255, 0))
        self.assertEqual(rows[8], (1, 3, 0))
        self.assertEqual(rows[9], (1, 3, 0))
        self.assertEqual(rows[18], (1, 1, 0))
        self.assertEqual(rows[31], (1, 0, 0))
        self.assertEqual(rows[39], (1, 0, 1))
        self.assertEqual(rows[50], (1, 0, 14))
        self.assertEqual(rows[51], (1, 0, 15))
        self.assertEqual(rows[104], (0, 0, 0))
        self.assertEqual(rows[255], (0, 0, 0))
        self.assertEqual(self.manifest["registry_schema"]["native_fields"], {
            "registered": "uu.m[id] != null", "opacity_raw": "uu.q[id]", "emission_raw": "uu.s[id]"
        })

    def test_parser_rejects_coverage_encoding_and_unregistered_mutations(self):
        with self.assertRaisesRegex(ValueError, "exactly 256"):
            audit.parse_registry(self.output[:-len(b"255\t0\t0\t0\n")])
        with self.assertRaisesRegex(ValueError, "ordered exactly once"):
            audit.parse_registry(self.output.replace(b"0\t0\t0\t0\n1\t1", b"1\t0\t0\t0\n1\t1", 1))
        with self.assertRaisesRegex(ValueError, "canonical decimal"):
            audit.parse_registry(self.output.replace(b"18\t1\t1\t0\n", b"018\t1\t1\t0\n", 1))
        with self.assertRaisesRegex(ValueError, "unregistered ID 104"):
            audit.parse_registry(self.output.replace(b"104\t0\t0\t0\n", b"104\t0\t1\t0\n", 1))
        with self.assertRaisesRegex(ValueError, "LF line endings"):
            audit.parse_registry(self.output.replace(b"\n", b"\r\n", 1))

    def test_evidence_validator_rejects_mutated_capture_and_hashes(self):
        source = audit.SOURCE.read_bytes()
        audit.validate_evidence([self.output, self.output], self.output, self.manifest, source, JAR)
        with self.assertRaisesRegex(ValueError, "captures differ"):
            audit.validate_evidence([self.output, self.output[:-1],], self.output, self.manifest, source, JAR)
        wrong_hash = dict(self.manifest)
        wrong_hash["source_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "source_sha256 mismatch"):
            audit.validate_evidence([self.output, self.output], self.output, wrong_hash, source, JAR)
        missing = dict(self.manifest)
        del missing["class_sha256"]
        with self.assertRaisesRegex(ValueError, "missing fields: class_sha256"):
            audit.validate_evidence([self.output, self.output], self.output, missing, source, JAR)

    def test_wrong_jar_is_rejected_without_output(self):
        with tempfile.NamedTemporaryFile() as wrong_jar:
            wrong_jar.write(b"not the pinned client JAR")
            wrong_jar.flush()
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--jar", wrong_jar.name],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
            )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"wrong or missing pinned client JAR", result.stderr)


if __name__ == "__main__":
    unittest.main()
