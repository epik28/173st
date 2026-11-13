#!/usr/bin/env python3
"""Compare flat native/Rust face colors using a disposable copy of a 3x3 saved area.

Native reads the already copied world only after it has been copied again into a
private temporary directory. No world, capture, or generated class is persisted.
The two renderers receive native neighbor light/opacity values; independent Rust
relighting, GPU, textures, tint, overlays and screenshots are outside scope.
"""
from collections import Counter
from pathlib import Path
import argparse
import hashlib
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[4]
jar = root.parent / '.local/minecraft/client-b1.7.3.jar'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('world', type=Path, help='copied world directory inside artifacts')
world = parser.parse_args().world.resolve(strict=True)
artifacts = (root / 'artifacts').resolve(strict=True)
if not world.is_dir() or not world.is_relative_to(artifacts) or world == artifacts:
    raise ValueError('world must be a copied directory inside artifacts')
source = Path(__file__).parent
shared = root / 'tools/oracle/src'
names = [f'r.{x}.{z}.mcr' for x in (-1, 0) for z in (-1, 0)]
with tempfile.TemporaryDirectory(prefix='saved-face-probe-') as tmp:
    tmpdir = Path(tmp)
    classes = tmpdir / 'classes'
    classes.mkdir()
    before = {}
    for name in names:
        path = world / name
        if path.is_symlink() or not path.is_file() or path.resolve().parent != world.resolve():
            raise ValueError(f'unsafe region path: {path}')
        before[name] = hashlib.sha256(path.read_bytes()).hexdigest()
        shutil.copyfile(path, tmpdir / name)
    subprocess.run(['javac', '-cp', str(jar), '-d', str(classes),
                    str(shared / 'LightUpdateOracle.java'),
                    str(shared / 'LightMaterialEditV2Oracle.java'),
                    str(source / 'LightFaceOracle.java'), str(source / 'SavedFaceOracle.java')],
                   check=True, stdout=subprocess.DEVNULL)
    command = ['java', '-cp', f'{classes}:{jar}', 'SavedFaceOracle', str(jar), str(tmpdir)]
    native = subprocess.check_output(command)
    native2 = subprocess.check_output(command)
    if native != native2:
        raise ValueError('two native captures differ')
    rust = subprocess.check_output(['cargo', 'run', '--quiet', '--locked', '--offline', '-p', 'beta-view', '--example', 'saved_light_face_probe'], cwd=root, input=native)
    a = Counter(line for line in native.decode().splitlines() if line.startswith('face\t'))
    b = Counter(rust.decode().splitlines())
    if not a:
        raise ValueError('empty native capture')
    if a != b:
        print('native only:', *(a-b).elements(), sep='\n')
        print('rust only:', *(b-a).elements(), sep='\n')
        raise SystemExit('FAIL: flat face comparison differs')
    if any(hashlib.sha256((world / name).read_bytes()).hexdigest() != before[name] for name in names):
        raise ValueError('source region changed during capture')
    cases = sum(line.startswith(b'case\t') for line in native.splitlines())
    print(f'PASS: {sum(a.values())} flat faces across {cases} block-phase samples match native')
    print('Scope: x=-7..-3 y=65..68 z=2..6, before/after placing stone at (-5,67,4); native light cells in both renderers; no GPU or independent Rust relight.')
