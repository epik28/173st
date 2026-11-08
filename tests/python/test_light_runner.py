"""Offline protocol/evidence checks plus opt-in original-JAR live acceptance."""
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
import light_run as light
from light_review import retained, validate_revision_sources, verify_light


def sample(case_id="s", dimension=0, block=8, sky=5, subtraction=3, minimum=7):
    return f"{case_id}\tsample\t{dimension}\t{block}\t{sky}\t{subtraction}\t{minimum}\n".encode()


class LightRunnerTests(unittest.TestCase):
    def test_native_corpus_audit_and_pinned_fixture_are_bound(self):
        raw = (ROOT / light.FIXTURES / "input.tsv").read_bytes()
        expected = (ROOT / light.FIXTURES / "reference.tsv").read_bytes()
        cases = light.parse_input(raw)
        self.assertEqual(len(cases), 72_928)
        self.assertEqual(light.compare(expected, expected, cases)["mismatch_count"], 0)
        pinned = light.strict_json((ROOT / light.FIXTURES / "audit.json").read_bytes(), "test")
        with patch.object(light, "jar_path", return_value=Path("/tmp/pinned-light.jar")), \
                patch.object(light, "native_hashes", return_value=pinned["classes"]):
            light.validate_pinned(ROOT, pinned)
            bad_pinned = copy.deepcopy(pinned)
            bad_pinned["direct_counts"]["fd_a_IIII"] = True
            with self.assertRaises(light.LightError):
                light.validate_pinned(ROOT, bad_pinned)
        self.assertEqual(pinned["direct_counts"]["fd_a_IIII"], 24_608)
        self.assertEqual(pinned["direct_counts"]["xa_a_JF"], 48_064)

    def test_typed_domains_keep_negative_time_angles_unclamped(self):
        raw = (b"s\tsample\t0\t0\t0\t15\t7\n"
               b"night\ttime\t-1\t-9223372036854775808\t80000000\n"
               b"glow\temission\t89\n")
        cases = light.parse_input(raw)
        output = (b"s\tsample\t0\t3e000000\n"
                  b"night\ttime\tbf800000\t11\n"
                  b"glow\temission\t15\n")
        self.assertEqual(light.parse_output(output, cases, "fixture")[1][2], "bf800000")
        # A finite negative-time angle is validated by raw bits, without range clipping.
        for bad in (raw.replace(b"\t80000000\n", b"\t7f800000\n"),
                    raw.replace(b"\t80000000\n", b"\t3f800001\n"),
                    raw.replace(b"-9223372036854775808", b"9223372036854775808")):
            with self.subTest(bad=bad), self.assertRaises(light.LightError):
                light.parse_input(bad)
        with self.assertRaises(light.LightError):
            light.parse_output(output.replace(b"\t11\n", b"\t12\n"), cases)

    def test_input_framing_canonical_values_duplicates_and_limits(self):
        good = sample()
        invalid = [b"", good[:-1], good + b"\n", good + good,
                   b"x\tsample\t1\t0\t0\t0\t0\n", b"x\tsample\t0\t01\t0\t0\t0\n",
                   b"x\ttime\t0\t-0\t00000000\n", b"x\ttime\t0\t0\t0000000A\n",
                   b"x\ttime\t0\t0\t7fc00000\n", b"x\temission\t256\n", b"x\temission\t-1\n",
                   b"# comment\n", b"\xff\temission\t1\n", b"x\temission\t1\r\n",
                   good + b"late\tunknown\n", b"x\temission\t1\textra\n",
                   b"x" * (16 * 1024 * 1024 + 1)]
        for bad in invalid:
            with self.subTest(size=len(bad), prefix=bad[:50]), self.assertRaises(light.LightError):
                light.parse_input(bad)
        with self.assertRaises(light.LightError):
            light.parse_input(b"".join(f"x{i}\temission\t1\n".encode() for i in range(100_001)))

    def test_output_rejects_removed_added_reordered_and_wrongly_typed_rows(self):
        cases = light.parse_input(sample("a") + sample("b", block=9))
        good = b"a\tsample\t8\t3e800000\nb\tsample\t9\t3e800000\n"
        self.assertEqual(light.parse_output(good, cases), [
            ["a", "sample", "8", "3e800000"], ["b", "sample", "9", "3e800000"]])
        invalid = [good[:-1], good + b"extra\tsample\t0\t00000000\n",
                   b"b\tsample\t9\t3e800000\na\tsample\t8\t3e800000\n",
                   good.replace(b"3e800000", b"7f800000", 1),
                   good.replace(b"a\tsample", b"c\tsample", 1)]
        for bad in invalid:
            with self.subTest(bad=bad[:60]), self.assertRaises(light.LightError):
                light.parse_output(bad, cases)
        with self.assertRaises(light.LightError):
            light.parse_output(good + b"\n", cases)

    def test_comparator_finds_first_row_and_field_mutation(self):
        cases = light.parse_input(sample())
        baseline = b"s\tsample\t8\t3e800000\n"
        changed = b"s\tsample\t8\t3e800001\n"
        result = light.compare(baseline, changed, cases)
        self.assertEqual(result["mismatch_count"], 1)
        self.assertEqual((result["first_mismatch"]["record"], result["first_mismatch"]["component"]), (0, 1))

    def test_native_audit_derives_counts_hashes_source_and_constructor_seam(self):
        with tempfile.TemporaryDirectory() as temporary:
            jar = Path(temporary) / "client.jar"
            with zipfile.ZipFile(jar, "w") as archive:
                archive.writestr("fd.class", b"original fd")
            jar_hash = light.sha256(jar.read_bytes())
            classes = {"fd": light.sha256(b"original fd")}
            raw = (sample() + b"t\ttime\t-1\t-1\t80000000\n" + b"e\temission\t89\n")
            cases = light.parse_input(raw)
            output = b"s\tsample\t8\t3e800000\nt\ttime\t3f800000\t11\ne\temission\t15\n"
            boot = b"registry bootstrap\n"
            pinned = {"jar_sha256": jar_hash, "classes": classes,
                      "bootstrap_base64": __import__("base64").b64encode(boot).decode(),
                      "bootstrap_sha256": light.sha256(boot), "internal_calls": "native direct/inferred",
                      "weather_state": "clear", "provider_dimensions": "-1/0",
                      "special_light_query_ids_excluded": [44, 53, 60, 67],
                      "light_used_static_flag": "excluded"}
            audit = {**pinned, "code_source": str(jar), "input_sha256": light.sha256(raw),
                     "output_sha256": light.sha256(output),
                     "direct_counts": {"lm_c_explicit": 1, "fd_a_IIII": 1, "xa_a_JF": 1,
                                       "fd_a_F": 1, "xa_factory": 2, "xa_e": 2,
                                       "ei_a_J": 1, "uu_s_reads": 1},
                     "constructors": {"fd": 0, "synthetic_fd": 0, "lm": 2}}
            with patch.object(light, "JAR_SHA", jar_hash), patch.object(light, "native_hashes", return_value=classes):
                light.validate_audit(audit, cases, jar, pinned, raw, output)
                for key, value in (("code_source", str(Path(temporary) / "other.jar")),
                                   ("direct_counts", {**audit["direct_counts"], "fd_a_IIII": True}),
                                   ("constructors", {"fd": False, "synthetic_fd": 0, "lm": 2}),
                                   ("output_sha256", "0" * 64),
                                   ("bootstrap_base64", "not base64")):
                    with self.subTest(key=key), self.assertRaises(light.LightError):
                        light.validate_audit({**audit, key: value}, cases, jar, pinned, raw, output)

    def test_report_counts_reject_json_booleans(self):
        cases = light.parse_input(sample() + b"e\temission\t89\n")
        good = {"case_count": 2, "operation_counts": {"sample": 1, "emission": 1}}
        light.validate_report_counts(good, cases)
        for bad in ({**good, "case_count": True},
                    {**good, "operation_counts": {"sample": True, "emission": 1}}):
            with self.subTest(bad=bad), self.assertRaises(light.LightError):
                light.validate_report_counts(bad, cases)

    def test_captured_source_revision_allows_docs_only_descendant_but_rejects_code_change(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            def git(*args):
                return subprocess.run(["git", *args], cwd=root, capture_output=True, check=True).stdout
            git("init", "-q")
            git("config", "user.email", "test@example.invalid")
            git("config", "user.name", "LIGHT test")
            source = root / "src.rs"
            source.write_text("pub fn value() -> u8 { 1 }\n")
            git("add", "src.rs")
            git("commit", "-qm", "source")
            captured = git("rev-parse", "HEAD").decode().strip()
            snapshot = {"src.rs": light.sha256(source.read_bytes()), "missing.txt": None}
            (root / "README.md").write_text("docs only\n")
            git("add", "README.md")
            git("commit", "-qm", "docs")
            validate_revision_sources(captured, snapshot, root)
            source.write_text("pub fn value() -> u8 { 2 }\n")
            git("add", "src.rs")
            git("commit", "-qm", "source change")
            with self.assertRaisesRegex(light.LightError, "current source differs"):
                validate_revision_sources(captured, snapshot, root)

    def test_saved_review_rejects_missing_process_and_escaping_artifact(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "report"
            directory.mkdir()
            outside = Path(temporary) / "outside"
            outside.write_bytes(b"bytes")
            with self.assertRaisesRegex(light.LightError, "escapes"):
                retained(outside, directory)
            with self.assertRaisesRegex(SystemExit, "successful LIGHT"):
                verify_light(directory / "report.json", {"suite": "light", "status": "error"})


@unittest.skipUnless(os.environ.get("BETA_LIVE_LIGHT") == "1",
                     "set BETA_LIVE_LIGHT=1 for original-JAR and live protocol tests")
class LiveLightEvidenceTests(unittest.TestCase):
    def test_strict_live_report_saved_review_and_protocol(self):
        report, path = light.run_light_suite()
        self.assertEqual(report["status"], "pass", report.get("reason"))
        self.assertIs(report["rust_dirty"], False)
        self.assertEqual(verify_light(path, report, protocol=True), 0)
        for field, value in (("rust_dirty", True), ("rust_revision", "0" * 40),
                             ("mismatch_count", 1), ("rust_output_sha256", "0" * 64)):
            changed = copy.deepcopy(report)
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(SystemExit):
                verify_light(path, changed)
        for label in ("java-run", "java-repeat", "rust-run", "rust-build"):
            changed = copy.deepcopy(report)
            changed["processes"][label]["returncode"] = False
            with self.subTest(process=label), self.assertRaises(SystemExit):
                verify_light(path, changed)
        for name in ("rust_binary", "java-run-audit", "input"):
            artifact = Path(report["log_paths"][name])
            original = artifact.read_bytes()
            try:
                artifact.write_bytes(original + b"mutation")
                with self.subTest(artifact=name), self.assertRaises(SystemExit):
                    verify_light(path, report)
            finally:
                artifact.write_bytes(original)


if __name__ == "__main__":
    unittest.main()
