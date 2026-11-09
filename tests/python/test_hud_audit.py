import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile
from io import BytesIO


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/oracle"))
import hud_audit  # noqa: E402


class HudNativeAuditTests(unittest.TestCase):
    def test_pinned_resources_and_layout_bytecode(self):
        jar = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
        result = hud_audit.verify(jar)
        self.assertEqual(result["layout"]["health_full_20"]["hearts"], 10)

    def test_rejects_wrong_jar(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "wrong.jar"
            path.write_bytes(b"not the pinned Beta client")
            with self.assertRaisesRegex(ValueError, "JAR hash/size"):
                hud_audit.verify(path)

    def test_rejects_missing_native_resource(self):
        jar = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
        with zipfile.ZipFile(jar) as original:
            gui_png = original.read("gui/gui.png")
        archive = BytesIO()
        with zipfile.ZipFile(archive, "w") as output:
            output.writestr("gui/gui.png", gui_png)
        with zipfile.ZipFile(BytesIO(archive.getvalue())) as source:
            with self.assertRaisesRegex(ValueError, "missing gui/icons.png"):
                hud_audit.verify_archive_contents(source)

    def test_rejects_altered_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audit.json"
            mutations = (
                ("layout", lambda item: item["layout"]["hotbar"]["frame_uv_size"].__setitem__(2, 181), "layout evidence"),
                ("class provenance", lambda item: item["reference"].__setitem__("class_sha256", "0" * 64), "class provenance"),
                ("method provenance", lambda item: item["reference"]["render_method"].__setitem__("descriptor", "()V"), "method provenance"),
                ("bytecode", lambda item: item["bytecode_evidence"].__setitem__("crosshair", "changed"), "bytecode evidence"),
            )
            for name, mutate, message in mutations:
                with self.subTest(name=name):
                    fixture = json.loads(hud_audit.FIXTURE.read_text(encoding="utf-8"))
                    mutate(fixture)
                    path.write_text(json.dumps(fixture), encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, message):
                        hud_audit.load_fixture(path)

    def test_rejects_malformed_fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audit.json"
            path.write_text('{"schema_version":', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "valid audit fixture"):
                hud_audit.load_fixture(path)


if __name__ == "__main__":
    unittest.main()
