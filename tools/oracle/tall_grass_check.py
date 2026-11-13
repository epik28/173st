#!/usr/bin/env python3
"""Reproduce native tall_grass geometry without opening a world or GL context."""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR = (ROOT.parent / '.local/minecraft/client-b1.7.3.jar').resolve()
SOURCE = ROOT / 'tools/oracle/src/TallGrassOracle.java'
FIXTURE = ROOT / 'tests/fixtures/tall_grass/native.tsv'

with tempfile.TemporaryDirectory(prefix='tall_grass-render-') as temp:
    subprocess.run(['javac', '-cp', str(JAR), '-d', temp, str(SOURCE)], check=True)
    command = ['java', '-XX:-UsePerfData', '-cp', f'{temp}:{JAR}', 'TallGrassOracle', str(JAR)]
    a, b = [subprocess.run(command, stdout=subprocess.PIPE, check=True).stdout for _ in range(2)]
    if a != b or a != FIXTURE.read_bytes():
        raise SystemExit('native tall_grass capture mismatch')
print('PASS: two independent native tall_grass captures match fixture')
