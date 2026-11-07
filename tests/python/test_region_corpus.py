"""Synthetic filesystem protection tests; not original world parity evidence."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("region_corpus", ROOT / "tools/oracle/region_corpus.py")
corpus = importlib.util.module_from_spec(spec)
spec.loader.exec_module(corpus)


class SnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "config").mkdir()
        (self.root / "tests/fixtures/region").mkdir(parents=True)
        (self.root / "config/local.toml").write_text('[paths]\nprism_instance="source"\nartifacts="artifacts"\n')
        self.source = self.root / "source/region"
        self.source.mkdir(parents=True)
        self.content = bytes(8192)
        self.file = self.source / "r.0.0.mcr"
        self.file.write_bytes(self.content)
        (self.root / "tests/fixtures/region/sources.json").write_text(json.dumps({
            "world_relative_path": "region", "files": {self.file.name: {
                "size": len(self.content), "sha256": corpus.digest(self.content)}}}))
        self.run = self.root / "artifacts/run"
        self.run.mkdir(parents=True)

    def test_copy_and_source_remain_identical(self):
        target, evidence = corpus.snapshot(self.run, self.root)
        self.assertEqual((target / self.file.name).read_bytes(), self.content)
        corpus.verify_snapshot(self.run, evidence, self.root)
        self.assertEqual(evidence["after_run"], evidence["before"])
        with self.assertRaises(FileExistsError):
            corpus.snapshot(self.run, self.root)

    def test_wrong_source_hash_stops_snapshot(self):
        self.file.write_bytes(b"x" + self.content[1:])
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            corpus.snapshot(self.run, self.root)
        self.assertFalse((self.run / "input").exists())

    def test_source_and_copy_changes_are_detected(self):
        target, evidence = corpus.snapshot(self.run, self.root)
        (target / self.file.name).write_bytes(b"x" + self.content[1:])
        with self.assertRaisesRegex(ValueError, "changed"):
            corpus.verify_snapshot(self.run, evidence, self.root)
        (target / self.file.name).write_bytes(self.content)
        self.file.write_bytes(b"x" + self.content[1:])
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            corpus.verify_snapshot(self.run, evidence, self.root)

    def test_escape_and_symlink_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "artifacts"):
            corpus.snapshot(self.source, self.root)
        moved = self.source / "other"
        self.file.rename(moved)
        self.file.symlink_to(moved)
        with self.assertRaisesRegex(ValueError, "escape"):
            corpus.snapshot(self.run, self.root)


if __name__ == "__main__":
    unittest.main()
