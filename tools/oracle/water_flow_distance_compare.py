#!/usr/bin/env python3
"""Compare every saved downhill native transcript with Rust's transition."""
import subprocess
from pathlib import Path
from water_flow_distance_corpus import CASES, FIXTURE
from water_tick_corpus import parse_native
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'parity'))
from water_tick_run import _inputs, _expected

root = Path(__file__).resolve().parents[2]
subprocess.run(['cargo', 'build', '--quiet', '--locked', '--offline', '-p', 'beta-core', '--example', 'water_tick_probe'], cwd=root, check=True)
fixture = parse_native((FIXTURE / 'native.tsv').read_bytes(), CASES)
for case in fixture['cases']:
    previous = None
    for step in case['steps']:
        actual = subprocess.run([str(root / 'target/debug/examples/water_tick_probe')], input=_inputs(case, step, previous), capture_output=True, check=True)
        if actual.stderr or actual.stdout != _expected(step):
            raise AssertionError(f"native mismatch: {case['name']}\n{actual.stderr.decode()}\n{actual.stdout.decode()}")
        previous = step['state']
print(f"pass: {len(fixture['cases'])} native downhill transcripts")
