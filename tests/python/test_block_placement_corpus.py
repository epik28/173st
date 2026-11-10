"""Native BLOCK-PLACEMENT-002 action corpus integrity and regression checks."""
import unittest

from tools.oracle import placement_corpus as corpus
from tools.oracle import placement_grass_rays as grass_rays


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

    def test_native_tallgrass_off_center_and_boundary_rays_repeat(self):
        report = grass_rays.verify()
        self.assertEqual(report["cases"], 14)
        self.assertTrue(report["repeat_identical"])
        rows = {row["id"]: row for row in grass_rays.parse_output(grass_rays.FIXTURE.read_bytes())}
        self.assertEqual((rows["x_mid_inside"]["kind"], rows["x_mid_inside"]["side"]), ("block", "4"))
        self.assertEqual(rows["x_above_height"]["kind"], "miss")
        self.assertEqual(rows["x_min_exact_along_z"]["kind"], "block")
        self.assertEqual(rows["x_min_below_along_z"]["kind"], "miss")
        self.assertEqual(rows["x_max_exact_along_z"]["kind"], "block")
        self.assertEqual(rows["x_max_above_along_z"]["kind"], "miss")
        self.assertEqual(rows["vertical_x_min_exact"]["kind"], "block")
        self.assertEqual(rows["vertical_x_below"]["kind"], "miss")
        self.assertEqual(rows["diagonal_inside"]["kind"], "block")
        self.assertEqual(rows["diagonal_above"]["kind"], "miss")

    def test_grass_ray_parser_rejects_order_and_raw_word_mutations(self):
        raw = grass_rays.FIXTURE.read_bytes()
        with self.assertRaises(ValueError):
            grass_rays.parse_output(raw.replace(b"x_mid_inside", b"x_min_below_along_z", 1))
        with self.assertRaises(ValueError):
            grass_rays.parse_output(raw.replace(b"3fb9999980000000", b"3fb9999980000001", 1))
        inputs = grass_rays.INPUT.read_bytes()
        with self.assertRaises(ValueError):
            grass_rays.parse_input(inputs.replace(b"diagonal_above", b"diagonal_inside", 1))


if __name__ == "__main__":
    unittest.main()
