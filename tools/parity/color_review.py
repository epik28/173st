"""Review retained climate/color evidence before optional protocol executions."""
from collections import Counter
import json
from pathlib import Path
import subprocess

from color_run import (ROOT, FIXTURES, PALETTES, EXAMPLE, REQUIRED, ColorError,
                       compare, expected_commands, jar_path, parse_audit,
                       parse_input, parse_output, source_snapshot, validate_audit)
from run import sha256


def require(value, message):
    if not value:
        raise ColorError(message)


def retained(path, directory):
    path = Path(path)
    require(not path.is_symlink() and path.is_file(), 'missing/symlinked retained file: ' + str(path))
    require(path.resolve().is_relative_to(directory), 'retained file escapes report directory')
    return path.read_bytes()


def verify_color(report_path, report, protocol=False):
    try:
        return _verify_color(Path(report_path), report, protocol)
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise SystemExit('invalid color evidence: ' + str(error)) from error


def _verify_color(report_path, report, protocol):
    directory = report_path.resolve().parent
    require(report.get('suite') == 'color' and report.get('status') == 'pass', 'successful color report required')
    require(report.get('rust_dirty') is False, 'clean Rust snapshot required')
    processes = report['processes']
    commands = expected_commands(ROOT, directory, jar_path(ROOT))
    for label in REQUIRED:
        p = processes[label]
        require(type(p.get('returncode')) is int and p['returncode'] == 0
                and p.get('timeout') is False and p.get('error') is None, 'failed process: ' + label)
        require(p.get('command') == commands[label] and p.get('cwd') == str(ROOT), 'command/cwd differs: ' + label)
        for stream in ('stdout', 'stderr'):
            expected = directory / (label + '.' + stream)
            require(p.get(stream) == str(expected), 'process artifact path differs: ' + label)
            require(report['log_paths'][label][stream] == str(expected), 'log path differs: ' + label)
            require(sha256(retained(expected, directory)) == p[stream + '_sha256'], 'process log hash differs: ' + label)
    revision = retained(directory / 'git-revision.stdout', directory).decode().strip()
    require(revision == report['rust_revision'] and len(revision) == 40, 'revision metadata differs')
    require(not retained(directory / 'git-dirty.stdout', directory), 'recorded working tree was dirty')
    snapshot = report['source_hashes']
    require(snapshot == report['source_hashes_after'] == source_snapshot(ROOT), 'source snapshot differs')
    for name, digest in snapshot.items():
        if digest is not None:
            require(sha256(retained(directory / 'sources' / name, directory)) == digest, 'retained source differs: ' + name)
    for tool in ('java', 'javac', 'rustc', 'cargo'):
        actual = (retained(directory / ('version-' + tool + '.stdout'), directory)
                  + retained(directory / ('version-' + tool + '.stderr'), directory)).decode().strip()
        require(actual == report['runtime_versions'][tool], 'runtime version metadata differs')
    jar = jar_path(ROOT)
    require(report.get('jar_path') == str(jar) and report['jar_sha256'] == sha256(jar.read_bytes()), 'original JAR differs')
    for name, digest in PALETTES.items():
        require(sha256(retained(directory / (name + 'color.png'), directory)) == digest, 'retained palette differs')
    require(sha256(retained(directory / EXAMPLE, directory)) == report['rust_binary_sha256'], 'retained executable differs')
    classes = directory / 'classes'
    require(not classes.is_symlink() and classes.is_dir(), 'compiled adapter directory invalid')
    class_hashes = {str(p.relative_to(classes)): sha256(retained(p, directory)) for p in classes.rglob('*.class')}
    require(class_hashes and class_hashes == report['adapter_class_hashes'], 'compiled adapter differs')
    raw = retained(directory / 'input.tsv', directory)
    require(sha256(raw) == report['input_sha256'], 'retained input differs')
    cases = parse_input(raw)
    require(len(cases) == report['case_count'] and dict(Counter(p[1] for p in cases)) == report['operation_counts'], 'case counts differ')
    require(report.get('scenario') in ('pinned', 'custom'), 'invalid scenario origin')
    if report['scenario'] == 'pinned':
        require(raw == (ROOT / FIXTURES / 'cases.tsv').read_bytes(), 'pinned input differs')
    pinned = parse_audit((ROOT / FIXTURES / 'native.audit').read_bytes())
    outputs, audits = [], []
    for label in ('java-run', 'java-repeat', 'rust-run'):
        output = retained(directory / (label + '.stdout'), directory)
        parse_output(output, cases)
        outputs.append(output)
        if label != 'rust-run':
            audit = parse_audit(retained(directory / (label + '.stderr'), directory))
            validate_audit(audit, cases, jar, pinned)
            audits.append(audit)
    require(outputs[0] == outputs[1] and audits[0] == audits[1], 'native results not repeatable')
    require(sha256(outputs[0]) == report['reference_output_sha256']
            and sha256(outputs[2]) == report['rust_output_sha256'], 'output hash metadata differs')
    if report['scenario'] == 'pinned':
        require(outputs[0] == (ROOT / FIXTURES / 'native.tsv').read_bytes(), 'native fixture differs')
    comparison = compare(outputs[0], outputs[2], cases)
    require(comparison['mismatch_count'] == 0 and all(report[k] == v for k, v in comparison.items()), 'comparison summary differs')
    mutations = mutation_checks(outputs[0], cases)
    checks = 0
    if protocol:
        for bad in malformed_inputs(raw):
            for label in ('java-run', 'rust-run'):
                result = subprocess.run(commands[label], cwd=ROOT, input=bad, capture_output=True, timeout=30)
                require(result.returncode != 0 and not result.stdout and bool(result.stderr), 'malformed input not atomically rejected')
                checks += 1
    print(f'PASS: saved color evidence; output mutations={mutations}; malformed executions={checks}')
    return 0


def mutation_checks(output, cases):
    rows = output.splitlines(keepends=True)
    p = rows[0].decode().rstrip('\n').split('\t')
    p[-1] = f'{int(p[-1], 16) ^ 1:08x}'
    mutations = [('\t'.join(p) + '\n').encode() + b''.join(rows[1:]), b''.join(rows[:-1]),
                 (b''.join(rows[1:] + rows[:1]) if len(rows) > 1 else b'changed' + output), output + rows[-1]]
    for changed in mutations:
        try:
            require(compare(output, changed, cases)['mismatch_count'] > 0, 'output mutation missed')
        except ColorError as error:
            if str(error) == 'output mutation missed':
                raise
    return len(mutations)


def malformed_inputs(raw):
    first = raw.splitlines(keepends=True)[0]
    return [b'', raw.rstrip(b'\n'), first + first, first + b'late\tunknown\n',
            b'x\tclimate\t9223372036854775808\t0\t0\n',
            b'x\tclimate\t0\t30000001\t0\n', b'x\tclimate\t0\t+1\t0\n',
            b'x\tpalette\t7ff0000000000000\t0000000000000000\n',
            b'x\tpalette\t3ff0000000000001\t0000000000000000\n',
            first + b'\n', b'x\tpalette\t3FF0000000000000\t0000000000000000\n',
            b'\xff\tclimate\t0\t0\t0\n']
