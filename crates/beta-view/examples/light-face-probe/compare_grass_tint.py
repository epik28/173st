#!/usr/bin/env python3
"""Compare native and Rust flat grass tinted/overlay colors before/after edit.

A synthetic world supplies controlled lighting and four explicit RGB values; the
actual biome palette, terrain texture, ambient occlusion, GPU and screenshots
are intentionally outside this test. No original assets are copied or saved.
"""
from collections import Counter
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[4]
jar = root.parent / '.local/minecraft/client-b1.7.3.jar'
source = Path(__file__).parent
with tempfile.TemporaryDirectory(prefix='grass-tint-probe-') as classes:
    subprocess.run(['javac', '-cp', str(jar), '-d', classes,
                    str(source / 'LightFaceOracle.java'),
                    str(source / 'GrassTintFaceOracle.java')], check=True)
    command = ['java', '-cp', f'{classes}:{jar}', 'GrassTintFaceOracle', str(jar)]
    native = subprocess.check_output(command)
    if native != subprocess.check_output(command):
        raise ValueError('two native captures differ')
    rust = subprocess.check_output(['cargo', 'run', '--quiet', '--locked', '--offline',
                                    '-p', 'beta-view', '--example', 'grass_tint_face_probe'], cwd=root)
    a, b = Counter(native.decode().splitlines()), Counter(rust.decode().splitlines())
    if a != b:
        print('native only:', *(a-b).elements(), sep='\n')
        print('rust only:', *(b-a).elements(), sep='\n')
        raise SystemExit('FAIL: tinted flat face comparison differs')
    print(f'PASS: {sum(a.values())} native vertex colors match, including grass side overlays')
