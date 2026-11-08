import json
import unittest

from tools.oracle import block_corpus


class BlockCorpusTests(unittest.TestCase):
    def test_committed_native_fixture_has_required_live_ray_cases_and_raw_words(self):
        raw = block_corpus.FIXTURE.read_bytes()
        rows = block_corpus.parse(raw)
        by_id = {row["id"]: row for row in rows}
        self.assertEqual(len(rows), 17)
        for name, side in (("face_west", "4"), ("face_east", "5"), ("face_down", "0"),
                           ("face_up", "1"), ("face_north", "2"), ("face_south", "3")):
            self.assertEqual(by_id[name]["side"], side)
            self.assertEqual(len(by_id[name]["px_bits"]), 16)
        self.assertEqual(by_id["negative_voxel"]["x"], "-1")
        self.assertEqual(by_id["tie_corner"]["side"], "4")
        self.assertEqual(by_id["shape_slab"]["py_bits"], "3fd0000000000000")
        self.assertEqual(by_id["liquid_stop_false"]["kind"], "miss")
        self.assertEqual(by_id["liquid_stop_true"]["kind"], "block")
        self.assertEqual(by_id["range_before"]["kind"], "miss")
        self.assertEqual(by_id["range_at"]["kind"], "miss")
        self.assertEqual(by_id["range_after"]["px_bits"], "4014000000000000")

    def test_pinned_native_audit_maps_classes_and_separates_action_limit(self):
        audit = json.loads(block_corpus.AUDIT.read_text())
        self.assertEqual(audit["classes"], block_corpus.CLASS_SHA)
        self.assertEqual(audit["native_cases"], 17)
        self.assertEqual(audit["input_sha256"], block_corpus.digest(block_corpus.INPUT_FIXTURE.read_bytes()))
        self.assertEqual(audit["reach"], "ob.b():5.0f; pj extends ob")
        self.assertEqual(audit["actions"]["break"], "not_verified")
        self.assertEqual(audit["actions"]["place"], "not_verified")

    def test_parser_rejects_changed_source_shape_and_raw_word(self):
        fixture = block_corpus.FIXTURE.read_bytes()
        cases = (
            fixture.replace(b"face_west\tlive", b"face_west\tsynthetic", 1),
            fixture.replace(b"face_west\tlive\tblock", b"face_west\tlive\tblock\textra", 1),
            fixture.replace(b"0000000000000000", b"bad", 1),
            fixture.replace(b"#BLOCK_NATIVE\t1", b"#BLOCK_NATIVE\t2", 1),
        )
        for damaged in cases:
            with self.subTest(prefix=damaged[:60]), self.assertRaises(ValueError):
                block_corpus.parse(damaged)


if __name__ == "__main__":
    unittest.main()
