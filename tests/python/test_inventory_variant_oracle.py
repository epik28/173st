"""Reproduce item stack variant evidence from the pinned client."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
JAR = ROOT.parent / '.local/minecraft/client-b1.7.3.jar'
FIXTURE = ROOT / 'tests/fixtures/inventory_variants/native.tsv'


class InventoryVariantOracleTests(unittest.TestCase):
    def test_reproduce_native_variants(self):
        if not JAR.is_file() or not shutil.which('java') or not shutil.which('javac'):
            self.skipTest('pinned local client JAR and Java toolchain required')
        self.assertEqual(hashlib.sha256(JAR.read_bytes()).hexdigest(), 'af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c')
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['javac', '-cp', str(JAR), '-d', directory,
                            str(ROOT / 'tools/oracle/src/ViewOracle.java'),
                            str(ROOT / 'tools/oracle/src/InventoryVariantOracle.java')], check=True)
            command = ['java', '-cp', f'{directory}:{JAR}', 'InventoryVariantOracle', str(JAR)]
            for _ in range(2):
                self.assertEqual(subprocess.run(command, check=True, stdout=subprocess.PIPE).stdout, FIXTURE.read_bytes())


if __name__ == '__main__':
    unittest.main()
