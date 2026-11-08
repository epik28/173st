import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/light"


def generated():
    return subprocess.check_output(["python3", str(ROOT / "tools/oracle/light_corpus.py")])


class LightFixtureTests(unittest.TestCase):
    def test_corpus_is_bounded_complete_and_unique(self):
        data = generated()
        rows = data.decode("ascii").splitlines()
        self.assertLessEqual(len(rows), 100_000)
        self.assertLessEqual(len(data), 16 * 1024 * 1024)
        ids = [row.split("\t", 1)[0] for row in rows]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertTrue(data.endswith(b"\n"))
        samples = [r.split("\t") for r in rows if "\tsample\t" in r]
        triples = {(int(r[3]), int(r[4]), int(r[5])) for r in samples if r[2] == "0"}
        self.assertEqual(len(triples), 4096)
        edge_rows = [r.split("\t") for r in rows if r.startswith("edge-time-")]
        for dimension in ("0", "-1"):
            self.assertEqual(sum(r[1] == "time" and r[2] == dimension for r in edge_rows), 32)
        self.assertEqual(sum(r.startswith("emit-") for r in ids), 256)
        ticks = [r.split("\t") for r in rows if r.startswith("ow-cycle-")]
        self.assertEqual(len(ticks), 48_000)
        self.assertEqual((ticks[0][3], ticks[0][4], ticks[-1][3], ticks[-1][4]), ("0", "00000000", "23999", "3f800000"))

    def test_saved_fixture_is_well_formed_and_retains_raw_levels(self):
        data = (FIXTURE / "reference.tsv").read_bytes()
        self.assertTrue(data.endswith(b"\n"))
        lines = data.decode("ascii").splitlines()
        self.assertGreater(len(lines), 50_000)
        for line in lines:
            cols = line.split("\t")
            if cols[1] == "sample":
                self.assertRegex(cols[2], r"0|[1-9][0-9]*")
                self.assertRegex(cols[3], r"[0-9a-f]{8}")
        audit = (FIXTURE / "audit.json").read_text()
        self.assertIn("bootstrap_base64", audit)
        self.assertIn("code_source", audit)
        self.assertIn("\"fd_a_IIII\"", audit)
        self.assertNotIn("/home/andrew/", audit)

    def test_all_native_brightness_table_entries_and_emission_edges(self):
        rows = [line.split("\t") for line in (FIXTURE / "reference.tsv").read_text().splitlines()]
        result = {r[0]: r for r in rows}
        expected = {
            0: ["3d4ccccd", "3d888888", "3dae7592", "3dd8d8d8", "3e044444", "3e1f49f6", "3e3e2be1", "3e61ce1e", "3e85b05c", "3e9e412b", "3ebbbbbc", "3edfc353", "3f066667", "3f235a36", "3f49f49f", "3f800000"],
            -1: ["3dcccccd", "3ded2308", "3e088888", "3e1c9c9c", "3e333333", "3e4cccce", "3e6a0ea0", "3e85e85f", "3e99999a", "3eb0df6c", "3ecccccd", "3eeeeeef", "3f0cccce", "3f283a84", "3f4ccccc", "3f800000"],
        }
        for dimension, table in expected.items():
            for level, bits in enumerate(table):
                self.assertEqual(result[f"table-{dimension}-{level}"][3], bits)
        self.assertEqual(result["emit-089"][2], "15")
        self.assertEqual(result["emit-091"][2], "15")


@unittest.skipUnless(os.environ.get("BETA_LIVE_LIGHT") == "1", "set BETA_LIVE_LIGHT=1 for original-JAR execution")
class LiveLightCaptureTests(unittest.TestCase):
    def test_original_native_rows_match_committed_snapshot(self):
        jar = pathlib.Path(os.environ.get("BETA_CLIENT_JAR", ROOT.parent / ".local/minecraft/client-b1.7.3.jar"))
        if not jar.is_file():
            self.fail(f"BETA_LIVE_LIGHT=1 but pinned client JAR is missing: {jar}")
        javac, java = shutil.which("javac"), shutil.which("java")
        if not javac or not java:
            self.fail("BETA_LIVE_LIGHT=1 requires javac and java")
        expected_input = (FIXTURE / "input.tsv").read_bytes()
        self.assertEqual(expected_input, generated())
        with tempfile.TemporaryDirectory(prefix="light-native-") as temp:
            classes = pathlib.Path(temp) / "classes"
            classes.mkdir()
            subprocess.run([javac, "-cp", str(jar), "-d", str(classes), str(ROOT / "tools/oracle/src/LightOracle.java")], check=True)
            audit_path = pathlib.Path(temp) / "audit.json"
            result = subprocess.run([java, "-cp", f"{classes}{os.pathsep}{jar}", "LightOracle", str(jar), str(audit_path)], input=expected_input, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
            self.assertEqual(result.stdout, (FIXTURE / "reference.tsv").read_bytes())
            import json
            actual = json.loads(audit_path.read_text())
            expected = json.loads((FIXTURE / "audit.json").read_text())
            actual["code_source"] = "<configured-original-jar>"
            self.assertEqual(actual, expected)


if __name__ == "__main__":
    unittest.main()
