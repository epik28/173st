"""Audit the metadata-zero native registry inventory against the pinned client."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
JAR = ROOT.parent / '.local/minecraft/client-b1.7.3.jar'
FIXTURE = ROOT / 'tests/fixtures/view/block-registry-meta0.tsv'
JAR_SHA = 'af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c'


class BlockRegistryOracleTests(unittest.TestCase):
    def test_fixture_schema_and_context_marker(self):
        lines = FIXTURE.read_text().splitlines()
        self.assertEqual(lines[0], '# jar_sha256\t' + JAR_SHA)
        rows = [line.split('\t') for line in lines if not line.startswith('#')]
        ids = [int(row[0]) for row in rows]
        self.assertEqual(ids, sorted(set(ids)))
        self.assertEqual(len(rows), 96)
        for row in rows:
            self.assertEqual(len(row), 7)
            self.assertEqual(len(row[2]), 64)
            self.assertIn(row[4], ('0', '1'))
            self.assertIn(row[5], ('0', '1'))
            self.assertEqual(len(row[6].split(',')), 6)
        grass = next(row for row in rows if row[0] == '2')
        self.assertEqual(grass[5:], ['1', '2,0,3,3,3,3'])

    def test_reproduce_against_pinned_client(self):
        if not JAR.is_file() or not shutil.which('javac') or not shutil.which('java'):
            self.skipTest('pinned local client JAR and Java toolchain required')
        self.assertEqual(hashlib.sha256(JAR.read_bytes()).hexdigest(), JAR_SHA)
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['javac', '-cp', str(JAR), '-d', directory,
                            str(ROOT / 'tools/oracle/src/ViewOracle.java'),
                            str(ROOT / 'tools/oracle/src/BlockRegistryOracle.java')], check=True)
            result = subprocess.run(['java', '-cp', f'{directory}:{JAR}',
                                     'BlockRegistryOracle', str(JAR)],
                                    check=True, stdout=subprocess.PIPE)
        self.assertEqual(result.stdout, FIXTURE.read_bytes())

    def test_reproduce_cube_bounds_alpha_and_visibility(self):
        terrain = ROOT.parent / '.local/minecraft/terrain.png'
        if not JAR.is_file() or not terrain.is_file() or not shutil.which('javac') or not shutil.which('java'):
            self.skipTest('pinned local client JAR, terrain and Java toolchain required')
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['javac', '-cp', str(JAR), '-d', directory,
                            str(ROOT / 'tools/oracle/src/ViewOracle.java'),
                            str(ROOT / 'tools/oracle/src/BlockCubeOracle.java')], check=True)
            command = ['java', '-cp', f'{directory}:{JAR}', 'BlockCubeOracle', str(JAR), str(terrain)]
            expected = (ROOT / 'tests/fixtures/view/block-cubes-meta0.tsv').read_bytes()
            for _ in range(2):
                self.assertEqual(subprocess.run(command, check=True, stdout=subprocess.PIPE).stdout, expected)

    def test_reproduce_metadata_cubes(self):
        terrain = ROOT.parent / '.local/minecraft/terrain.png'
        if not JAR.is_file() or not terrain.is_file() or not shutil.which('javac') or not shutil.which('java'):
            self.skipTest('pinned local client JAR, terrain and Java toolchain required')
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['javac', '-cp', str(JAR), '-d', directory,
                            str(ROOT / 'tools/oracle/src/ViewOracle.java'),
                            str(ROOT / 'tools/oracle/src/BlockMetadataCubeOracle.java')], check=True)
            command = ['java', '-cp', f'{directory}:{JAR}', 'BlockMetadataCubeOracle', str(JAR), str(terrain)]
            expected = (ROOT / 'tests/fixtures/view/block-metadata-cubes.tsv').read_bytes()
            for _ in range(2):
                self.assertEqual(subprocess.run(command, check=True, stdout=subprocess.PIPE).stdout, expected)


if __name__ == '__main__':
    unittest.main()
