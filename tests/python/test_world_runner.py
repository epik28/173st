"""WORLD-LOAD-001 runner validation; synthetic protocol data only."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
import world_review
import world_run


HASH = b"a" * 64


def level_row(case_id=b"level"):
    return b"\t".join([case_id, b"level", b"0", b"1", b"-2", b"3", b"4", b"5", b"6",
                       b"0", b"-", b"19132", b"7", b"1", b"8", b"0", b"0", b"0", b"-", b"0", HASH]) + b"\n"


def chunk_row(case_id, state, x, z):
    if state == b"absent":
        rest = [b"-"] * 12
    else:
        rest = [b"0", b"1", HASH, HASH, HASH, HASH, HASH, b"0", b"0", b"-", b"10", HASH]
    return b"\t".join([case_id, b"chunk", state, str(x).encode(), str(z).encode(), *rest]) + b"\n"


class WorldRunnerTests(unittest.TestCase):
    def test_fixture_accepts_comments_blank_lines_negative_zero_and_floor_coordinates(self):
        data = (b"# comment\n\nlevel\tlevel\tlevel.dat\t-\t-\n"
                b"zero\tchunk\tr.0.0.mcr\t-0\t0\n"
                b"negative\tchunk\tr.-1.0.mcr\t-1\t0\n")
        cases = world_run.parse_world_fixture(data)
        self.assertEqual([(case["x"], case["z"]) for case in cases[1:]], [(0, 0), (-1, 0)])

    def test_fixture_rejects_whitespace_duplicate_ids_bad_paths_and_coordinate_overflow(self):
        invalid = [b" \n", b"same\tlevel\tlevel.dat\t-\t-\nsame\tlevel\tlevel.dat\t-\t-\n",
                   b"x\tchunk\t../r.0.0.mcr\t0\t0\n", b"x\tchunk\tr.0.0.mcr\t2147483648\t0\n",
                   b"x\tlevel\tlevel.dat\t0\t0\n", b"x\tchunk\tr.0.0.mcr\t+0\t0\n"]
        for data in invalid:
            with self.subTest(data=data), self.assertRaises(world_run.WorldRunError):
                world_run.parse_world_fixture(data)

    def test_fixture_enforces_case_limit(self):
        data = b"".join(f"case{i}\tlevel\tlevel.dat\t-\t-\n".encode() for i in range(100001))
        with self.assertRaisesRegex(world_run.WorldRunError, "limit 100000"):
            world_run.parse_world_fixture(data)

    def test_typed_records_and_comparator_detect_order_missing_extra_and_bad_types(self):
        cases = world_run.parse_world_fixture(
            b"level\tlevel\tlevel.dat\t-\t-\nzero\tchunk\tr.0.0.mcr\t-0\t0\n")
        rows = level_row() + chunk_row(b"zero", b"absent", 0, 0)
        self.assertTrue(world_run.valid_world_record(level_row(), world_run.world_keys(cases)[0]))
        self.assertTrue(world_run.valid_world_record(chunk_row(b"zero", b"absent", 0, 0),
                                                    world_run.world_keys(cases)[1]))
        self.assertEqual(world_run.compare_world_outputs(rows, rows, cases)["mismatch_count"], 0)
        for damaged in (rows[:-1], rows + chunk_row(b"extra", b"absent", 0, 0),
                        chunk_row(b"zero", b"absent", 0, 0) + level_row(),
                        level_row().replace(b"\t0\t-\t19132", b"\t00\t-\t19132") +
                        chunk_row(b"zero", b"absent", 0, 0)):
            with self.subTest(damaged=damaged[-60:]):
                self.assertGreater(world_run.compare_world_outputs(rows, damaged, cases)["mismatch_count"], 0)

    def test_present_absent_records_require_exact_shape_and_lowercase_hashes(self):
        case = world_run.parse_world_fixture(b"c\tchunk\tr.0.0.mcr\t0\t0\n")[0]
        key = world_run.world_keys([case])[0]
        present = chunk_row(b"c", b"present", 0, 0)
        self.assertTrue(world_run.valid_world_record(present, key))
        self.assertFalse(world_run.valid_world_record(present.replace(HASH, b"A" * 64), key))
        self.assertFalse(world_run.valid_world_record(present[:-1] + b"\r\n", key))
        self.assertFalse(world_run.valid_world_record(chunk_row(b"c", b"absent", 1, 0), key))

    def test_saved_log_verifier_rejects_missing_required_process(self):
        with self.assertRaisesRegex(SystemExit, "required process missing"):
            world_review._read_log({}, "java-run")

    def test_saved_report_verifier_reports_missing_original_jar_cleanly(self):
        required = ("version-java", "version-javac", "version-rustc", "version-cargo", "java-build",
                    "java-run", "rust-build", "rust-run", "git-revision", "git-dirty")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            log = root / "log"
            log.write_bytes(b"ok\n")
            record = {"returncode": 0, "timeout": False, "error": None,
                      "stdout": str(log), "stderr": str(log),
                      "stdout_sha256": world_run.sha256(b"ok\n"),
                      "stderr_sha256": world_run.sha256(b"ok\n")}
            report = {"schema_version": 1, "suite": "world", "status": "pass",
                      "processes": {name: record for name in required}}
            report_path = root / "report.json"
            report_path.write_text("{}\n")
            with self.assertRaisesRegex(SystemExit, "original JAR is missing"):
                world_review.verify_world(report_path, report)

    def test_negative_zero_protocol_requires_canonical_typed_row(self):
        self.assertTrue(world_review._negative_zero_protocol_accepts(
            chunk_row(b"zero", b"absent", 0, 0)))
        malformed = chunk_row(b"zero", b"absent", 0, 0).replace(b"\t0\t0\t", b"\t-0\t0\t", 1)
        self.assertFalse(world_review._negative_zero_protocol_accepts(malformed))
        self.assertFalse(world_review._negative_zero_protocol_accepts(b"zero\tchunk\tabsent\t0\t0\nextra\n"))

    def test_native_audit_mutation_is_rejected_by_pinned_audit_validator(self):
        audit = json.loads((ROOT / "tests/fixtures/world/audit.json").read_text())["native"]
        world_run._validate_audit(audit, audit["input_sha256"], audit["live_rows"])
        audit["native_calls"]["qj_a_int_int"] -= 1
        with self.assertRaisesRegex(world_run.WorldRunError, "native call/class/source audit"):
            world_run._validate_audit(audit, audit["input_sha256"], audit["live_rows"])

    def test_copy_guard_requires_exact_files_and_rejects_symlinks(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            files = {"level.dat": b"level", "r.0.0.mcr": b"region"}
            for name, contents in files.items():
                (directory / name).write_bytes(contents)
            (directory / ".world-input-copy").write_text(world_run.MARK_WORLD)
            (directory / ".region-input-copy").write_text(world_run.MARK_REGION)
            hashes = {name: world_run.sha256(contents) for name, contents in files.items()}
            self.assertEqual(world_run._check_copy(directory, hashes), hashes)
            (directory / "extra").write_text("unexpected")
            with self.assertRaisesRegex(world_run.WorldRunError, "unexpected entries"):
                world_run._check_copy(directory, hashes)
            (directory / "extra").unlink()
            (directory / "link").symlink_to(directory / "level.dat")
            with self.assertRaisesRegex(world_run.WorldRunError, "unexpected entries"):
                world_run._check_copy(directory, hashes)

    def test_retained_manifest_rejects_unexpected_entries_and_symlinks(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "payload.nbt").write_bytes(b"raw")
            self.assertEqual(world_run._retained_manifest(directory, {}, prefix="Rust"), {
                "payload.nbt": {"sha256": world_run.sha256(b"raw"), "length": 3}})
            (directory / "unsafe").mkdir()
            with self.assertRaisesRegex(world_run.WorldRunError, "unsafe"):
                world_run._retained_manifest(directory, {}, prefix="Rust")


if __name__ == "__main__":
    unittest.main()
