#!/usr/bin/env python3
"""Compare saved native height maps to column scans used by Rust relighting.

Read only a native capture under artifacts; keep all world-derived results there.
Run after light_saved_world_sequence_check with a saved capture file.
"""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = (ROOT / 'artifacts').resolve()
REGISTRY = ROOT / 'tests/fixtures/light_material/native.tsv'


def run():
    if len(sys.argv) != 2:
        raise SystemExit('usage: light_saved_height_check.py ARTIFACTS_NATIVE_CAPTURE')
    capture = Path(sys.argv[1]).resolve(strict=True)
    if not capture.is_relative_to(ARTIFACTS) or not capture.is_file():
        raise ValueError('capture must be under artifacts')
    opacity = [0] * 256
    for row in REGISTRY.read_text().splitlines():
        bid, _, material_opacity, _ = map(int, row.split('\t'))
        opacity[bid] = material_opacity
    heights, derived = {}, {}
    phases = []
    with capture.open(encoding='ascii') as data:
        for line in data:
            fields = line.rstrip('\n').split('\t')
            if fields[0] == 'snapshot':
                phases.append(fields[2]); heights[fields[2]] = {}; derived[fields[2]] = {}
            elif fields[0] == 'height':
                phase, x, z, h = fields[2], *map(int, fields[3:])
                heights[phase][(x, z)] = h & 255
            elif fields[0] == 'state':
                phase = fields[2]
                x, y, z, bid = map(int, fields[3:7])
                # Rust's sky_column_top intentionally excludes y=0.
                if y >= 1 and opacity[bid]:
                    key = (x, z)
                    derived[phase][key] = max(derived[phase].get(key, 0), y + 1)
    out = {'scope': 'Saved native heightmap vs Rust relight column scan; saved prestate is not independently relit.', 'phases': {}}
    for phase in phases:
        mismatches = []
        for x in range(-16, 32):
            for z in range(-16, 32):
                n, r = heights[phase].get((x, z), 0), derived[phase].get((x, z), 0)
                if n != r:
                    mismatches.append({'x': x, 'z': z, 'native_height': n, 'derived_height': r})
        out['phases'][phase] = {'columns': 2304, 'mismatch_count': len(mismatches), 'mismatches': mismatches}
    path = ARTIFACTS / 'light-saved-world' / 'height-report.json'
    path.parent.mkdir(exist_ok=True)
    path.write_text(json.dumps(out, indent=2) + '\n')
    print('saved local report:', path)
    for phase in phases:
        print(phase, 'height mismatches:', out['phases'][phase]['mismatch_count'])


if __name__ == '__main__':
    run()
