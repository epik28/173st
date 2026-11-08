import copy
import json
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/parity'))
import color_run as c
import color_review as r


class ColorRunnerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (ROOT / c.FIXTURES / 'cases.tsv').read_bytes()
        cls.output = (ROOT / c.FIXTURES / 'native.tsv').read_bytes()
        cls.cases = c.parse_input(cls.raw)

    def test_raw_climate_and_argb_comparison_detects_each_ordered_record(self):
        self.assertEqual(c.compare(self.output, self.output, self.cases)['mismatch_count'], 0)
        rows = self.output.splitlines()
        p = rows[0].split(b'\t'); p[2] = f'{int(p[2],16)^1:016x}'.encode()
        changed = b'\t'.join(p) + b'\n' + b'\n'.join(rows[1:]) + b'\n'
        result = c.compare(self.output, changed, self.cases)
        self.assertEqual(result['mismatch_count'], 1)
        self.assertEqual(result['first_mismatch']['component'], 0)
        self.assertEqual(r.mutation_checks(self.output, self.cases), 4)

    def test_malformed_domains_framing_and_canonical_numbers(self):
        for bad in r.malformed_inputs(self.raw) + [b'x\tclimate\t-0\t0\t0\n',
                b'x\tclimate\t0\t01\t0\n', b'x\tpalette\t3ff0000000000000\tfff0000000000000\n']:
            with self.subTest(bad=bad[:80]), self.assertRaises(c.ColorError):
                c.parse_input(bad)
        with self.assertRaises(c.ColorError):
            c.parse_input(b'x' * (4 * 1024 * 1024 + 1))

    def test_output_order_count_kind_and_raw_color_width(self):
        for bad in (self.output[:-1], self.output + self.output.splitlines(keepends=True)[0],
                    self.output.replace(b'\tclimate\t', b'\tpalette\t', 1),
                    self.output.replace(b'ff', b'FF', 1)):
            with self.assertRaises(c.ColorError):
                c.parse_output(bad, self.cases)

    def test_audit_pins_primary_counts_resources_and_bootstrap(self):
        pinned = c.parse_audit((ROOT / c.FIXTURES / 'native.audit').read_bytes())
        jar = Path('/tmp/fake-native-color.jar')
        audit = {**pinned, 'code_source': jar.as_uri()}
        hashes = dict(item.split(':', 1) for item in audit['classes'].rstrip(',').split(','))
        with patch.object(c, 'native_hashes', return_value=hashes):
            c.validate_audit(audit, self.cases, jar, pinned)
            for key, value in [('wp_b', '0'), ('xv_seed_reads', '0'), ('native_world_constructors', '1'),
                               ('code_source', 'file:///tmp/other.jar'), ('grass', '0'*64),
                               ('bootstrap_base64', '')]:
                with self.subTest(key=key), self.assertRaises((c.ColorError, ValueError)):
                    c.validate_audit({**audit, key: value}, self.cases, jar, pinned)

    def test_missing_parts_and_unsuccessful_saved_report_are_not_pass(self):
        with self.assertRaises(SystemExit):
            r.verify_color(Path('/tmp/not-a-color-report.json'), {'suite':'color','status':'error'})
        with self.assertRaises(c.ColorError):
            c.parse_audit(b'')
        with self.assertRaises(c.ColorError):
            c.parse_audit(b'COLOR_NATIVE_AUDIT\t1\tx=1\tx=2\n')


@unittest.skipUnless(os.environ.get('BETA_LIVE_COLOR') == '1', 'set BETA_LIVE_COLOR=1 for native color comparison/mutations')
class LiveColorEvidenceTests(unittest.TestCase):
    def test_live_native_trace_saved_evidence_and_mutations(self):
        report, path = c.run_color_suite()
        self.assertEqual(report['status'], 'pass', report)
        self.assertFalse(report['rust_dirty'])
        self.assertEqual(r.verify_color(path, report, protocol=True), 0)
        for field, value in [('rust_dirty', True), ('rust_revision', '0'*40), ('mismatch_count', 1), ('rust_output_sha256', '0'*64)]:
            changed = copy.deepcopy(report); changed[field] = value
            with self.subTest(field=field), self.assertRaises(SystemExit):
                r.verify_color(path, changed)
        for label in ('rust-run', 'java-run', 'rust-build'):
            changed = copy.deepcopy(report); changed['processes'][label]['returncode'] = False
            with self.subTest(process=label), self.assertRaises(SystemExit):
                r.verify_color(path, changed)
            changed = copy.deepcopy(report); changed['processes'][label]['command'][0] = '/tmp/untrusted-command'
            with self.subTest(command=label), self.assertRaises(SystemExit):
                r.verify_color(path, changed)
        directory = path.parent
        for name in (c.EXAMPLE, 'grasscolor.png', 'java-run.stderr', 'java-run.stdout', 'input.tsv'):
            artifact = directory / name; original = artifact.read_bytes()
            try:
                artifact.write_bytes(original + b'mutated')
                with self.subTest(artifact=name), self.assertRaises(SystemExit):
                    r.verify_color(path, report)
            finally:
                artifact.write_bytes(original)
        self.assertEqual(r.verify_color(path, report), 0)
