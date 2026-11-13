#!/usr/bin/env python3
"""Reproduce native plant geometry without opening a world or GL context."""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR = (ROOT.parent / '.local/minecraft/client-b1.7.3.jar').resolve()
SOURCE = ROOT / 'tools/oracle/src/PlantRenderOracle.java'
FIXTURE = ROOT / 'tests/fixtures/plant_render/native.tsv'

with tempfile.TemporaryDirectory(prefix='plant-render-') as temp:
    subprocess.run(['javac', '-cp', str(JAR), '-d', temp, str(SOURCE)], check=True)
    command = ['java', '-XX:-UsePerfData', '-cp', f'{temp}:{JAR}', 'PlantRenderOracle', str(JAR)]
    a, b = [subprocess.run(command, stdout=subprocess.PIPE, check=True).stdout for _ in range(2)]
    if a != b or a != FIXTURE.read_bytes():
        raise SystemExit('native plant capture mismatch')
print('PASS: two independent native plant captures match fixture')
