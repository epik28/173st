"""Offline structure/provenance checks and opt-in original LEAF-001 capture."""

import os
from pathlib import Path
import unittest

from tools.oracle import leaf_corpus

ROOT = Path(__file__).resolve().parents[2]


class LeafFixtureTests(unittest.TestCase):
    def test_fixture_covers_native_leaf_and_grass_contract(self):
        result = leaf_corpus.fixture_check()
        self.assertEqual(result["leaf_rows"], 192)
        self.assertEqual(result["visibility_rows"], 48)
        self.assertEqual(result["grass_vertices"], 96)
        self.assertTrue(result["repeatable"])

    def test_grass_packed_colors_keep_face_and_overlay_layers_distinct(self):
        rows = [line.split("\t") for line in (ROOT / "tests/fixtures/leaf/native.tsv").read_text().splitlines()
                if line.startswith("vertex\t")]
        self.assertEqual({int(row[1]) for row in rows}, {0, 1})
        self.assertEqual({int(row[2]) for row in rows}, set(range(60)))
        colors = {int(row[1]): {} for row in rows}
        for row in rows:
            colors[int(row[1])].setdefault(int(row[8], 16), 0)
            colors[int(row[1])][int(row[8], 16)] += 1
        self.assertEqual(set(colors[0]), {0xFF7F7F7F, 0xFF996633, 0xFFCCCCCC, 0xFF999999})
        self.assertEqual(set(colors[1]), {0xFF7F7F7F, 0xFF996633, 0xFFCCCCCC,
                                          0xFF999999, 0xFF7A5128, 0xFF5B3D1E})


@unittest.skipUnless(os.environ.get("BETA_LIVE_LEAF") == "1",
                     "set BETA_LIVE_LEAF=1 to call the pinned original client")
class LiveLeafCaptureTests(unittest.TestCase):
    def test_two_native_runs_match_committed_fixture_and_render_state(self):
        audit = leaf_corpus.verify_live()
        self.assertTrue(audit["repeatable"])
        self.assertEqual(audit["gl_depth_func"], "GL_LEQUAL (515)")
        self.assertEqual(audit["gl_alpha_func"], "GL_GREATER (516), cutoff 0.1f")


if __name__ == "__main__":
    unittest.main()
