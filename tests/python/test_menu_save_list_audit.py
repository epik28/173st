import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools.oracle import menu_save_list_audit as audit
from tools.oracle import native_menu_capture as capture
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

    def test_synthetic_capture_sequence_uses_defined_native_row_coordinates(self):
        class FakeProcess:
            def __init__(self):
                self.returncode = None

            def poll(self):
                return self.returncode

            def terminate(self):
                self.returncode = 0

            def wait(self, timeout=None):
                self.returncode = 0
                return 0

        class FakeXSession:
            def __init__(self):
                pass

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return None

            def focus_and_click(self, window_id, x, y):
                clicks.append((window_id, x, y))

            def focus_and_press_key(self, window_id, keysym):
                keys.append((window_id, keysym))

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            context = {
                "game_home": root / "home",
                "game_cwd": root / "cwd",
                "native_dir": root / "natives",
                "classpath": "pinned-client-and-libraries",
                "save_set": "synthetic",
                "synthetic_save_fixture": json.loads(audit.FIXTURE.read_text(encoding="utf-8")),
            }
            (context["game_home"] / ".minecraft").mkdir(parents=True)
            context["game_cwd"].mkdir()
            output = root / "output"
            output.mkdir()
            clicks, keys = [], []
            for profile, expected_gui_y, expected_physical_y in zip(PROFILES, (190, 70), (190, 210)):
                with self.subTest(profile=profile["name"]):
                    dummy = FakeProcess()
                    with patch.object(capture.subprocess, "Popen", return_value=dummy), \
                         patch.object(capture, "await_window", return_value={"id": "0x40000c", "width": profile["width"], "height": profile["height"]}), \
                         patch.object(capture, "set_window_size", return_value=(profile["width"], profile["height"])), \
                         patch.object(capture, "capture_record", side_effect=lambda _wid, path, p: {"path": str(path), "sha256": "0" * 64, "bytes": 24, "dimensions": [p["width"], p["height"]]}), \
                         patch.object(capture, "XSession", FakeXSession), \
                         patch.object(capture.time, "sleep"):
                        result = capture.launch_profile(context, profile, output, 45)
                    self.assertEqual(result["world_row_click"]["scaled_gui_xy"][1], expected_gui_y)
                    self.assertEqual(result["world_row_click"]["physical_xy"][1], expected_physical_y)
                    self.assertEqual(result["play_button_clicked"], False)
            self.assertEqual(len(clicks), 8)  # Singleplayer, row, Cancel, Singleplayer for two profiles.
            self.assertEqual(keys, [(0x40000c, 0xff0d), (0x40000c, 0xff1b)] * 2)

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
