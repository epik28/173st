"""Audit the original GUI cube renderer through recording GL/tessellator stubs."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
JAR = ROOT.parent / '.local/minecraft/client-b1.7.3.jar'
FIXTURE = ROOT / 'tests/fixtures/inventory_cube/native.tsv'


class InventoryCubeOracleTests(unittest.TestCase):
    def test_fixture_has_six_quads_per_case(self):
        lines = FIXTURE.read_text().splitlines()
        cases = [line for line in lines if line.startswith('case\t')]
        self.assertEqual(len(cases), 7)
        self.assertEqual(sum(line.startswith('vertex\t') for line in lines), 7 * 24)
        self.assertEqual(sum(line.startswith('normal\t') for line in lines), 7 * 6)
        self.assertEqual(sum(line.startswith('color\t') for line in lines), 7 * 2)

    def test_reproduce_native_cube_capture(self):
        if not JAR.is_file() or not shutil.which('java') or not shutil.which('javac'):
            self.skipTest('pinned local client JAR and Java toolchain required')
        stub = ROOT / 'tools/oracle/inventory_cube_stub'
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['javac', '-cp', str(JAR), '-d', directory,
                            str(stub / 'nw.java'), str(stub / 'ji.java'),
                            str(stub / 'org/lwjgl/opengl/GL11.java'),
                            str(ROOT / 'tools/oracle/src/ViewOracle.java'),
                            str(ROOT / 'tools/oracle/src/InventoryCubeOracle.java')], check=True)
            command = ['java', '-cp', f'{directory}:{JAR}', 'InventoryCubeOracle', str(JAR)]
            for _ in range(2):
                self.assertEqual(subprocess.run(command, check=True, stdout=subprocess.PIPE).stdout, FIXTURE.read_bytes())


if __name__ == '__main__':
    unittest.main()
