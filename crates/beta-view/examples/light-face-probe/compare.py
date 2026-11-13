#!/usr/bin/env python3
"""Compare native tessellation to beta-view mesh in a synthetic, headless edit.

This compares flat face visibility and packed monochrome vertex colors. Lighting
cells are specified identically by the harness; no propagation, GPU or real save
is tested. Requires the configured original jar and a local Java toolchain.
"""
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[4]
jar = root.parent / '.local/minecraft/client-b1.7.3.jar'
source = Path(__file__).with_name('LightFaceOracle.java')
with tempfile.TemporaryDirectory(prefix='light-face-') as directory:
    subprocess.run(['javac', '-cp', str(jar), '-d', directory, str(source)], check=True)
    native = subprocess.check_output(['java', '-cp', f'{directory}:{jar}', 'LightFaceOracle', str(jar)])
    rust = subprocess.check_output(['cargo', 'run', '--quiet', '--locked', '--offline', '-p', 'beta-view', '--example', 'light_face_probe'], cwd=root)
    a, b = sorted(native.decode().splitlines()), sorted(rust.decode().splitlines())
    if a != b:
        print('native only:', *sorted(set(a)-set(b)), sep='\n')
        print('rust only:', *sorted(set(b)-set(a)), sep='\n')
        raise SystemExit('FAIL: face comparison differs')
    print(f'PASS: {len(a)} flat face colors match native before/after edit')
