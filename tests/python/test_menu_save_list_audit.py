import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools.oracle import menu_save_list_audit as audit


class MenuSaveListAuditTest(unittest.TestCase):
    def test_pinned_native_save_reader(self):
        fixture = audit.verify()
        self.assertEqual(len(fixture["cases"]), 4)
        output = audit.EXPECTED.read_text(encoding="utf-8")
        self.assertEqual(output.count("world\t"), 3)
        self.assertIn("MenuConvert\tNative Conversion", output)
        self.assertIn("MenuFallback\tNative Backup", output)
        self.assertNotIn("MenuInvalid", output)

    def test_save_fixture_mutation_is_rejected(self):
        fixture = json.loads(audit.FIXTURE.read_text(encoding="utf-8"))
        fixture["native_contract"]["row_filter"] = "show every directory"
        with tempfile.TemporaryDirectory() as directory:
            candidate = Path(directory) / "synthetic_saves.json"
            candidate.write_text(json.dumps(fixture), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "synthetic save fixture hash"):
                audit.load_fixture(candidate)

    def test_native_output_mutation_is_rejected(self):
        original = audit.EXPECTED.read_text(encoding="utf-8")
        with tempfile.TemporaryDirectory() as directory:
            candidate = Path(directory) / "synthetic_save_list.tsv"
            candidate.write_text(original.replace("MenuConvert", "MenuChanged"), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "saved native save-list reference hash"):
                audit.verify(expected_path=candidate)


if __name__ == "__main__":
    unittest.main()
