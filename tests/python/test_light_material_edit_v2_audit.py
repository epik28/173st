from __future__ import annotations
import hashlib, importlib.util, json, pathlib, unittest

ROOT=pathlib.Path(__file__).resolve().parents[2]
SPEC=importlib.util.spec_from_file_location('light_material_edit_v2_audit',ROOT/'tools/oracle/light_material_edit_v2_audit.py')
audit=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(audit)
FIXTURE=ROOT/'tests/fixtures/light_material/native_edits_v2.tsv'
MANIFEST=ROOT/'tests/fixtures/light_material/edit_v2_audit.json'

class LightMaterialEditV2AuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=FIXTURE.read_bytes();cls.parsed=audit.validate(cls.data)

    def test_frozen_provenance_and_two_native_captures(self):
        manifest=json.loads(MANIFEST.read_text())
        digest=hashlib.sha256(self.data).hexdigest()
        self.assertEqual(digest,manifest['output_sha256'])
        self.assertEqual([digest,digest],manifest['two_capture_output_sha256'])
        self.assertEqual('af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c',manifest['jar_sha256'])
        self.assertEqual(len(self.data.decode('ascii').splitlines()),manifest['row_count'])
        self.assertEqual(len(self.data),manifest['byte_count'])

    def test_complete_ordered_phases_and_native_state_diffs(self):
        expected={'canopy-log-sequence','leaf-attenuation','water-attenuation','glowstone-loaded-chunk-edge','opaque-overhead-place-remove'}
        self.assertEqual(expected,set(self.parsed['cases']))
        for case in self.parsed['cases'].values():
            self.assertEqual(['before','after-place','after-remove'],case['phases'])
            self.assertEqual(2,case['steps'])
        self.assertGreater(self.parsed['counts']['glowstone-loaded-chunk-edge']['deltas'],9000)
        self.assertEqual(2,self.parsed['counts']['glowstone-loaded-chunk-edge']['hdeltas'])

    def test_opaque_overhead_native_light_is_captured_at_and_below_edit(self):
        rows=[line.split('\t') for line in self.data.decode('ascii').splitlines() if line.startswith('sample\topaque-overhead-place-remove\t')]
        samples={(r[2],int(r[3]),int(r[4]),int(r[5])):tuple(map(int,r[6:])) for r in rows}
        self.assertEqual(15,samples[('before',8,64,8)][4])
        self.assertEqual(1,samples[('after-place',8,65,8)][0])
        self.assertLess(samples[('after-place',8,64,8)][4],15)
        self.assertEqual(samples[('before',8,64,8)][4],samples[('after-remove',8,64,8)][4])

    def test_height_coordinates_keep_asymmetric_native_column(self):
        case = self.parsed['cases']['glowstone-loaded-chunk-edge']
        height = case['height']['after-place']
        self.assertEqual(65, height[(15, 8)])
        self.assertNotIn((8, 15), height)
        self.assertEqual((0, 65), case['hdeltas'][('before', 'after-place')][(15, 8)])
        self.assertNotIn((8, 15), case['hdeltas'][('before', 'after-place')])

    def test_glowstone_native_emission_crosses_chunk_edge(self):
        rows=[line.split('\t') for line in self.data.decode('ascii').splitlines() if line.startswith('sample\tglowstone-loaded-chunk-edge\tafter-place\t')]
        by_pos={(int(r[3]),int(r[4]),int(r[5])):tuple(map(int,r[6:])) for r in rows}
        self.assertEqual((89,0,255,15,0,15,15),by_pos[(15,64,8)])
        self.assertEqual((0,0,0,0,15,14,15),by_pos[(14,64,8)])
        self.assertEqual((0,0,0,0,15,14,15),by_pos[(16,64,8)])
        self.assertEqual((0,0,0,0,15,13,15),by_pos[(13,64,8)])
        self.assertEqual((0,0,0,0,15,13,15),by_pos[(17,64,8)])

    def test_domain_defaults_and_seed_mutation_rejected(self):
        text=self.data.decode('ascii')
        changed=text.replace('default_sky=0@y0;15@y1..127(native-lm.c-all-air)','default_sky=15',1).encode()
        with self.assertRaisesRegex(ValueError,'domain contract'):
            audit.validate(changed)
        seed=next(line for line in text.splitlines() if line.startswith('seed\t'))
        changed=text.replace(seed,seed.replace('\t17\t0','\t18\t0',1),1).encode()
        with self.assertRaisesRegex(ValueError,'seed differs'):
            audit.validate(changed)

    def test_snapshot_hash_or_delta_mutation_rejected(self):
        text=self.data.decode('ascii')
        snapshot=next(line for line in text.splitlines() if line.startswith('snapshot\t'))
        changed=text.replace(snapshot,snapshot[:-1]+('0' if snapshot.endswith('1') else '1'),1).encode()
        with self.assertRaisesRegex(ValueError,'before snapshot digest'):
            audit.validate(changed)
        delta=next(line for line in text.splitlines() if line.startswith('delta\t'))
        fields=delta.split('\t');fields[-1]=str((int(fields[-1])+1)%16)
        changed=text.replace(delta,'\t'.join(fields),1).encode()
        with self.assertRaisesRegex(ValueError,'native cell deltas'):
            audit.validate(changed)
        state=next(line for line in text.splitlines() if line.startswith('state\t'))
        fields=state.split('\t');fields[6]=str((int(fields[6])+1)%256)
        mutated=text.replace(state,'\t'.join(fields),1).encode()
        with self.assertRaisesRegex(ValueError,'native snapshot digest'):
            audit.validate(mutated)

    def test_jvm_warning_on_stdout_is_rejected(self):
        warning = b'[0.001s][warning][perf,memops] lock failure\n'
        with self.assertRaises(ValueError):
            audit.validate(warning + self.data)

    def test_wrong_jar_and_bounded_protocol_rejected(self):
        with self.assertRaisesRegex(ValueError,'wrong or missing pinned'):
            audit.pinned_jar(pathlib.Path('/definitely/not/a/client.jar'))
        for data in (b'',b'material-edits-v2\nextra\n',b'material-edits-v2'):
            with self.assertRaises(Exception):
                audit.validate(data)

if __name__=='__main__': unittest.main()
