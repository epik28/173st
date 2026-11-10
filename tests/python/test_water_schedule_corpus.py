import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "water_schedule_corpus", ROOT / "tools/oracle/water_schedule_corpus.py")
CORPUS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CORPUS)


class WaterScheduleCorpusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.result = CORPUS.fixture_check()
        cls.cases = {case["name"]: case for case in cls.result["cases"]}

    def test_separate_fixture_binds_two_identical_memory_only_captures(self):
        audit = json.loads((CORPUS.FIXTURES / "audit.json").read_text())
        self.assertEqual(self.result["extension_version"], "water-tick-seeding-1")
        self.assertEqual(self.result["case_count"], 3)
        self.assertEqual(audit["captures"], 2)
        self.assertTrue(audit["repeat_identical"])
        self.assertFalse(audit["save_access"])

    def test_initial_flowing_water_placement_schedules_one_due_tick(self):
        case = self.cases["initial-flow-placement"]
        self.assertEqual(case["queue_before"], [])
        self.assertEqual(case["events"], [
            ("schedule", "0", "64", "0", "8", "5"),
            ("edit", "0", "64", "0", "0", "0", "8", "0"),
        ])
        self.assertEqual(case["queue_after"], [(0, 64, 0, 8, 5)])
        self.assertEqual(case["state"]["0,64,0"], (8, 0))

    def test_adjacent_block_placement_notifies_still_water_and_seeds_flow(self):
        case = self.cases["place-adjacent-solid-neighbor"]
        self.assertEqual(case["setup_events"][-1], ("edit", "0", "64", "0", "0", "0", "9", "0"))
        self.assertEqual(case["events"], [
            ("edit", "1", "64", "0", "0", "0", "1", "0"),
            ("neighbor", "1", "64", "0", "1"),
            ("schedule", "0", "64", "0", "8", "5"),
            ("edit", "0", "64", "0", "9", "0", "8", "0"),
            ("schedule", "0", "64", "0", "8", "5"),
        ])
        self.assertEqual(case["queue_after"], [(0, 64, 0, 8, 5)])

    def test_support_removal_notifies_still_water_and_seeds_flow(self):
        case = self.cases["remove-still-water-support"]
        self.assertEqual(case["events"][0], ("edit", "0", "63", "0", "1", "0", "0", "0"))
        self.assertEqual(case["events"][1], ("neighbor", "0", "63", "0", "0"))
        self.assertEqual(case["events"][2:], [
            ("schedule", "0", "64", "0", "8", "5"),
            ("edit", "0", "64", "0", "9", "0", "8", "0"),
            ("schedule", "0", "64", "0", "8", "5"),
        ])
        self.assertEqual(case["queue_after"], [(0, 64, 0, 8, 5)])


if __name__ == "__main__":
    unittest.main()
