from __future__ import annotations
import hashlib, importlib.util, json, pathlib, unittest

ROOT=pathlib.Path(__file__).resolve().parents[2]
SPEC=importlib.util.spec_from_file_location('light_material_edit_settled_audit',ROOT/'tools/oracle/light_material_edit_settled_audit.py')
audit=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(audit)
FIXTURE=ROOT/'tests/fixtures/light_material/native_edits_settled_v1.tsv'
MANIFEST=ROOT/'tests/fixtures/light_material/edit_settled_v1_audit.json'

class SettledLightBaselineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=FIXTURE.read_bytes();cls.parsed=audit.validate(cls.data)
        cls.manifest=json.loads(MANIFEST.read_text())

    def test_native_chunkwise_relight_is_declared_and_reproducible(self):
        digest=hashlib.sha256(self.data).hexdigest()
        self.assertEqual(digest,self.manifest['output_sha256'])
        self.assertEqual([digest,digest],self.manifest['two_capture_output_sha256'])
        self.assertEqual('af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c',self.manifest['jar_sha256'])
        for case in self.parsed['cases'].values():
            self.assertEqual('15@y0..127(native-chunkwise-relight)',case['descriptor']['default_sky'])
            self.assertEqual('fd.a(eb.a,x0,0,z0,x0+15,127,z0+15);fd.j-for-each-chunk',case['descriptor']['baseline'])

    def test_settled_empty_world_defaults_to_sky_15_at_bottom(self):
        case=self.parsed['cases']['leaf-attenuation']
        self.assertEqual('2b8c32120a400725cd24cee1cc99b7375d148361baabbd523a44bfa61a809289',case['snap']['before'])
        for x,z in ((-15,-15),(0,0),(15,15),(-1,15)):
            self.assertEqual((0,0,0,15),case['state'].get('before',{}).get((x,0,z),(0,0,0,15)))

    def test_default_protocol_mutation_is_rejected(self):
        text=self.data.decode('ascii')
        bad=text.replace('15@y0..127(native-chunkwise-relight)','0@y0;15@y1..127',1).encode()
        with self.assertRaisesRegex(ValueError,'domain contract'):
            audit.validate(bad)

    def test_wrong_jar_and_protocol_are_rejected(self):
        with self.assertRaisesRegex(ValueError,'wrong or missing pinned'):
            audit.pinned_jar(pathlib.Path('/definitely/not/a/client.jar'))
        with self.assertRaises(Exception):
            audit.validate(b'material-edits-settled-v1\nextra\n')

if __name__=='__main__': unittest.main()
