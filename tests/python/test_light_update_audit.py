import json, pathlib, unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIX = ROOT / "tests/fixtures/light_update"

class LightUpdateFixtureTests(unittest.TestCase):
    def test_manifest_and_rows(self):
        rows = [r for r in (FIX / "native.tsv").read_text().splitlines() if "\t" in r]
        self.assertEqual(len(rows), 40)
        self.assertEqual(len({r.split("\t")[0] for r in rows}), len(rows))
        self.assertEqual(rows[0].split("\t")[:4], ["remove", "before", "1", "0"])
        self.assertIn("edge-q1-1", "\n".join(rows))
        audit = json.loads((FIX / "audit.json").read_text())
        self.assertEqual(audit["emission"].split(":", 1)[0], "excluded")
        self.assertEqual(audit["domain"], "chunks -1..1, y 0..127, queue bounded by 27 loaded chunks")

if __name__ == "__main__": unittest.main()
