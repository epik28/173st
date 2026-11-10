import unittest
from pathlib import Path

from tools.oracle import water_schedule_corpus
from tools.parity import water_schedule_run


class WaterScheduleRunnerTests(unittest.TestCase):
    def test_expected_stream_preserves_event_queue_and_full_state_order(self):
        native = water_schedule_corpus.fixture_check()
        output = water_schedule_run.expected_stream(native).decode("ascii")
        self.assertIn("schedule\t0\t64\t0\t8\t5\n", output)
        self.assertIn("queue_after\t0:64:0:8:5\n", output)
        self.assertEqual(output.count("#case_end\t"), 3)
        first_state = next(line for line in output.splitlines() if line.startswith("state\t"))
        self.assertEqual(first_state.split("\t", 1)[1].split(";")[:4], [
            "-1,65,-1=0:0", "0,65,-1=0:0", "1,65,-1=0:0", "-1,65,0=0:0"
        ])

    def test_native_event_and_queue_mutations_change_expected_stream(self):
        native = water_schedule_corpus.fixture_check()
        original = water_schedule_run.expected_stream(native)
        mutated = {**native, "cases": [dict(case) for case in native["cases"]]}
        mutated["cases"][1]["events"] = list(reversed(mutated["cases"][1]["events"]))
        self.assertNotEqual(original, water_schedule_run.expected_stream(mutated))
        mutated = {**native, "cases": [dict(case) for case in native["cases"]]}
        mutated["cases"][1]["queue_after"] = []
        self.assertNotEqual(original, water_schedule_run.expected_stream(mutated))

    def test_runner_manifest_binds_app_runtime_probe_and_native_fixture(self):
        paths = set(water_schedule_run.SOURCES)
        for expected in (
            Path("crates/beta-viewer/src/main.rs"),
            Path("crates/beta-viewer/src/water_runtime.rs"),
            Path("crates/beta-viewer/examples/water_schedule_probe.rs"),
            Path("tests/fixtures/water_tick_schedule/native.tsv"),
            Path("tools/oracle/src/WaterTickOracle.java"),
        ):
            self.assertIn(expected, paths)


if __name__ == "__main__":
    unittest.main()
