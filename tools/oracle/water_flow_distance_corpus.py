#!/usr/bin/env python3
"""Freeze and verify original-client downhill water scenes."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from water_tick_corpus import configured_jar, parse_native, JAR_SHA

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'tools/oracle/src/WaterFlowDistanceOracle.java'
FIXTURE = ROOT / 'tests/fixtures/water_flow_distance'
CASES = ['east-two', 'north-two', 'corner-two', 'corner-block-east', 'east-three-turn', 'east-three-block-east', 'east-three-block-south']

def sha(data):
    return hashlib.sha256(data).hexdigest()

def run(write=False):
    jar = configured_jar()
    if sha(jar.read_bytes()) != JAR_SHA:
        raise ValueError('wrong pinned client')
    with tempfile.TemporaryDirectory(prefix='water-flow-') as d:
        subprocess.run(['javac', '-Xlint:none', '-cp', str(jar), '-d', d, str(SOURCE)], check=True, capture_output=True)
        def capture():
            return subprocess.run(['java', '-XX:-UsePerfData', '-cp', f'{d}:{jar}', 'WaterFlowDistanceOracle', str(jar)], check=True, capture_output=True).stdout
        first, second = capture(), capture()
    if first != second:
        raise ValueError('native runs differ')
    data = parse_native(first, CASES)
    audit = {k:data[k] for k in ('case_count','input_sha256','native_sha256','events')}
    audit.update(jar_sha256=JAR_SHA, oracle_sha256=sha(SOURCE.read_bytes()), captures=2, repeat_identical=True)
    inputs = ('\n'.join(line for line in first.decode('ascii').splitlines() if line.startswith(('case\t','input\t'))) + '\n').encode('ascii')
    if write:
        FIXTURE.mkdir(parents=True, exist_ok=True)
        (FIXTURE / 'native.tsv').write_bytes(first)
        (FIXTURE / 'input.tsv').write_bytes(inputs)
        (FIXTURE / 'audit.json').write_text(json.dumps(audit, indent=2, sort_keys=True)+'\n')
    elif ((FIXTURE / 'native.tsv').read_bytes() != first or (FIXTURE / 'input.tsv').read_bytes() != inputs or json.loads((FIXTURE / 'audit.json').read_text()) != audit):
        raise ValueError('fixture differs from live original')
    print(json.dumps(audit, indent=2))

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument('--write', action='store_true')
    run(parser.parse_args().write)
