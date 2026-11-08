import unittest

from tools.oracle import view_corpus


class ViewCorpusTests(unittest.TestCase):
    def test_fixed_scene_is_25_slots_around_the_pinned_spawn_chunk(self):
        rows = view_corpus.fixture_rows().splitlines()
        self.assertEqual(len(rows) - 1, 25)
        self.assertEqual(rows[1], "-3\t-3")
        self.assertEqual(rows[-1], "1\t1")

    def test_native_mapping_protocol_accepts_exact_rows(self):
        raw = (
            "#VIEW_NATIVE\t1\t-9\t64\t-5\t-3\t-3\n"
            "#states\n"
            "state\t1\t0\t-1\t0\t1\t1\t111111\t1,1,1,1,1,1\t1\n"
            "#neighbors\n"
            "neighbor\t1\t0\t0\t0\t0\t1\n"
        ).encode()
        text, header, sections = view_corpus.parse_stdout(raw)
        self.assertEqual(header, "#VIEW_NATIVE\t1\t-9\t64\t-5\t-3\t-3")
        self.assertEqual(len(sections["states"]), 1)
        self.assertEqual(len(sections["neighbors"]), 1)
        self.assertTrue(text.endswith("\n"))

    def test_committed_native_fixture_uses_world_aware_virtual_face_mapping(self):
        raw = (view_corpus.ROOT / "tests/fixtures/view/live.tsv").read_bytes()
        _, _, sections = view_corpus.parse_stdout(raw)
        states = {(int(row[1]), int(row[2]), int(row[3])): row for row in sections["states"]}
        self.assertEqual(states[(2, 0, 0)][8], "2,0,3,3,3,3")
        self.assertEqual(states[(17, 0, -1)][8], "21,21,20,20,20,20")
        self.assertEqual(states[(17, 2, -1)][8], "21,21,117,117,117,117")

    def test_synthetic_inputs_are_tsv_and_cover_edge_materials(self):
        lines = (view_corpus.ROOT / "tests/fixtures/view/synthetic.tsv").read_text().splitlines()
        rows = [line.split("\t") for line in lines if line and not line.startswith("#")]
        self.assertGreaterEqual(len(rows), 30)
        self.assertTrue(all(len(row) == 6 for row in rows if row[0] == "neighbor"))
        self.assertTrue(all(len(row) == 4 for row in rows if row[0] == "state"))
        self.assertTrue(any(row[1:3] == ["255", "15"] for row in rows))

    def test_native_protocol_rejects_malformed_or_duplicate_state_rows(self):
        header = "#VIEW_NATIVE\t1\t-9\t64\t-5\t-3\t-3\n#states\n"
        row = "state\t1\t0\t-1\t0\t1\t1\t111111\t1,1,1,1,1,1\t1\n"
        with self.assertRaisesRegex(ValueError, "malformed native output row"):
            view_corpus.parse_stdout((header + "state\t1\n#neighbors\n").encode())
        with self.assertRaisesRegex(ValueError, "duplicate native block/context state"):
            view_corpus.parse_stdout((header + row + row + "#neighbors\n").encode())

    def test_native_protocol_rejects_changed_spawn_or_invalid_alpha_mapping(self):
        changed_spawn = "#VIEW_NATIVE\t1\t-8\t64\t-5\t-3\t-3\n#states\n"
        with self.assertRaisesRegex(ValueError, "pinned test world spawn"):
            view_corpus.parse_stdout((changed_spawn + "state\t1\t0\t-1\t0\t1\t1\t111111\t1,1,1,1,1,1\t1\n#neighbors\n").encode())
        invalid_alpha = "#VIEW_NATIVE\t1\t-9\t64\t-5\t-3\t-3\n#states\nstate\t1\t0\t-1\t0\t1\t1\t111110\t1,1,1,1,1,1\t1\n#neighbors\n"
        with self.assertRaisesRegex(ValueError, "inconsistent"):
            view_corpus.parse_stdout(invalid_alpha.encode())


if __name__ == "__main__":
    unittest.main()
