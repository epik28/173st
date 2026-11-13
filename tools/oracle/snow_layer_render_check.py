#!/usr/bin/env python3
"""Reproduce native snow_layer geometry and visibility without a world or GL context."""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR = (ROOT.parent / '.local/minecraft/client-b1.7.3.jar').resolve()
SOURCE = ROOT / 'tools/oracle/src'
FIXTURE = ROOT / 'tests/fixtures/snow_layer_render'

with tempfile.TemporaryDirectory(prefix='snow_layer-render-') as temp:
    classes = ('SnowLayerRenderOracle', 'SnowLayerVisibilityOracle')
    subprocess.run(['javac', '-cp', str(JAR), '-d', temp,
                    *[str(SOURCE / f'{name}.java') for name in classes]], check=True)
    for name, fixture in zip(classes, ('native.tsv', 'visibility.tsv')):
        command = ['java', '-XX:-UsePerfData', '-cp', f'{temp}:{JAR}', name, str(JAR)]
        a, b = [subprocess.run(command, stdout=subprocess.PIPE, check=True).stdout for _ in range(2)]
        if a != b or a != (FIXTURE / fixture).read_bytes():
            raise SystemExit(f'native {name} capture mismatch')
print('PASS: independent native snow_layer geometry and visibility captures match fixtures')
