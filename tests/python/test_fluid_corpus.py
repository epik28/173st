"""FLUID-001 native water geometry fixture and opt-in direct-client capture."""
import os
from pathlib import Path
import unittest

from tools.oracle import fluid_corpus


class FluidFixtureTests(unittest.TestCase):
    def test_committed_native_vertices_are_well_formed(self):
        report = fluid_corpus.fixture_check()
        self.assertEqual(report["cases"], 44)
        self.assertGreater(report["vertices"], 1000)
        self.assertEqual(report["per_case_vertices"]["level-9-0"], 30)
        self.assertEqual(report["per_case_vertices"]["flow-column"], 30)
        self.assertEqual((fluid_corpus.FIXTURES / "input.tsv").read_text("ascii"),
                         fluid_corpus.SUITE + "\n")

    def test_mutated_native_vertex_is_rejected(self):
        raw = (fluid_corpus.FIXTURES / "native.tsv").read_bytes()
        marker = b"vertex\tlevel-9-0\t"
        offset = raw.index(marker) + len(marker)
        changed = bytearray(raw)
        changed[offset] = ord("1") if changed[offset] != ord("1") else ord("2")
        with self.assertRaises(ValueError):
            fluid_corpus.fixture_check(bytes(changed))


@unittest.skipUnless(os.environ.get("BETA_LIVE_FLUID") == "1",
                     "set BETA_LIVE_FLUID=1 to invoke the pinned original client")
class LiveFluidTests(unittest.TestCase):
    def test_two_native_captures_match_committed_reference(self):
        report = fluid_corpus.verify_live()
        self.assertEqual(report["cases"], 44)
        self.assertEqual(report["vertices"], fluid_corpus.fixture_check()["vertices"])


if __name__ == "__main__":
    unittest.main()
