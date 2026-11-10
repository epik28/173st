from pathlib import Path
import unittest

from tools.oracle import water_tick_corpus
from tools.parity import water_tick_run


class WaterTickRunnerTests(unittest.TestCase):
    def test_probe_input_preserves_full_loaded_state_and_native_queue_order(self):
        case = water_tick_corpus.fixture_check()["cases"][4]
        step = case["steps"][1]
        raw = water_tick_run._inputs(case, step, case["steps"][0]["state"])
        lines = raw.decode("ascii").splitlines()
        self.assertEqual(sum(line.startswith("cell\t") for line in lines), 100)
        self.assertEqual(
            [line for line in lines if line.startswith("queue\t")],
            ["queue\t" + "\t".join(map(str, row)) for row in step["queue_before"]],
        )
        self.assertEqual(lines[-1], f"run\t{step['world_time']}\t64")

    def test_expected_stream_keeps_native_event_order_queue_and_full_state(self):
        case = water_tick_corpus.fixture_check()["cases"][4]
        step = case["steps"][1]
        lines = water_tick_run._expected(step).decode("ascii").splitlines()
        event_lines = ["\t".join(row) for row in step["events"]]
        self.assertEqual(lines[0], "result\tprocessed_ticks=2")
        event_start = 1
        self.assertEqual(lines[event_start:event_start + len(event_lines)], event_lines)
        queue_index = event_start + len(event_lines)
        expected_queue = ",".join(":".join(map(str, row)) for row in step["queue_after"])
        self.assertEqual(lines[queue_index], f"queue_after\t{expected_queue}")
        states = lines[queue_index + 1:]
        self.assertEqual(len(states), 100)
        self.assertEqual(states[0].split("\t")[1:4], ["-2", "65", "-2"])
        self.assertEqual(states[-1].split("\t")[1:4], ["2", "62", "2"])

    def test_expected_state_output_is_mutation_sensitive(self):
        case = water_tick_corpus.fixture_check()["cases"][0]
        expected = water_tick_run._expected(case["steps"][0])
        altered = expected.replace(b"result\tprocessed_ticks=1\n", b"result\tprocessed_ticks=0\n")
        self.assertNotEqual(altered, expected)

    def test_source_formation_extension_uses_same_exact_step_schema(self):
        dataset = water_tick_corpus.source_fixture_check()
        self.assertEqual(dataset["case_count"], 4)
        case = dataset["cases"][0]
        step = case["steps"][0]
        raw = water_tick_run._inputs(case, step, None)
        self.assertEqual(sum(line.startswith(b"cell\t") for line in raw.splitlines()), 100)
        self.assertTrue(water_tick_run._expected(step).startswith(b"result\tprocessed_ticks="))
        self.assertEqual(len(water_tick_run._expected(step).splitlines()), 102 + len(step["events"]))

    def test_runner_source_manifest_covers_probe_fixture_and_native_adapter(self):
        hashes = water_tick_run._hash_sources(Path(water_tick_corpus.ROOT))
        self.assertIn("crates/beta-core/examples/water_tick_probe.rs", hashes)
        self.assertIn("tests/fixtures/water_tick/native.tsv", hashes)
        self.assertIn("tests/fixtures/water_tick_sources/native.tsv", hashes)
        self.assertIn("tools/oracle/src/WaterTickOracle.java", hashes)
        self.assertTrue(all(len(value) == 64 for value in hashes.values()))


if __name__ == "__main__":
    unittest.main()
