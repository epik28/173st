import json
from pathlib import Path
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/oracle"))
import hotbar_audit  # noqa: E402


class HotbarNativeAuditTests(unittest.TestCase):
    def test_pinned_input_and_default_binding_bytecode(self):
        jar = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
        fixture = hotbar_audit.verify(jar)
        self.assertEqual(fixture["behavior"]["wheel"]["examples"][0]["result"], 8)
        self.assertEqual(fixture["behavior"]["number_keys"]["slots"], list(range(9)))

    def test_rejects_wrong_jar(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "wrong.jar"
            path.write_bytes(b"not the pinned Beta client")
            with self.assertRaisesRegex(ValueError, "JAR hash/size"):
                hotbar_audit.verify(path)

    def test_rejects_altered_behavior_or_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audit.json"
            mutations = (
                ("wheel direction", lambda item: item["behavior"]["wheel"].__setitem__("direction", "reversed"), "behavior evidence"),
                ("wheel steps", lambda item: item["behavior"]["wheel"].__setitem__("step_count", "one step per 120 units"), "behavior evidence"),
                ("key map", lambda item: item["behavior"]["number_keys"]["physical_codes"].__setitem__(0, 3), "behavior evidence"),
                ("class hash", lambda item: item["reference"]["classes"].__setitem__("ix", "0" * 64), "class hashes"),
                ("method", lambda item: item["reference"]["native_methods"]["inventory_scroll"].__setitem__("descriptor", "()V"), "method provenance"),
                ("bytecode", lambda item: item["bytecode_evidence"].__setitem__("number_key_mapping_and_assignment", "changed"), "bytecode evidence"),
                ("binding bytecode", lambda item: item["default_binding_bytecode"].__setitem__("key.jump", "changed"), "default binding bytecode"),
            )
            for name, mutate, message in mutations:
                with self.subTest(name=name):
                    fixture = json.loads(hotbar_audit.FIXTURE.read_text(encoding="utf-8"))
                    mutate(fixture)
                    path.write_text(json.dumps(fixture), encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, message):
                        hotbar_audit.load_fixture(path)

    def test_rejects_malformed_fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audit.json"
            path.write_text('{"schema_version":', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "valid hotbar audit fixture"):
                hotbar_audit.load_fixture(path)


if __name__ == "__main__":
    unittest.main()
