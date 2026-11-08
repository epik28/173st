"""Synthetic REGION-001 parser, path, comparator and Rust probe checks."""

import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
import zlib

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("region_runner", ROOT / "tools/parity/run.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def record_region(compressed, compression=2, count=1):
    record = struct.pack(">I", len(compressed)+1) + bytes([compression]) + compressed
    contents = bytearray((2+count)*4096)
    contents[:4] = struct.pack(">I", (2 << 8) | count)
    contents[4096:4100] = struct.pack(">I", 0x12345678)
    contents[8192:8192+len(record)] = record
    return contents


class RegionProtocolTests(unittest.TestCase):
    def test_fixture_validates_ids_coordinates_and_exact_region_basename(self):
        rows = b"a\tr.-1.0.mcr\t-1\t0\nb\tr.0.-2.mcr\t31\t-33\n"
        cases = runner.parse_region_fixture(rows)
        self.assertEqual([(c["x"], c["z"]) for c in cases], [(-1, 0), (31, -33)])
        for bad in (b"x\t../r.0.0.mcr\t0\t0\n", b"x\tr.1.0.mcr\t0\t0\n",
                    b"x\tr.0.0.mcr\t2147483648\t0\n",
                    b"bad id\tr.0.0.mcr\t0\t0\n", b"x\tr.0.0.mcr\t0\t0\nx\tr.0.0.mcr\t0\t0\n"):
            with self.subTest(bad=bad), self.assertRaises(runner.RunError):
                runner.parse_region_fixture(bad)

    def test_copy_path_requires_marker_basename_and_no_symlink(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root/".region-input-copy").write_text(runner.REGION_MARKER)
            good = root/"r.0.0.mcr"
            good.write_bytes(bytes(8192))
            self.assertEqual(runner.safe_region_file(root, good.name), good)
            for name in ("../r.0.0.mcr", "/tmp/r.0.0.mcr", "r.00.0.mcr"):
                with self.subTest(name=name), self.assertRaises(runner.RunError):
                    runner.safe_region_file(root, name)
            elsewhere = root.parent/"region-runner-outside.mcr"
            elsewhere.write_bytes(bytes(8192))
            link = root/"r.1.0.mcr"
            link.symlink_to(elsewhere)
            with self.assertRaises(runner.RunError):
                runner.safe_region_file(root, link.name)
            (root/".region-input-copy").unlink()
            with self.assertRaises(runner.RunError):
                runner.safe_region_file(root, good.name)

    def test_output_records_are_ordered_complete_typed_and_mutation_sensitive(self):
        cases = runner.parse_region_fixture(b"a\tr.0.0.mcr\t0\t0\nb\tr.0.0.mcr\t1\t0\n")
        left = (b"a\tpresent\t4294967295\t2\t4\t" + b"a"*64 + b"\n"
                b"b\tabsent\t-\t-\t-\t-\n")
        self.assertEqual(runner.compare_region_outputs(left, left, cases)["mismatch_count"], 0)
        variants = (left.replace(b"\t2\t", b"\t9\t", 1),
                    left.replace(b"4294967295", b"4294967296", 1),
                    left.replace(b"a"*64, b"b"*64, 1), left+b"extra\n",
                    left.replace(b"\n", b"\r\n"))
        for variant in variants:
            with self.subTest(variant=variant):
                self.assertGreater(runner.compare_region_outputs(left, variant, cases)["mismatch_count"], 0)

    @classmethod
    def setUpClass(cls):
        cls.binary = ROOT/"target/debug/examples/region_probe"
        subprocess.run(["cargo", "build", "--locked", "--offline", "-p", "beta-core", "--example", "region_probe"],
                       cwd=ROOT, check=True, capture_output=True, timeout=120)

    def probe(self, contents, rows, *, extra_slot=False):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            input_dir, payload_dir = root/"input", root/"payloads"
            input_dir.mkdir(); payload_dir.mkdir()
            (input_dir/".region-input-copy").write_text(runner.REGION_MARKER)
            (input_dir/"r.0.0.mcr").write_bytes(contents)
            result = subprocess.run([str(self.binary), str(input_dir), str(payload_dir)],
                                    input=rows, capture_output=True, timeout=20)
            return result, payload_dir, (input_dir/"r.0.0.mcr").read_bytes()

    def test_invalid_header_overlap_unknown_compression_and_truncated_stream_reject(self):
        valid = record_region(zlib.compress(b"payload"))
        cases = []
        cases.append((bytes(4096), b"x\tr.0.0.mcr\t0\t0\n"))
        overlap = bytearray(valid)
        overlap[4:8] = overlap[0:4]
        cases.append((bytes(overlap), b"x\tr.0.0.mcr\t0\t0\n"))
        cases.append((bytes(record_region(zlib.compress(b"payload"), compression=255)), b"x\tr.0.0.mcr\t0\t0\n"))
        cases.append((bytes(record_region(zlib.compress(b"payload")[:-2])), b"x\tr.0.0.mcr\t0\t0\n"))
        for contents, rows in cases:
            with self.subTest(length=len(contents)):
                result, _, after = self.probe(contents, rows)
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(result.stderr)
                self.assertEqual(after, contents)

    def test_over_limit_decompression_fails_without_modifying_copy(self):
        compressed = zlib.compress(b"\0"*(64*1024*1024+1))
        contents = bytes(record_region(compressed, count=17))
        result, _, after = self.probe(contents, b"x\tr.0.0.mcr\t0\t0\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(result.stderr)
        self.assertEqual(after, contents)

    def test_region_probe_rejects_symlink_even_when_target_is_inside_artifacts(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            input_dir, payload_dir = root/"input", root/"payloads"
            input_dir.mkdir(); payload_dir.mkdir()
            (input_dir/".region-input-copy").write_text(runner.REGION_MARKER)
            outside_dir = root/"outside"
            outside_dir.mkdir()
            outside = outside_dir/"r.0.0.mcr"
            original = bytes(record_region(zlib.compress(b"payload")))
            outside.write_bytes(original)
            (input_dir/"r.0.0.mcr").symlink_to(outside)
            result = subprocess.run([str(self.binary), str(input_dir), str(payload_dir)],
                                    input=b"x\tr.0.0.mcr\t0\t0\n", capture_output=True, timeout=20)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"regular copied file", result.stderr)
            self.assertEqual(outside.read_bytes(), original)

    def test_saved_report_rejects_missing_process_even_if_status_claims_pass(self):
        # Report verifier enforces completeness before considering saved output hashes.
        sys.path.insert(0, str(ROOT/"tools/parity"))
        import region_review
        report = {"schema_version": 1, "suite": "region", "status": "pass", "processes": {}}
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)/"report.json"
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(SystemExit, "required process"):
                region_review.verify_region(path, report)

    def test_saved_report_evidence_helpers_detect_audit_payload_and_copy_tampering(self):
        sys.path.insert(0, str(ROOT/"tools/parity"))
        import region_review
        report = {"reference_class_sha256": runner.REGION_CLASS_SHA256}
        audit = {"class": "qj", "method": "a(int,int)", "class_sha256": runner.REGION_CLASS_SHA256,
                 "jar_sha256": runner.REGION_JAR_SHA256, "native_reads": 748, "present": 704, "absent": 44}
        region_review._check_audit(audit, report, 748)
        for key, value in (("native_reads", 747), ("class_sha256", "0"*64), ("jar_sha256", "0"*64)):
            mutated = dict(audit, **{key: value})
            with self.subTest(key=key), self.assertRaises(SystemExit):
                region_review._check_audit(mutated, report, 748)

        payload = b"payload"
        row = b"x\tpresent\t1\t2\t7\t" + runner.sha256(payload).encode() + b"\n"
        region_review._check_payload_tsv(payload, row)
        with self.assertRaises(SystemExit):
            region_review._check_payload_tsv(payload+b"!", row)

        with tempfile.TemporaryDirectory() as temp:
            copied = Path(temp)/"input"
            copied.mkdir()
            (copied/".region-input-copy").write_text(runner.REGION_MARKER)
            (copied/"r.0.0.mcr").write_bytes(b"region")
            hashes = {"r.0.0.mcr": runner.sha256(b"region")}
            region_review._check_copy(copied, hashes)
            (copied/"r.0.0.mcr").write_bytes(b"changed")
            with self.assertRaises(SystemExit):
                region_review._check_copy(copied, hashes)

    def test_region_process_timeout_is_reported_and_partial_logs_are_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            result = runner.run_process([sys.executable, "-c", "import time; print('start', flush=True); time.sleep(2)"],
                                        cwd=ROOT, directory=directory, label="region-timeout", timeout=1.0)
            self.assertTrue(result["timeout"])
            self.assertIsNone(result["returncode"])
            self.assertEqual(Path(result["stdout"]).read_bytes(), b"start\n")
            self.assertEqual(result["stdout_sha256"], runner.sha256(b"start\n"))


if __name__ == "__main__":
    unittest.main()
