"""CHUNK-001 synthetic protocol regressions, independent of live original-JAR tests."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('chunk_runner',ROOT/'tools/parity/run.py')
runner=importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class ChunkTests(unittest.TestCase):
    def test_fixture_structure(self):
        cases=runner.parse_chunk_fixture((ROOT/'tests/fixtures/chunk/layout.tsv').read_bytes())
        self.assertEqual(sum(case['count'] for case in cases),4182)
        self.assertEqual(len(cases),2)

    def test_invalid_protocol(self):
        for data in [b' \n',b'\t\n',b'x\tunknown',b'x\t',b'x\tcoord:0;',b'x\tget:-1:0:0',
                     b'x\tget:256:0:0',b'x\tcoord:2147483648',b'x\tbyte:data:16384',
                     b'x\tbyte:blocks:-1',b'x\tbyte:unknown:0',b'x\tcoord:0\nx\tcoord:0',
                     b'x\t'+b';'.join([b'coord:0']*100001)]:
            with self.subTest(data=data[:40]),self.assertRaises(runner.RunError):
                runner.parse_chunk_fixture(data)

    def test_typed_results_and_error_priority(self):
        data=b'x\tcoord:-17;index:15:127:15;get:0:0:0;set:16:0:0:255:255:255:255;set:0:0:0:0:16:0:0;byte:data:0'
        keys=runner.extended_keys(runner.parse_chunk_fixture(data))
        good=[b'-2,15',b'32767',b'255,15,0,12',b'invalid_coordinate',b'invalid_nibble',b'163']
        bad=[b'-1,-1',b'32766',b'256,15,0,12',b'invalid_nibble',b'ok',b'256']
        for key,value,wrong in zip(keys,good,bad):
            prefix=f'{key[0]}\t{key[1]}\t{key[2]}\t'.encode()
            self.assertTrue(runner.valid_record(prefix+value+b'\n',key))
            self.assertFalse(runner.valid_record(prefix+wrong+b'\n',key))

    def test_swapped_nibbles_wrong_index_and_incomplete_output(self):
        keys=runner.extended_keys(runner.parse_chunk_fixture(b'x\tbyte:data:0;index:0:1:0;get:0:0:0'))
        original=b'x\t0\tbyte\t163\nx\t1\tindex\t1\nx\t2\tget\t255,3,7,11\n'
        self.assertEqual(runner.compare_outputs(original,original,keys)['mismatch_count'],0)
        for actual in [original.replace(b'163',b'58'), original.replace(b'index\t1',b'index\t2'),
                       b'',original[:-1],original+original.splitlines(keepends=True)[0],
                       b''.join(original.splitlines(keepends=True)[::-1]),
                       original.replace(b'255,3,7,11',b'0,0,0,0')]:
            self.assertGreater(runner.compare_outputs(original,actual,keys)['mismatch_count'],0)

    def test_missing_original_jar_is_error_not_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            path=root/'tests/fixtures/chunk/layout.tsv'
            path.parent.mkdir(parents=True)
            path.write_bytes(b'x\tget:0:0:0')
            report,_=runner.run_rng(root,suite='chunk')
            self.assertEqual(report['status'],'error')
            self.assertIn('JAR not configured',report['reason'])
            self.assertEqual(runner.exit_code(report['status'],True),1)

if __name__=='__main__':unittest.main()
