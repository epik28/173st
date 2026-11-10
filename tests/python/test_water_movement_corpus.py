"""Structural and pinned-native checks for the in-water player movement fixture."""
import json
from pathlib import Path
import re
import struct
import unittest

from tools.oracle import water_movement_corpus as water

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/water_movement"
RAW64 = re.compile(r"^[0-9a-f]{16}$")
RAW32 = re.compile(r"^[0-9a-f]{8}$")


def f64(raw):
    return struct.unpack(">d", bytes.fromhex(raw))[0]


class WaterMovementCorpusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.input = (FIXTURE / "input.tsv").read_bytes()
        cls.reference = (FIXTURE / "reference.tsv").read_bytes()
        cls.audit = json.loads((FIXTURE / "audit.json").read_text())
        cls.inputs = [line.decode("ascii").split("\t") for line in cls.input.splitlines()]
        cls.outputs = [line.decode("ascii").split("\t") for line in cls.reference.splitlines()]

    def test_input_is_reproducible_bounded_and_covers_requested_water_states(self):
        self.assertEqual(water.corpus(), self.input)
        self.assertLessEqual(len(self.input), 4 * 1024 * 1024)
        self.assertLessEqual(len(self.inputs), 512)
        self.assertEqual(self.input[-1:], b"\n")
        self.assertEqual(len({row[0] for row in self.inputs}), len(self.inputs))
        by_id = {row[0]: row for row in self.inputs}
        self.assertEqual(set(by_id), {
            "entry-side", "submerged-diagonal-release", "vertical-jump-and-coast",
            "surface-feet-wet-eyes-dry", "leave-water-to-shore", "wall-and-floor",
            "partial-fluid-level",
        })
        self.assertTrue(any("8," in row[2] for row in self.inputs))
        self.assertTrue(any("9," in row[2] for row in self.inputs))
        self.assertTrue(any(",4" in cell for row in self.inputs for cell in row[2].split(";")))
        for row in self.inputs:
            self.assertEqual(len(row), 5)
            self.assertEqual(len(row[1].split(",")), 18)
            self.assertLessEqual(len(row[2].split(";")), 512)
            self.assertLessEqual(len(row[3].split(":")), 128)
            self.assertLessEqual(len(row[4].split(";")), 256)
            for raw in row[1].split(",")[:12]:
                self.assertRegex(raw, RAW64)
            for cell in row[2].split(";"):
                if cell == "-":
                    continue
                values = cell.split(",")
                self.assertEqual(len(values), 5)
                self.assertIn(values[3], {"8", "9"})
                self.assertLessEqual(int(values[4]), 15)

    def test_output_has_full_raw_player_state_water_probes_and_entry_exit_edges(self):
        expected_count = sum(len(row[4].split(";")) for row in self.inputs)
        self.assertEqual(len(self.outputs), expected_count)
        expected_keys = [(row[0], str(i)) for row in self.inputs
                         for i, _ in enumerate(row[4].split(";"))]
        actual_keys = []
        states = {}
        for row in self.outputs:
            self.assertEqual(len(row), 7)
            self.assertRegex(row[1], r"^(0|[1-9][0-9]*)$")
            values = row[2].split(",")
            self.assertEqual(len(values), 18)
            for raw in values[:12]:
                self.assertRegex(raw, RAW64)
            for raw in values[12:17]:
                self.assertRegex(raw, RAW32)
            self.assertIn(row[3], {"0", "1"})
            self.assertIn(row[4], {"0", "1"})
            self.assertRegex(row[5], r"^[0-9]+$")
            self.assertRegex(row[6], r"^[0-9]+$")
            actual_keys.append((row[0], row[1]))
            states.setdefault(row[0], []).append(row)
        self.assertEqual(actual_keys, expected_keys)
        self.assertEqual(states["entry-side"][0][3], "0")
        self.assertIn("1", [row[3] for row in states["entry-side"][1:]])
        self.assertIn("0", [row[3] for row in states["leave-water-to-shore"]])
        self.assertEqual({row[4] for row in states["surface-feet-wet-eyes-dry"]}, {"0"})
        self.assertTrue(all(row[3] == "1" for row in states["submerged-diagonal-release"]))
        self.assertGreater(f64(states["vertical-jump-and-coast"][0][2].split(",")[4]), 0.0)

    def test_audit_binds_original_classes_methods_invocations_and_exact_bytes(self):
        audit = self.audit
        self.assertEqual(audit["schema_version"], 1)
        self.assertEqual(audit["suite"], "water-movement")
        self.assertEqual(audit["jar_sha256"], water.JAR_SHA)
        classes, methods = water.source_audit(water.paths())
        self.assertEqual(audit["class_sha256"], classes)
        self.assertEqual(audit["method_code_sha256"], methods)
        self.assertEqual(audit["input_sha256"], water.sha(self.input))
        self.assertEqual(audit["output_sha256"], water.sha(self.reference))
        self.assertEqual(audit["repeat_output_sha256"], audit["output_sha256"])
        self.assertEqual(audit["oracle_sha256"], water.sha(water.JAVA_SOURCE.read_bytes()))
        self.assertEqual(audit["native_world_constructors"], 0)
        self.assertEqual(audit["environment"], "synthetic-sparse-water-and-solid-boxes")
        self.assertEqual(audit["state_count"], len(self.outputs))
        self.assertEqual(audit["case_count"], len(self.inputs))
        invocations = audit["invocations"]
        self.assertEqual(invocations["entity_update"], len(self.outputs))
        self.assertEqual(invocations["player_travel"], len(self.outputs))
        self.assertEqual(invocations["jump"], 1)
        self.assertGreater(invocations["water_box_queries"], 0)
        self.assertGreater(invocations["native_water_queries"], 0)
        self.assertEqual(invocations["water_contact_queries"], len(self.outputs))
        self.assertGreater(invocations["metadata_queries"], 0)
        self.assertEqual(set(audit["code_sources"]), set(classes))
        self.assertEqual({Path(source.removeprefix("file:")).resolve()
                          for source in audit["code_sources"].values()},
                         {water.paths()})


if __name__ == "__main__":
    unittest.main()
