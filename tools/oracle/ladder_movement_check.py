#!/usr/bin/env python3
"""Native ladder travel with forced ladder detection; world selection is out of scope."""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR = (ROOT.parent / '.local/minecraft/client-b1.7.3.jar').resolve()
SOURCE = ROOT / 'tools/oracle/src'
FIXTURE = ROOT / 'tests/fixtures/ladder_movement'

with tempfile.TemporaryDirectory(prefix='ladder-movement-') as temp:
    subprocess.run(['javac', '-cp', str(JAR), '-d', temp, str(SOURCE / 'LadderMovementOracle.java'), str(SOURCE / 'LadderPresenceOracle.java')], check=True)
    request = (FIXTURE / 'input.tsv').read_bytes()
    captures = []
    for run in range(2):
        command = ['java', '-XX:-UsePerfData', '-cp', f'{temp}:{JAR}', 'LadderMovementOracle',
                   str(JAR), str(Path(temp) / f'audit-{run}.json')]
        captures.append(subprocess.run(command, input=request, stdout=subprocess.PIPE, check=True).stdout)
    if captures[0] != captures[1] or captures[0] != (FIXTURE / 'reference.tsv').read_bytes():
        raise SystemExit('native ladder movement captures disagree')
    presence_command = ['java', '-XX:-UsePerfData', '-cp', f'{temp}:{JAR}', 'LadderPresenceOracle', str(JAR)]
    presence = [subprocess.run(presence_command, stdout=subprocess.PIPE, check=True).stdout for _ in range(2)]
    if presence[0] != presence[1] or presence[0] != (FIXTURE / 'presence.tsv').read_bytes():
        raise SystemExit('native ladder presence captures disagree')
    rust = subprocess.run(['cargo', 'run', '--quiet', '--locked', '--offline', '-p', 'beta-core',
                           '--example', 'ladder_movement_probe'], cwd=ROOT, input=request,
                          stdout=subprocess.PIPE, check=True).stdout
    if rust != captures[0]:
        raise SystemExit('Rust ladder movement differs from native')
print(f'PASS: two native captures and Rust agree on {len(captures[0].splitlines())} states')
print('Scope: forced ladder detection throughout each case; not full world selection or water interaction.')
