"""Headless native UV capture regression; a missing JAR is an explicit skip."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
ORACLE = ROOT / "tools/oracle/src/UvOracle.java"
FIXTURES = ROOT / "tests/fixtures/uv"
DEFAULT_JAR = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
DEFAULT_LWJGL = Path.home() / ".local/share/PrismLauncher/libraries/org/lwjgl/lwjgl/lwjgl/2.9.4-nightly-20150209/lwjgl-2.9.4-nightly-20150209.jar"


class NativeUvCaptureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.jar = Path(os.environ.get("MINECRAFT_BETA_173_JAR", DEFAULT_JAR))
        cls.lwjgl = Path(os.environ.get("LWJGL2_JAR", DEFAULT_LWJGL))
        if shutil.which("javac") is None or shutil.which("java") is None:
            raise unittest.SkipTest("Java toolchain unavailable; native comparison was not run")
        if not cls.jar.is_file() or not cls.lwjgl.is_file():
            raise unittest.SkipTest("pinned client JAR or LWJGL2 helper JAR unavailable; native comparison was not run")
        cls.temp = tempfile.TemporaryDirectory(prefix="uv-native-")
        cls.classes = Path(cls.temp.name) / "classes"
        cls.classes.mkdir()
        compile_result = subprocess.run(
            ["javac", "-cp", os.pathsep.join((str(cls.jar), str(cls.lwjgl))), "-d", str(cls.classes), str(ORACLE)],
            capture_output=True, check=False, timeout=30,
        )
        if compile_result.returncode:
            cls.temp.cleanup()
            raise AssertionError(f"UvOracle javac failed: {compile_result.stderr.decode(errors='replace')}")

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "temp"):
            cls.temp.cleanup()

    def run_oracle(self, data):
        return subprocess.run(
            ["java", "-cp", os.pathsep.join((str(self.classes), str(self.jar), str(self.lwjgl))),
             "UvOracle", str(self.jar)],
            input=data, capture_output=True, check=False, timeout=30,
        )

    def test_captured_native_vertices_match_checked_in_fixture(self):
        result = self.run_oracle((FIXTURES / "cases.tsv").read_bytes())
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        self.assertEqual(result.stdout, (FIXTURES / "native.tsv").read_bytes())

    def test_face_fixture_has_six_submissions_and_four_consistent_corners(self):
        groups = {}
        for line in (FIXTURES / "native.tsv").read_text(encoding="ascii").splitlines():
            if not line or line.startswith("#"):
                continue
            case_id, corner, *bits = line.split("\t")
            self.assertEqual(len(bits), 5)
            groups.setdefault(case_id, []).append((int(corner), tuple(bits)))
        self.assertEqual(len(groups), 84)
        for case_id, records in groups.items():
            with self.subTest(case=case_id):
                self.assertEqual([corner for corner, _ in records], list(range(6)))
                by_position = {}
                for _, bits in records:
                    position = bits[:3]
                    uv = bits[3:]
                    if position in by_position:
                        self.assertEqual(by_position[position], uv)
                    else:
                        by_position[position] = uv
                self.assertEqual(len(by_position), 4)

    def test_malformed_input_is_rejected(self):
        result = self.run_oracle(b"bad\t6\t20\t0\t0\t0\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"out of range", result.stderr)


if __name__ == "__main__":
    unittest.main()
