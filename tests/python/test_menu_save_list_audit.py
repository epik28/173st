import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools.oracle import menu_save_list_audit as audit
from tools.oracle.native_menu_capture import PROFILES, gui_to_physical, native_first_row_top


class MenuSaveListAuditTest(unittest.TestCase):
    def test_pinned_native_save_reader(self):
        fixture = audit.verify()
        self.assertEqual(len(fixture["cases"]), 4)
        output = audit.EXPECTED.read_text(encoding="utf-8")
        self.assertEqual(output.count("world\t"), 3)
        self.assertIn("MenuConvert\tNative Conversion", output)
        self.assertIn("MenuFallback\tNative Backup", output)
        self.assertNotIn("MenuInvalid", output)

    def test_native_short_list_first_row_hit_coordinates(self):
        row_count = 3
        expected = [(172, 190, 190), (52, 70, 210)]
        for profile, (top, logical_y, physical_y) in zip(PROFILES, expected):
            with self.subTest(profile=profile["name"]):
                actual_top = native_first_row_top(profile["scaled_height"], row_count)
                self.assertEqual(actual_top, top)
                self.assertEqual(actual_top + 18, logical_y)
                self.assertEqual(gui_to_physical(profile, profile["scaled_width"] // 2, logical_y)[1],
                                 physical_y)

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
