import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"tools/oracle"))
import hotbar_items_audit as audit

class HotbarItemsAuditTests(unittest.TestCase):
    def test_native_hotbar_evidence_and_repeat_oracle(self):
        fixture=audit.verify(ROOT.parent/".local/minecraft/client-b1.7.3.jar")
        self.assertEqual(fixture["rows"][2][7],"/gui/items.png")
        self.assertEqual(fixture["rows"][0][7],"/terrain.png")
        self.assertEqual(fixture["digit_widths"],audit.WIDTHS)

    def test_rejects_native_contract_mutations(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"audit.json"
            for change in (lambda x:x["resources"].__setitem__("terrain.png","0"*64),
                           lambda x:x["bytecode_evidence"].__setitem__("atlas_threshold","changed"),
                           lambda x:x["rows"][2].__setitem__(7,"/terrain.png"),
                           lambda x:x["digit_widths"].__setitem__(4,8)):
                v=json.loads(audit.FIXTURE.read_text()); change(v); p.write_text(json.dumps(v))
                with self.assertRaisesRegex(ValueError,"fixture differs"):
                    audit.load_fixture(p)

    def test_rejects_wrong_jar(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"wrong.jar"; p.write_bytes(b"wrong")
            with self.assertRaisesRegex(ValueError,"JAR hash"):
                audit.verify(p)

if __name__=="__main__": unittest.main()
