"""Offline COLOR-001 fixture checks and opt-in native re-execution."""

import os
import base64
import hashlib
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
ORACLE = ROOT / "tools/oracle/src/ColorOracle.java"
GENERATOR = ROOT / "tools/oracle/color_corpus.py"
FIXTURES = ROOT / "tests/fixtures/color"
DEFAULT_JAR = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"


def normalize_code_source(audit):
    return re.sub(rb"code_source=[^\t]+", b"code_source=<configured-original-jar>", audit)


class ColorFixtureTests(unittest.TestCase):
    def test_deterministic_input_corpus_and_layer_counts(self):
        generated = subprocess.run([sys.executable, str(GENERATOR)], capture_output=True, check=False, timeout=10)
        self.assertEqual(generated.returncode, 0, generated.stderr.decode(errors="replace"))
        cases = (FIXTURES / "cases.tsv").read_bytes()
        self.assertEqual(generated.stdout, cases)
        rows = [line.split("\t") for line in cases.decode("ascii").splitlines()]
        climate = [row for row in rows if row[1] == "climate"]
        palette = [row for row in rows if row[1] == "palette"]
        self.assertEqual((len(climate), len(palette)), (1280, 320))
        self.assertEqual({int(row[2]) for row in climate}, {0, 1, -1, -(1 << 63), (1 << 63) - 1})
        self.assertEqual(len({row[0] for row in rows}), 1600)

    def test_two_native_snapshots_and_audits_are_byte_identical(self):
        first = (FIXTURES / "native.tsv").read_bytes()
        repeat = (FIXTURES / "native-repeat.tsv").read_bytes()
        audit = (FIXTURES / "native.audit").read_bytes()
        repeat_audit = (FIXTURES / "native-repeat.audit").read_bytes()
        self.assertEqual(first, repeat)
        self.assertEqual(audit, repeat_audit)
        rows = first.decode("ascii").splitlines()
        self.assertEqual(len(rows), 1600)
        self.assertIn(b"xv_explicit_samples=1280", audit)
        self.assertIn(b"wp_b=1280", audit)
        self.assertIn(b"bk_b=5120", audit)
        self.assertIn(b"ia_explicit_DD=320", audit)
        self.assertIn(b"jh_explicit_DD=320", audit)
        self.assertIn(b"native_world_constructors=0", audit)
        self.assertIn(b"synthetic_fd_ctor_calls=0", audit)
        self.assertIn(b"manager_ctor_calls=5", audit)
        self.assertIn(b"counts_scope=oracle_direct_calls_only", audit)
        self.assertIn(b"bootstrap_base64=MTYgYWNoaWV2ZW1lbnRzCjE1MSByZWNpcGVzCg==", audit)
        bootstrap = (FIXTURES / "bootstrap.txt").read_bytes()
        self.assertEqual(base64.b64decode(audit.split(b"bootstrap_base64=", 1)[1].strip()), bootstrap)
        self.assertIn(b"bootstrap_sha=" + hashlib.sha256(bootstrap).hexdigest().encode("ascii"), audit)


@unittest.skipUnless(os.environ.get("BETA_LIVE_COLOR") == "1", "set BETA_LIVE_COLOR=1 to run the native color oracle")
class LiveNativeColorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.jar = Path(os.environ.get("MINECRAFT_BETA_173_JAR", DEFAULT_JAR))
        if shutil.which("javac") is None or shutil.which("java") is None:
            raise AssertionError("BETA_LIVE_COLOR=1 but javac/java is unavailable")
        if not cls.jar.is_file():
            raise AssertionError(f"BETA_LIVE_COLOR=1 but pinned client JAR is missing: {cls.jar}")
        cls.temp = tempfile.TemporaryDirectory(prefix="color-native-")
        cls.classes = Path(cls.temp.name) / "classes"
        cls.classes.mkdir()
        cls.grass = Path(cls.temp.name) / "grasscolor.png"
        cls.foliage = Path(cls.temp.name) / "foliagecolor.png"
        with zipfile.ZipFile(cls.jar) as archive:
            cls.grass.write_bytes(archive.read("misc/grasscolor.png"))
            cls.foliage.write_bytes(archive.read("misc/foliagecolor.png"))
        compile_result = subprocess.run(
            ["javac", "-cp", str(cls.jar), "-d", str(cls.classes), str(ORACLE)],
            capture_output=True, check=False, timeout=30,
        )
        if compile_result.returncode:
            cls.temp.cleanup()
            raise AssertionError(f"ColorOracle javac failed: {compile_result.stderr.decode(errors='replace')}")

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "temp"):
            cls.temp.cleanup()

    def run_oracle(self, input_data, jar=None):
        return subprocess.run(
            ["java", "-cp", os.pathsep.join((str(self.classes), str(self.jar))), "ColorOracle",
             str(jar or self.jar), str(self.grass), str(self.foliage)],
            input=input_data, capture_output=True, check=False, timeout=60,
        )

    def test_two_independent_jvms_match_native_fixture_and_audit(self):
        cases = (FIXTURES / "cases.tsv").read_bytes()
        first = self.run_oracle(cases)
        second = self.run_oracle(cases)
        for result in (first, second):
            self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        self.assertEqual(first.stdout, second.stdout)
        self.assertEqual(first.stderr, second.stderr)
        self.assertEqual(first.stdout, (FIXTURES / "native.tsv").read_bytes())
        self.assertEqual(normalize_code_source(first.stderr), (FIXTURES / "native.audit").read_bytes())

    def test_invalid_late_row_is_rejected_without_partial_stdout(self):
        data = b"valid\tclimate\t0\t0\t0\nlate\tpalette\t7ff8000000000000\t3fe0000000000000\n"
        result = self.run_oracle(data)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"nonfinite palette climate", result.stderr)

    def test_wrong_client_jar_is_rejected_without_stdout(self):
        result = self.run_oracle(b"", self.grass)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"wrong client JAR SHA-256", result.stderr)


if __name__ == "__main__":
    unittest.main()
