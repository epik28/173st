"""RNG-002 synthetic protocol/comparator regressions; not live parity results."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('extended_runner', ROOT/'tools/parity/run.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class ExtendedTests(unittest.TestCase):
    def test_corpus_structure_and_types(self):
        cases = runner.parse_extended_fixture((ROOT/'tests/fixtures/rng/extended.tsv').read_bytes())
        self.assertEqual(sum(case['count'] for case in cases), 13227)
        self.assertEqual(len(cases), 25)
        case = next(case for case in cases if case['id'] == 'rejection')
        self.assertEqual(case['seed'], 0)
        self.assertEqual(case['operations'], [('bound',1073741825)]*2048)

    def test_invalid_fixture_syntax_and_limits(self):
        for data in (b' \n',b'\t\n',b'x\t0\t',b'x\t0\tint;',b'x\t0\tunknown',
                     b'x\t0\tlong;;int',b'x\t0\tbound:2147483648',b'x\t0\tseed:9223372036854775808',
                     b'x\t0\tint\nx\t0\tint',b'x\t0\tint ',
                     b'x\t0\t'+b';'.join([b'int']*100001)):
            with self.subTest(data=data[:40]), self.assertRaises(runner.RunError):
                runner.parse_extended_fixture(data)

    def test_each_result_type_and_bounds(self):
        cases = runner.parse_extended_fixture(b'x\t0\tint;long;boolean;float;double;bound:0;seed:1;bound:17')
        keys = runner.extended_keys(cases)
        values = [b'-2147483648',b'9223372036854775807',b'1',b'3f000000',b'3fe0000000000000',b'invalid_bound',b'ok',b'16']
        bad = [b'2147483648',b'9223372036854775808',b'true',b'0.5',b'3ff0000000000000',b'0',b'1',b'17']
        for key, good, wrong in zip(keys,values,bad):
            prefix = f'{key[0]}\t{key[1]}\t{key[2]}\t'.encode()
            self.assertTrue(runner.valid_record(prefix+good+b'\n',key))
            self.assertFalse(runner.valid_record(prefix+wrong+b'\n',key))

    def test_missing_extra_order_bits_and_state_errors(self):
        cases = runner.parse_extended_fixture(b'x\t0\tbound:0;int;seed:42;double')
        keys = runner.extended_keys(cases)
        original = b'x\t0\tbound\tinvalid_bound\nx\t1\tint\t7\nx\t2\tseed\tok\nx\t3\tdouble\t3fe0000000000000\n'
        self.assertEqual(runner.compare_outputs(original,original,keys)['mismatch_count'],0)
        variants = [b'',original.split(b'\n',1)[1], original+original.splitlines(keepends=True)[0],
                    b''.join(original.splitlines(keepends=True)[::-1]),original.replace(b'\t7\n',b'\t8\n'),
                    original.replace(b'3fe0000000000000',b'3fe0000000000001'),original.replace(b'ok',b'invalid_bound')]
        for actual in variants:
            self.assertGreater(runner.compare_outputs(original,actual,keys)['mismatch_count'],0)

if __name__ == '__main__': unittest.main()
