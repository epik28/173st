import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools.oracle import menu_parity_audit as audit


class MenuParityAuditTest(unittest.TestCase):
    def test_pinned_native_contract_and_repeatable_oracle(self):
        fixture = audit.verify(audit.default_jar())
        self.assertEqual(fixture["native_input"], audit.EXPECTED_INPUT)
        self.assertEqual(len(audit.EXPECTED_NATIVE.read_text(encoding="utf-8").splitlines()), 265)

    def test_layout_mutation_is_rejected(self):
        fixture = json.loads(audit.FIXTURE.read_text(encoding="utf-8"))
        fixture["layout"]["world_select"]["list"]["row_height"] = 42
        with tempfile.TemporaryDirectory() as directory:
            candidate = Path(directory) / "audit.json"
            candidate.write_text(json.dumps(fixture), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "frozen menu/font/button/scaling API"):
                audit.load_fixture(candidate)

    def test_translation_mutation_is_rejected(self):
        fixture = json.loads(audit.FIXTURE.read_text(encoding="utf-8"))
        fixture["labels"]["menu.singleplayer"] = "Play"
        with tempfile.TemporaryDirectory() as directory:
            candidate = Path(directory) / "audit.json"
            candidate.write_text(json.dumps(fixture), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "en_US menu labels"):
                audit.load_fixture(candidate)

    def test_native_output_mutation_is_rejected_even_with_updated_fixture_hash(self):
        fixture = json.loads(audit.FIXTURE.read_text(encoding="utf-8"))
        native = audit.EXPECTED_NATIVE.read_bytes().replace(b"glyph\t0\t1", b"glyph\t0\t2", 1)
        fixture["native_output_sha256"] = audit.sha256(native)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            candidate_fixture = root / "audit.json"
            candidate_native = root / "native.tsv"
            candidate_fixture.write_text(json.dumps(fixture), encoding="utf-8")
            candidate_native.write_bytes(native)
            with patch.object(audit, "EXPECTED_NATIVE", candidate_native):
                with self.assertRaisesRegex(ValueError, "native oracle output hash"):
                    audit.load_fixture(candidate_fixture)


if __name__ == "__main__":
    unittest.main()
