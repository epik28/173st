#!/usr/bin/env python3
"""Compare one saved-world edit; input must be a pinned copy inside artifacts.

Pass a pinned copy in artifacts as the argument. The matching summary is stored
locally in artifacts/light_saved_world/summary.json and is not part of Git.
Prism files are never opened. Full captured world state remains under ignored artifacts.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = (ROOT / 'artifacts').resolve()
JAR = (ROOT.parent / '.local/minecraft/client-b1.7.3.jar').resolve()
SOURCE = ROOT / 'tools/oracle/src'
# This summary contains facts derived from the user's save and stays local.
FIXTURE = ARTIFACTS / 'light_saved_world/summary.json'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run():
    if len(sys.argv) != 2:
        raise SystemExit('usage: light_saved_world_check.py ARTIFACTS_SOURCE_COPY')
    expected = json.loads(FIXTURE.read_text())
    source = Path(sys.argv[1]).resolve(strict=True)
    if not source.is_dir() or not source.is_relative_to(ARTIFACTS) or source == ARTIFACTS:
        raise ValueError('source must be an existing copied world directory under artifacts')
    if sha(JAR) != expected['jar_sha256']:
        raise ValueError('wrong client JAR')
    for name, digest in expected['source_region_sha256'].items():
        path = source / name
        if path.is_symlink() or not path.is_file() or path.resolve().parent != source or sha(path) != digest:
            raise ValueError(f'missing or changed source copy: {name}')
    with tempfile.TemporaryDirectory(prefix='light-saved-world-', dir=ARTIFACTS) as temp:
        run_dir = Path(temp)
        classes, inputs = run_dir / 'classes', run_dir / 'input'
        classes.mkdir(); inputs.mkdir()
        for name in expected['source_region_sha256']:
            shutil.copyfile(source / name, inputs / name)
        subprocess.run(['javac', '-cp', str(JAR), '-d', str(classes),
                        str(SOURCE / 'LightUpdateOracle.java'),
                        str(SOURCE / 'LightMaterialEditV2Oracle.java'),
                        str(SOURCE / 'LightSavedWorldOracle.java')], check=True)
        command = ['java', '-cp', f'{classes}:{JAR}', 'LightSavedWorldOracle', str(JAR), str(inputs)]
        captures = [subprocess.run(command, stdout=subprocess.PIPE, check=True).stdout for _ in range(2)]
        if captures[0] != captures[1] or hashlib.sha256(captures[0]).hexdigest() != expected['capture_sha256']:
            raise ValueError('native capture differs from repeat or pinned hash')
        selected = [line for line in captures[0].decode('ascii').splitlines()
                    if line.startswith('snapshot\t') or any(
                        line.startswith(f'sample\tsaved-grass\t{phase}\t-5\t{y}\t4\t')
                        for phase in ('before', 'after-place') for y in (66, 67, 68))]
        if selected != expected['rows']:
            raise ValueError('sample rows differ')
        probe = subprocess.run(['cargo', 'run', '--quiet', '--locked', '--offline',
                                '-p', 'beta-core', '--example', 'light_snapshot_probe'],
                               cwd=ROOT, input=captures[0], stdout=subprocess.PIPE, check=True)
        lines = probe.stdout.decode('ascii').splitlines()
        if lines[-1] != 'saved-grass\tafter-place\t294912\t0\t0\t0':
            raise ValueError('saved-world Rust/native comparison differs')
        for name, digest in expected['source_region_sha256'].items():
            if sha(source / name) != digest or sha(inputs / name) != digest:
                raise ValueError(f'region source or input changed: {name}')
    print('PASS: pinned copied regions; repeated native capture; Rust matches all 294912 cells')
    print(expected['scope'])


if __name__ == '__main__':
    run()
