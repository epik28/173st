#!/usr/bin/env python3
"""Reproduce native grass/stone lighting capture and compare Rust cell states.

Run from any directory: python3 tools/oracle/light_grass_check.py
The full capture is held in memory and is not stored in the repository.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR = ROOT.parent / '.local/minecraft/client-b1.7.3.jar'
FIXTURE = ROOT / 'tests/fixtures/light_grass/native_summary.json'
SOURCE = ROOT / 'tools/oracle/src'
REQUEST = b'material-edits-v2\n'


def run():
    expected = json.loads(FIXTURE.read_text())
    jar = JAR.resolve(strict=True)
    if hashlib.sha256(jar.read_bytes()).hexdigest() != expected['jar_sha256']:
        raise ValueError('pinned Beta 1.7.3 client JAR SHA-256 mismatch')
    with tempfile.TemporaryDirectory(prefix='light-grass-') as classes:
        subprocess.run(['javac', '-cp', str(jar), '-d', classes,
                        str(SOURCE / 'LightUpdateOracle.java'),
                        str(SOURCE / 'LightMaterialEditV2Oracle.java'),
                        str(SOURCE / 'LightGrassPlacementOracle.java')], check=True)
        command = ['java', '-XX:-UsePerfData', '-cp', f'{classes}:{jar}', 'LightGrassPlacementOracle', str(jar)]
        captures = [subprocess.run(command, input=REQUEST, stdout=subprocess.PIPE,
                                   check=True).stdout for _ in range(2)]
        if captures[0] != captures[1]:
            raise ValueError('native captures differ across independent JVM runs')
        capture = captures[0]
        if hashlib.sha256(capture).hexdigest() != expected['capture_sha256']:
            raise ValueError('native full capture SHA-256 mismatch')
        rows = [line for line in capture.decode('ascii').splitlines()
                if line.startswith(('snapshot\t', 'sample\t'))]
        if rows != expected['rows']:
            raise ValueError('native snapshot/sample rows mismatch')
        probe = subprocess.run(['cargo', 'run', '--quiet', '--locked', '--offline',
                                '-p', 'beta-core', '--example', 'light_snapshot_probe'],
                               cwd=ROOT, input=capture, stdout=subprocess.PIPE, check=True)
        if probe.stdout.decode('ascii').splitlines() != expected['probe_rows']:
            raise ValueError('Rust/native cell comparison mismatch')
    print('PASS: two independent native captures match pinned summary; Rust matches both edit phases')
    print(expected['scope'])


if __name__ == '__main__':
    run()
