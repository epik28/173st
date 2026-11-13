#!/usr/bin/env python3
"""Compare one saved-world edit; input must be a pinned copy inside artifacts.

Pass a world copy in artifacts as the argument; input hashes and results stay local.
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
JAR_SHA256 = 'af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c'
REGIONS = ('r.-1.-1.mcr', 'r.-1.0.mcr', 'r.0.-1.mcr', 'r.0.0.mcr')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run():
    if len(sys.argv) != 2:
        raise SystemExit('usage: light_saved_world_sequence_check.py ARTIFACTS_SOURCE_COPY')
    source = Path(sys.argv[1]).resolve(strict=True)
    if not source.is_dir() or not source.is_relative_to(ARTIFACTS) or source == ARTIFACTS:
        raise ValueError('source must be an existing copied world directory under artifacts')
    if sha(JAR) != JAR_SHA256:
        raise ValueError('wrong client JAR')
    hashes = {}
    for name in REGIONS:
        path = source / name
        if path.is_symlink() or not path.is_file() or path.resolve().parent != source:
            raise ValueError(f'missing or unsafe source copy: {name}')
        hashes[name] = sha(path)
    with tempfile.TemporaryDirectory(prefix='light-saved-world-', dir=ARTIFACTS) as temp:
        run_dir = Path(temp)
        classes, inputs = run_dir / 'classes', run_dir / 'input'
        classes.mkdir(); inputs.mkdir()
        for name in hashes:
            shutil.copyfile(source / name, inputs / name)
        subprocess.run(['javac', '-cp', str(JAR), '-d', str(classes),
                        str(SOURCE / 'LightUpdateOracle.java'),
                        str(SOURCE / 'LightMaterialEditV2Oracle.java'),
                        str(SOURCE / 'LightSavedWorldSequenceOracle.java')], check=True)
        command = ['java', '-XX:-UsePerfData', '-Xmx1g', '-cp', f'{classes}:{JAR}', 'LightSavedWorldSequenceOracle', str(JAR), str(inputs)]
        captures = [subprocess.run(command, stdout=subprocess.PIPE, check=True).stdout for _ in range(2)]
        if captures[0] != captures[1]:
            raise ValueError('native captures differ')
        probe = subprocess.run(['cargo', 'run', '--quiet', '--locked', '--offline',
                                '-p', 'beta-core', '--example', 'light_saved_sequence_probe'],
                               cwd=ROOT, input=captures[0], stdout=subprocess.PIPE, check=True)
        lines = probe.stdout.decode('ascii').splitlines()
        wanted = [f'saved-grass-sequence\tplaced-{i}\t294912\t0\t0\t0' for i in range(1, 5)]
        if lines[1:] != wanted:
            raise ValueError('saved-world sequence Rust/native comparison differs')
        report = {'input_hashes': hashes,
                  'jar_sha256': JAR_SHA256,
                  'capture_sha256': hashlib.sha256(captures[0]).hexdigest(),
                  'probe_rows': lines, 'captures_repeat_equal': True,
                  'scope': 'Saved chunks, four edits; no height-map or GPU parity.'}
        (ARTIFACTS / 'light-saved-world').mkdir(exist_ok=True)
        (ARTIFACTS / 'light-saved-world' / 'sequence-report.json').write_text(json.dumps(report, indent=2) + '\n')
        for name, digest in hashes.items():
            if sha(source / name) != digest or sha(inputs / name) != digest:
                raise ValueError(f'region source or input changed: {name}')
    print('PASS: hashed copied regions; repeated native capture; Rust matches 294912 cells after each of four edits')
    print('Scope: saved chunks, four cumulative edits; no height-map or GPU parity.')


if __name__ == '__main__':
    run()
