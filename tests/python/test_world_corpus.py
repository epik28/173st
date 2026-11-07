"""WORLD-LOAD-001 C01 corpus and snapshot guard tests; no world files needed."""
import importlib.util
import json
import hashlib
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("world_corpus", ROOT / "tools/oracle/world_corpus.py")
world = importlib.util.module_from_spec(spec)
spec.loader.exec_module(world)


class WorldCorpusTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / "config").mkdir()
        (self.root / "config/local.toml").write_text('[paths]\nprism_instance="source"\nartifacts="artifacts"\n')
        self.worlddir = self.root / "source/minecraft/saves/New World"
        (self.worlddir / "region").mkdir(parents=True)
        names = ["level.dat", "r.-1.-1.mcr", "r.-1.0.mcr", "r.0.-1.mcr", "r.0.0.mcr"]
        self.files = {}
        for i, name in enumerate(names):
            data = f"fixture-{i}".encode()
            target = self.worlddir / name if name == "level.dat" else self.worlddir / "region" / name
            target.write_bytes(data)
            self.files[name] = {"size": len(data), "sha256": hashlib.sha256(data).hexdigest()}
        fixture = self.root / "tests/fixtures/world"
        fixture.mkdir(parents=True)
        (fixture / "sources.json").write_text(json.dumps({"world_relative_path": "minecraft/saves/New World", "files": self.files}))
        self.run = self.root / "artifacts/run-test"
        self.run.mkdir(parents=True)

    def test_copy_markers_hashes_and_final_verification(self):
        target, evidence = world.snapshot(self.run, self.root)
        self.assertEqual(set(p.name for p in target.iterdir()), set(self.files) | {".world-input-copy", ".region-input-copy"})
        self.assertEqual((target / ".world-input-copy").read_text(), world.MARK_WORLD)
        self.assertEqual((target / ".region-input-copy").read_text(), world.MARK_REGION)
        world.verify_snapshot(self.run, evidence, self.root)
        self.assertEqual(evidence["source_before"], evidence["source_after"])
        with self.assertRaises(FileExistsError):
            world.snapshot(self.run, self.root)

    def test_source_copy_and_symlink_mutations_are_rejected(self):
        target, evidence = world.snapshot(self.run, self.root)
        (target / "level.dat").write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "changed"):
            world.verify_snapshot(self.run, evidence, self.root)
        (target / "level.dat").write_bytes((self.worlddir / "level.dat").read_bytes())
        source = self.worlddir / "level.dat"
        source.write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "size/hash"):
            world.verify_snapshot(self.run, evidence, self.root)
        source.write_bytes(b"fixture-0")
        link = self.worlddir / "region/r.0.0.mcr"
        real = link.with_name("real.mcr")
        link.rename(real)
        link.symlink_to(real)
        with self.assertRaisesRegex(ValueError, "unsafe source"):
            world.source_paths(self.root)

    def test_artifact_escape_and_manifest_traversal_rejected(self):
        outside = self.root / "outside"
        outside.mkdir()
        with self.assertRaisesRegex(ValueError, "artifacts"):
            world.snapshot(outside, self.root)
        doc = json.loads((self.root / "tests/fixtures/world/sources.json").read_text())
        doc["files"]["../level.dat"] = doc["files"].pop("level.dat")
        (self.root / "tests/fixtures/world/sources.json").write_text(json.dumps(doc))
        with self.assertRaisesRegex(ValueError, "basename"):
            world.source_paths(self.root)

    def test_checked_in_live_corpus_and_native_evidence_are_separate(self):
        rows = (ROOT / "tests/fixtures/world/live.tsv").read_text().splitlines()
        self.assertEqual(len(rows), 750)  # header plus 749 cases
        self.assertEqual(rows[1], "live-level\tlevel\tlevel.dat\t-\t-")
        self.assertEqual(sum(line.split("\t")[1] == "chunk" for line in rows[2:]), 748)
        for line in rows[1:]:
            self.assertEqual(len(line.split("\t")), 5)
        audit = json.loads((ROOT / "tests/fixtures/world/audit.json").read_text())
        native = audit["native"]
        self.assertEqual(native["live_rows"], 749)
        self.assertEqual(native["native_calls"], {
            "ij_b_DataInput": 705, "ei_constructor": 1, "qj_a_int_int": 748, "native_chunk_loads": 0})
        self.assertEqual(native["chunk_present"], 704)
        self.assertEqual(native["chunk_absent"], 44)
        self.assertEqual(set(audit["source_before"]), {
            "level.dat", "r.-1.-1.mcr", "r.-1.0.mcr", "r.0.-1.mcr", "r.0.0.mcr"})
        generated = audit["generated"]["generated_semantics"]
        self.assertEqual(generated["status"], "PASS")
        self.assertIn("wrong-long-int-byte-compound-types-cast-fail", generated["cases"])
        self.assertEqual(audit["generated"]["native_calls"]["qj_a_int_int"], 0)
        self.assertEqual(audit["generated"]["native_calls"]["native_chunk_loads"], 0)


if __name__ == "__main__":
    unittest.main()
