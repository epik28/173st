"""Native BLOCK-PLACEMENT-002 action corpus integrity and regression checks."""
import unittest

from tools.oracle import placement_corpus as corpus


class BlockPlacementCorpusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = corpus.parse(corpus.OUTPUT.read_bytes())
        cls.by_id = {row["id"]: row for row in cls.rows}

    def test_retained_fixture_matches_repeated_pinned_native_actions(self):
        report = corpus.verify()
        self.assertEqual(report["cases"], 16)
        self.assertTrue(report["repeat_identical"])

    def test_raycast_and_all_six_cube_faces_match_native(self):
        for side in range(6):
            row = self.by_id[f"target1_side{side}"]
            self.assertEqual((row["raycast_kind"], row["raycast_side"]), ("block", str(side)))
            self.assertEqual((row["actions"], row["adjacent_after"], row["stack_after"]),
                             ("true", "1:0", "3"))

    def test_native_id31_ray_hits_but_action_has_side_specific_result(self):
        for side in range(6):
            row = self.by_id[f"target31_side{side}"]
            self.assertEqual((row["raycast_kind"], row["raycast_side"]), ("block", str(side)))
            self.assertEqual(row["actions"], "true")
            self.assertEqual(row["stack_after"], "3")
            self.assertEqual(row["neighborhood_after"].split(",")[13], "0:0:0=0:0")
        self.assertEqual(self.by_id["target31_side2"]["adjacent_after"], "13:0")
        self.assertEqual(self.by_id["target31_side4"]["adjacent_after"], "13:0")

    def test_repeat_occupied_build_limit_and_chunk_edge(self):
        self.assertEqual(self.by_id["repeat_top"]["actions"], "true,false")
        self.assertEqual(self.by_id["repeat_top"]["stack_after"], "3")
        self.assertEqual(self.by_id["occupied_top"]["actions"], "false")
        self.assertEqual(self.by_id["occupied_top"]["stack_after"], "4")
        self.assertEqual(self.by_id["chunk_edge_east"]["adjacent_after"], "1:0")
        self.assertEqual(self.by_id["place_at_127"]["actions"], "false")

    def test_schema_and_payload_mutations_are_rejected(self):
        raw = corpus.OUTPUT.read_bytes()
        with self.assertRaises(ValueError):
            corpus.parse(raw.replace(b"target31_side2", b"target31_side4", 1))
        with self.assertRaises(ValueError):
            corpus.parse(raw.replace(b"raycast_side", b"wrong_side", 1))
        with self.assertRaises(ValueError):
            corpus.parse(raw.replace(b"true\t0:0\t1:0", b"yes\t0:0\t1:0", 1))


if __name__ == "__main__":
    unittest.main()
