import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("water_tick_corpus", ROOT / "tools/oracle/water_tick_corpus.py")
CORPUS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CORPUS)


class WaterTickCorpusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.result = CORPUS.fixture_check()
        cls.cases = {case["name"]: case for case in cls.result["cases"]}

    def test_fixture_is_pinned_repeatable_and_memory_only(self):
        self.assertEqual(self.result["case_count"], 9)
        self.assertGreater(self.result["events"], 50)
        self.assertTrue(self.result["repeatable"])
        self.assertEqual(CORPUS.JAR_SHA, "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c")

    def test_waterfall_native_metadata_and_blocked_flow_transitions(self):
        open_state = self.cases["flow-open"]["steps"][0]["state"]
        fall_state = self.cases["downward-open"]["steps"][0]["state"]
        decay_state = self.cases["flow-decay"]["steps"][0]["state"]
        blocked_state = self.cases["blocked-down-and-sides"]["steps"][0]["state"]
        self.assertEqual(open_state["0,63,0"], (8, 8))
        self.assertEqual(fall_state["0,63,0"], (8, 7))
        self.assertEqual(decay_state["0,64,0"], (0, 0))
        self.assertEqual(blocked_state["0,64,0"], (9, 0))

    def test_horizontal_spread_is_captured_after_the_second_native_tick(self):
        case = self.cases["horizontal-basin"]
        self.assertEqual(len(case["steps"]), 2)
        state = case["steps"][1]["state"]
        self.assertEqual(state["-1,64,0"], (8, 1))
        self.assertEqual(state["1,64,0"], (8, 1))
        self.assertEqual(state["0,64,-1"], (8, 1))
        self.assertEqual(state["0,64,1"], (8, 1))

    def test_same_due_time_queue_order_is_frozen(self):
        case = self.cases["two-tick-spread"]
        queue = case["steps"][1]["queue_after"]
        self.assertEqual([row[:4] for row in queue], [
            (0, 62, 0, 8), (0, 63, 0, 8), (-1, 64, 0, 8),
            (0, 64, 0, 8), (1, 64, 0, 8), (0, 64, -1, 8), (0, 64, 1, 8),
        ])
        self.assertEqual({row[4] for row in queue}, {17})

    def test_source_probe_and_adjacent_sources_are_exact_synthetic_inputs(self):
        still = self.cases["still-source"]
        self.assertTrue(any(event.startswith("probe_schedule\t0\t64\t0\t9\t0") for event in still["setup_events"]))
        self.assertEqual(still["steps"][0]["state"]["0,64,0"], (9, 0))
        multiple = self.cases["two-adjacent-sources"]
        self.assertIn((0, 64, 0, 8, 0), multiple["inputs"])
        self.assertIn((-1, 64, 0, 9, 0), multiple["inputs"])
        self.assertIn((1, 64, 0, 9, 0), multiple["inputs"])

    def test_queue_and_neighbor_transcripts_are_present_and_ordered(self):
        case = self.cases["flow-open"]
        self.assertEqual(case["setup_queue"][0][:4], (0, 64, 0, 8))
        self.assertEqual(case["setup_queue"][0][4], 5)
        step = case["steps"][0]
        self.assertGreater(step["world_time"], step["queue_before"][0][4])
        self.assertTrue(any(event[0] == "schedule" and event[-1] == "5" for event in step["events"]))
        self.assertTrue(any(event[0] == "neighbor" for event in step["events"]))
        self.assertTrue(step["queue_after"])

    def test_live_mutation_cannot_be_mistaken_for_the_frozen_fixture(self):
        raw = (CORPUS.FIXTURES / "native.tsv").read_bytes()
        changed = raw.replace(b"#WATER_TICK_NATIVE\t1", b"#WATER_TICK_NATIVE\t9", 1)
        with self.assertRaises(ValueError):
            CORPUS.parse_native(changed)
        with self.assertRaises(ValueError):
            CORPUS.fixture_check(raw[:-1])

    def test_source_extension_is_separate_and_keeps_c01_bytes_frozen(self):
        sources = CORPUS.source_fixture_check()
        self.assertEqual(sources["case_count"], 4)
        self.assertEqual(sources["extension_version"], "water-source-formation-1")
        self.assertEqual(sources["native_sha256"], "267fe2da3d911c30ebbc457ab1217ba3cefba1b409089fa06d2f12a14294108f")
        self.assertEqual(CORPUS.fixture_check()["native_sha256"],
                         "cf829ee9aa63bdf2ace25104c36e9cc84eb9c6b1e92c1607573e8f18008f5276")
        self.assertTrue(sources["repeatable"])

    def test_supported_two_neighbor_flow_becomes_level_zero_then_still_water(self):
        case = {row["name"]: row for row in CORPUS.source_fixture_check()["cases"]}["two-neighbors-supported-meta1"]
        self.assertEqual(case["inputs"], [
            (0, 64, 0, 8, 1), (-1, 64, 0, 9, 0), (1, 64, 0, 9, 0),
            (0, 63, 0, 1, 0), (0, 64, -1, 1, 0), (0, 64, 1, 1, 0),
        ])
        self.assertEqual(case["steps"][0]["state"]["0,64,0"], (8, 0))
        self.assertEqual(case["steps"][0]["state"]["-1,64,0"], (8, 0))
        self.assertEqual(case["steps"][0]["state"]["1,64,0"], (8, 0))
        self.assertEqual(case["steps"][1]["state"]["0,64,0"], (9, 0))
        self.assertEqual(case["steps"][1]["queue_before"], [
            (-1, 64, 0, 8, 11), (1, 64, 0, 8, 11), (0, 64, 0, 8, 11),
        ])

    def test_one_neighbor_and_unsupported_controls_are_frozen(self):
        cases = {row["name"]: row for row in CORPUS.source_fixture_check()["cases"]}
        one = cases["one-neighbor-supported-meta1"]["steps"][0]["state"]
        self.assertEqual(one["0,64,0"], (9, 1))
        unsupported = cases["two-neighbors-unsupported-meta1"]["steps"][0]["state"]
        self.assertEqual(unsupported["0,64,0"], (8, 1))
        self.assertEqual(unsupported["0,63,0"], (8, 9))
        below_source = cases["two-neighbors-source-below-meta1"]["steps"][0]["state"]
        self.assertEqual(below_source["0,64,0"], (9, 1))


if __name__ == "__main__":
    unittest.main()
