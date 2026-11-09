import json
from pathlib import Path
import tempfile
import unittest
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/oracle"))
import inventory_interaction_audit  # noqa: E402


class InventoryInteractionNativeAuditTests(unittest.TestCase):
    def test_native_replay_and_protocol(self):
        jar = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
        fixture = inventory_interaction_audit.verify(jar)
        self.assertEqual(fixture["protocol"]["expected_rows"], 23)

    def test_rejects_wrong_jar(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "wrong.jar"
            path.write_bytes(b"wrong")
            with self.assertRaisesRegex(ValueError, "JAR hash/size"):
                inventory_interaction_audit.verify(path)

    def test_rejects_mutated_protocol(self):
        value = json.loads(inventory_interaction_audit.FIXTURE.read_text(encoding="utf-8"))
        value["protocol"]["expected_rows"] = 14
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audit.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "protocol bounds"):
                inventory_interaction_audit.load_fixture(path)


if __name__ == "__main__":
    unittest.main()
