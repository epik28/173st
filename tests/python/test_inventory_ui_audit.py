import json
from pathlib import Path
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/oracle"))
import inventory_ui_audit  # noqa: E402


class InventoryUiNativeAuditTests(unittest.TestCase):
    def test_pinned_resources_layout_and_native_item_indices(self):
        jar = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
        fixture = inventory_ui_audit.verify(jar)
        self.assertEqual(fixture["behavior"]["inventory_panel"]["size"], [176, 166])
        self.assertEqual(len(fixture["behavior"]["main_slots"]["positions"]), 36)
        self.assertEqual(fixture["runtime_oracle"]["expected_icon_rows"], inventory_ui_audit.EXPECTED_ICONS)

    def test_rejects_wrong_jar(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "wrong.jar"
            path.write_bytes(b"not the pinned Beta client")
            with self.assertRaisesRegex(ValueError, "JAR hash/size"):
                inventory_ui_audit.verify(path)

    def test_rejects_altered_fixture_claims(self):
        mutations = (
            ("resource hash", lambda value: value["reference"]["resources"]["gui/items.png"].__setitem__("sha256", "0" * 64), "resource or class hashes"),
            ("class hash", lambda value: value["reference"]["classes"].__setitem__("ue", "0" * 64), "resource or class hashes"),
            ("open key", lambda value: value["behavior"].__setitem__("open", "Escape opens"), "key behavior"),
            ("Escape close", lambda value: value["behavior"].__setitem__("close", "E closes"), "key behavior"),
            ("panel size", lambda value: value["behavior"]["inventory_panel"]["size"].__setitem__(0, 177), "panel dimensions"),
            ("slot coordinate", lambda value: value["behavior"]["main_slots"]["positions"][0].__setitem__(0, 9), "slot geometry"),
            ("slot count", lambda value: value["behavior"]["main_slots"].__setitem__("slot_count", 35), "slot geometry"),
            ("icon index", lambda value: value["behavior"]["item_icons"]["mapping"]["267"].__setitem__("tile_index", 67), "icon mapping"),
            ("native descriptor", lambda value: value["reference"]["native_methods"]["inventory_panel_draw"].__setitem__("descriptor", "()V"), "method provenance"),
            ("bytecode", lambda value: value["bytecode_evidence"].__setitem__("open_screen", "changed"), "bytecode evidence"),
            ("runtime oracle", lambda value: value["runtime_oracle"]["expected_icon_rows"][0].__setitem__(2, 99), "runtime icon oracle"),
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audit.json"
            for name, mutate, message in mutations:
                with self.subTest(name=name):
                    fixture = json.loads(inventory_ui_audit.FIXTURE.read_text(encoding="utf-8"))
                    mutate(fixture)
                    path.write_text(json.dumps(fixture), encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, message):
                        inventory_ui_audit.load_fixture(path)

    def test_rejects_malformed_fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audit.json"
            path.write_text('{"schema_version":', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "valid inventory UI fixture"):
                inventory_ui_audit.load_fixture(path)


if __name__ == "__main__":
    unittest.main()
