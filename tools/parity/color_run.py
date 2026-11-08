"""Exact scalar Beta climate and grass/foliage color comparison against native calls."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import shutil
import struct
import tempfile
import time
from urllib.parse import unquote, urlparse
import zipfile

from geometry_run import JAR_SHA, jar_path
from run import ROOT, artifacts_root, exit_code, run_process, sha256

FIXTURES = Path('tests/fixtures/color')
ORACLE = Path('tools/oracle/src/ColorOracle.java')
PROBE = Path('crates/beta-viewer/examples/color_probe.rs')
EXAMPLE = 'color_probe'
PALETTES = {'grass': 'd9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b',
            'foliage': '644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc'}
REQUIRED = ('git-revision', 'git-dirty', 'version-java', 'version-javac', 'version-rustc',
            'version-cargo', 'java-build', 'java-run', 'java-repeat', 'rust-build', 'rust-run')


class ColorError(ValueError):
    pass


def scalar(bits):
    if not re.fullmatch('[0-9a-f]{16}', bits):
        raise ColorError('expected lowercase raw f64 hex')
    value = struct.unpack('>d', bytes.fromhex(bits))[0]
    if not 0 <= value <= 1:
        raise ColorError('climate must be finite and in [0,1]')
    return value


def decimal(text, lo, hi):
    if not re.fullmatch(r'(?:0|-?[1-9][0-9]*)', text) or not lo <= int(text) <= hi:
        raise ColorError('noncanonical or out-of-domain integer')
    return int(text)


def lines(data):
    if not data or len(data) > 4 * 1024 * 1024:
        raise ColorError('input/output must be nonempty and <=4MiB')
    try:
        value = data.decode('ascii')
    except UnicodeDecodeError as error:
        raise ColorError('protocol requires ASCII') from error
    if not value.endswith('\n'):
        raise ColorError('protocol requires final LF')
    return value[:-1].split('\n')


def parse_input(data):
    rows = lines(data)
    if len(rows) > 4096:
        raise ColorError('row limit4096 exceeded')
    result, ids = [], set()
    for row in rows:
        p = row.split('\t')
        if len(p) < 2 or not re.fullmatch(r'[A-Za-z0-9_.:-]{1,96}', p[0]) or p[0] in ids:
            raise ColorError('invalid or duplicate case ID')
        ids.add(p[0])
        if p[1] == 'climate' and len(p) == 5:
            decimal(p[2], -(1 << 63), (1 << 63) - 1)
            for part in p[3:]:
                decimal(part, -30_000_000, 30_000_000)
        elif p[1] == 'palette' and len(p) == 4:
            scalar(p[2]); scalar(p[3])
        else:
            raise ColorError('unknown operation or wrong column count')
        result.append(p)
    return result


def parse_output(data, cases):
    rows = lines(data)
    if len(rows) != len(cases):
        raise ColorError('output record count mismatch')
    result = []
    for row, case in zip(rows, cases):
        p = row.split('\t')
        if p[:2] != case[:2] or len(p) != (9 if case[1] == 'climate' else 4):
            raise ColorError('output operation, count or order mismatch')
        start = 2
        if case[1] == 'climate':
            scalar(p[2]); scalar(p[3]); start = 4
        if any(not re.fullmatch('[0-9a-f]{8}', value) for value in p[start:]):
            raise ColorError('colors require raw u32 lowercase hex')
        result.append(p)
    return result


def compare(reference, actual, cases):
    left, right = parse_output(reference, cases), parse_output(actual, cases)
    mismatches, first = 0, None
    for i, (expected, observed) in enumerate(zip(left, right)):
        if expected != observed:
            mismatches += 1
            if first is None:
                field = next(k for k in range(2, len(expected)) if expected[k] != observed[k])
                first = {'record': i, 'case': cases[i][0], 'operation': cases[i][1],
                         'component': field - 2, 'reference': expected[field], 'rust': observed[field]}
    return {'mismatch_count': mismatches, 'first_mismatch': first}


def parse_audit(raw):
    rows = raw.decode('ascii').splitlines()
    if len(rows) != 1 or not rows[0].startswith('COLOR_NATIVE_AUDIT\t1\t'):
        raise ColorError('native audit missing or malformed')
    audit = {}
    for item in rows[0].split('\t')[2:]:
        key, value = item.split('=', 1)
        if key in audit:
            raise ColorError('duplicate audit field')
        audit[key] = value
    return audit


def native_hashes(jar, names):
    with zipfile.ZipFile(jar) as archive:
        return {name: sha256(archive.read(name + '.class')) for name in names}


def validate_audit(audit, cases, jar, pinned):
    import base64
    for key in ('jar', 'grass', 'foliage', 'classes', 'api', 'bootstrap_bytes', 'bootstrap_sha',
                'seed_only_world', 'counts_scope', 'internal_block_calls', 'palette_dimensions',
                'palette_order', 'ia_setter_calls', 'jh_setter_calls'):
        if audit.get(key) != pinned.get(key) or not pinned.get(key):
            raise ColorError('native pinned audit field differs: ' + key)
    if audit['jar'] != JAR_SHA or any(audit[k] != h for k, h in PALETTES.items()):
        raise ColorError('native resource identity differs')
    hashes = dict(item.split(':', 1) for item in audit['classes'].rstrip(',').split(','))
    if not hashes or hashes != native_hashes(jar, hashes):
        raise ColorError('native class hashes differ from original JAR')
    source = urlparse(audit.get('code_source', ''))
    if source.scheme != 'file' or source.netloc or source.query or source.fragment or Path(unquote(source.path)).resolve() != jar:
        raise ColorError('native class code source differs')
    if audit.get('native_world_constructors') != '0' or audit.get('synthetic_fd_ctor_calls') != '0':
        raise ColorError('native World constructor executed or missing audit')
    capture = base64.b64decode(audit['bootstrap_base64'], validate=True)
    if len(capture) != int(audit['bootstrap_bytes']) or sha256(capture) != audit['bootstrap_sha']:
        raise ColorError('native bootstrap capture differs')
    c = sum(row[1] == 'climate' for row in cases)
    p = len(cases) - c
    seeds = len({row[2] for row in cases if row[1] == 'climate'})
    expected = {'xv_explicit_samples': c, 'wp_b': c, 'bk_b': 4 * c,
                'climate_rows': c, 'palette_rows': p, 'xv_seed_reads': 3 * seeds, 'manager_ctor_calls': seeds,
                'ia_explicit_DD': p, 'jh_explicit_DD': p}
    for key, count in expected.items():
        if audit.get(key) != str(count):
            raise ColorError('native primary count differs: ' + key)


def source_snapshot(root=ROOT):
    paths = {Path('Cargo.toml'), Path('Cargo.lock'), Path('rust-toolchain.toml'),
             Path('.cargo/config'), Path('.cargo/config.toml'), Path('config/local.toml'),
             ORACLE, PROBE, Path('tools/oracle/color_corpus.py'), Path('tools/prepare_assets.py')}
    for folder, patterns in [('crates/beta-core', ('*.rs', 'Cargo.toml')),
                             ('crates/beta-viewer', ('*.rs', 'Cargo.toml')),
                             ('crates/beta-view', ('*.rs', 'Cargo.toml')),
                             ('tools/parity', ('*.py',)), (str(FIXTURES), ('*',))]:
        for pattern in patterns:
            paths.update(p.relative_to(root) for p in (root / folder).rglob(pattern) if p.is_file())
    return {str(p): sha256((root / p).read_bytes()) if (root / p).is_file() else None for p in sorted(paths)}


def expected_commands(root, directory, jar):
    classes = directory / 'classes'
    palettes = [str(directory / (name + 'color.png')) for name in PALETTES]
    result = {'git-revision': ['git', 'rev-parse', 'HEAD'], 'git-dirty': ['git', 'status', '--porcelain'],
              'java-build': ['javac', '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(classes), str(root / ORACLE)],
              'rust-build': ['cargo', 'build', '--locked', '--offline', '-p', 'beta-viewer', '--example', EXAMPLE,
                             '--message-format=json-render-diagnostics'],
              'rust-run': [str(directory / EXAMPLE)] + palettes}
    for tool in ('java', 'javac', 'rustc', 'cargo'):
        result['version-' + tool] = [tool, '-version' if tool in ('java', 'javac') else '--version']
    for label in ('java-run', 'java-repeat'):
        result[label] = ['java', '-XX:-UsePerfData', '-cp', str(classes) + ':' + str(jar), 'ColorOracle', str(jar)] + palettes
    return result


def run_color_suite(root=ROOT, timeout=60, input_path=None):
    root = Path(root).resolve()
    parent = artifacts_root(root) / 'parity/color'
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix='run-', dir=parent))
    path = directory / 'report.json'
    started = time.monotonic()
    report = {'schema_version': 1, 'suite': 'color', 'status': 'error', 'reason': 'comparison incomplete',
              'scenario': 'custom' if input_path else 'pinned', 'processes': {}, 'log_paths': {},
              'mismatch_count': None, 'first_mismatch': None, 'source_hashes': source_snapshot(root),
              'started_at': datetime.now(timezone.utc).isoformat()}

    def execute(label, data=None):
        p = run_process(commands[label], cwd=root, directory=directory, label=label, timeout=timeout, input_data=data)
        report['processes'][label] = p
        report['log_paths'][label] = {k: p[k] for k in ('stdout', 'stderr')}
        if p['error'] or p['returncode'] != 0:
            raise ColorError(f"{label}: {p['error'] or 'exit ' + str(p['returncode'])}")
        return Path(p['stdout']).read_bytes()

    try:
        jar = jar_path(root)
        commands = expected_commands(root, directory, jar)
        pinned = parse_audit((root / FIXTURES / 'native.audit').read_bytes())
        report.update(jar_path=str(jar), jar_sha256=sha256(jar.read_bytes()))
        with zipfile.ZipFile(jar) as archive:
            for name, expected in PALETTES.items():
                data = archive.read('misc/' + name + 'color.png')
                if sha256(data) != expected:
                    raise ColorError('native palette changed: ' + name)
                (directory / (name + 'color.png')).write_bytes(data)
        data = Path(input_path).read_bytes() if input_path else (root / FIXTURES / 'cases.tsv').read_bytes()
        cases = parse_input(data)
        (directory / 'input.tsv').write_bytes(data)
        report.update(input_sha256=sha256(data), case_count=len(cases),
                      operation_counts=dict(Counter(p[1] for p in cases)))
        report['rust_revision'] = execute('git-revision').decode().strip()
        report['rust_dirty'] = bool(execute('git-dirty'))
        report['runtime_versions'] = {}
        for tool in ('java', 'javac', 'rustc', 'cargo'):
            execute('version-' + tool)
            p = report['processes']['version-' + tool]
            report['runtime_versions'][tool] = (Path(p['stdout']).read_bytes() + Path(p['stderr']).read_bytes()).decode().strip()
        (directory / 'classes').mkdir()
        execute('java-build')
        report['adapter_class_hashes'] = {str(p.relative_to(directory / 'classes')): sha256(p.read_bytes())
                                        for p in (directory / 'classes').rglob('*.class')}
        outputs, audits = [], []
        for label in ('java-run', 'java-repeat'):
            output = execute(label, data)
            parse_output(output, cases)
            audit = parse_audit(Path(report['processes'][label]['stderr']).read_bytes())
            validate_audit(audit, cases, jar, pinned)
            outputs.append(output); audits.append(audit)
        if outputs[0] != outputs[1] or audits[0] != audits[1]:
            raise ColorError('native outputs or audit not repeatable')
        if not input_path and outputs[0] != (root / FIXTURES / 'native.tsv').read_bytes():
            raise ColorError('native output differs from committed fixture')
        report['reference_output_sha256'] = sha256(outputs[0])
        if not (root / PROBE).is_file():
            report.update(status='not_implemented', reason='Rust color_probe missing; native comparison executed twice')
        else:
            records = [json.loads(row) for row in execute('rust-build').splitlines()]
            binaries = [Path(r['executable']) for r in records if r.get('reason') == 'compiler-artifact'
                        and r.get('target', {}).get('name') == EXAMPLE and r.get('executable')]
            if len(binaries) != 1:
                raise ColorError('cargo did not identify exactly one color probe')
            shutil.copy2(binaries[0], directory / EXAMPLE)
            report['rust_binary_sha256'] = sha256((directory / EXAMPLE).read_bytes())
            actual = execute('rust-run', data)
            report['rust_output_sha256'] = sha256(actual)
            report.update(compare(outputs[0], actual, cases))
            report.update(status='fail' if report['mismatch_count'] else 'pass',
                          reason='color mismatch' if report['mismatch_count'] else 'all native climate/colors match exactly')
        if sha256(jar.read_bytes()) != report['jar_sha256']:
            raise ColorError('original JAR changed during run')
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        report.update(status='error', reason=str(error))
    finally:
        report['source_hashes_after'] = source_snapshot(root)
        if report['source_hashes'] != report['source_hashes_after']:
            report.update(status='error', reason='source inputs changed during run')
        for relative, digest in report['source_hashes'].items():
            if digest is not None:
                dest = directory / 'sources' / relative
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes((root / relative).read_bytes())
        report['duration_ms'] = round((time.monotonic() - started) * 1000, 3)
        path.write_text(json.dumps(report, indent=2) + '\n')
    return report, path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path)
    parser.add_argument('--timeout', type=float, default=60)
    parser.add_argument('--strict', action='store_true')
    args = parser.parse_args()
    if not 0 < args.timeout < float('inf'):
        parser.error('timeout must be positive and finite')
    report, path = run_color_suite(timeout=args.timeout, input_path=args.input)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return exit_code(report['status'], args.strict)


if __name__ == '__main__':
    raise SystemExit(main())
