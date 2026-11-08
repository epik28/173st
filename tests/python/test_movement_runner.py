"""Typed motion traces and native audit accounting, without a game installation."""
import copy
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/parity"))
import movement_run as m
from movement_review import _validate_saved_paths, verify_movement


def bits(number, width=64):
    return struct.pack(">d" if width == 64 else ">f", number).hex()


def state(flags=128):
    values = [0., 1., 0., 0., 0., 0., -.3, 1., -.3, .3, 2.8, .3]
    return ",".join([*(bits(x) for x in values), *(bits(x, 32) for x in (0., 0., .5, 0., 0.)), str(flags)])


def scenario(mode="move", frames=None, initial=None, boxes="-", surface="1", case="test"):
    if frames is None:
        frames = ",".join([bits(0.)] * 3) if mode == "move" else ",".join([bits(0., 32)] * 3 + ["0", "0"])
    return "\t".join([case, mode, initial or state(), boxes, surface, frames]).encode() + b"\n"


class MovementRunnerTests(unittest.TestCase):
    def test_frames_and_first_component_divergence_are_ordered(self):
        frame = ",".join([bits(0.)] * 3)
        cases = m.parse_input(scenario(frames=frame + ";" + frame))
        expected = f"test\t0\t{state()}\ntest\t1\t{state()}\n".encode()
        fields = state().split(",")
        fields[3] = bits(-0.)
        actual = f"test\t0\t{state()}\ntest\t1\t{','.join(fields)}\n".encode()
        mismatch = m.compare(expected, actual, cases)
        self.assertEqual(mismatch["mismatch_count"], 1)
        self.assertEqual(mismatch["first_mismatch"]["frame"], 1)
        self.assertEqual(mismatch["first_mismatch"]["component"], 3)

    def test_input_rejects_bad_framing_modes_and_domains(self):
        invalid = [b"", scenario()[:-1], scenario() + b"\n", scenario() * 2,
                   scenario(mode="unknown"), scenario(surface="2"),
                   scenario(frames=bits(33.) + "," + bits(0.) + "," + bits(0.)),
                   scenario(frames=",".join([bits(float("nan"))] * 3)),
                   scenario(mode="travel", frames=",".join([bits(1.1, 32), bits(0., 32), bits(0., 32), "0", "0"])),
                   scenario(boxes=",".join([bits(0.)] * 6)),
                   scenario(initial=state().replace(",128", ",0255")),
                   scenario(initial=state().replace(bits(.3), bits(30_000_001.))), b"# comment\n"]
        for raw in invalid:
            with self.subTest(raw=raw[:50]), self.assertRaises(m.MovementError):
                m.parse_input(raw)

    def test_body_dimensions_heading_width_and_frame_limits_are_validated(self):
        fields = state().split(",")
        fields[9] = fields[6]
        with self.assertRaises(m.MovementError):
            m.parse_input(scenario(initial=",".join(fields)))
        frame = ",".join([bits(0.)] * 3)
        with self.assertRaises(m.MovementError):
            m.parse_input(scenario(frames=";".join([frame] * 257)))
        self.assertEqual(len(m.parse_input(scenario(frames=";".join([frame] * 256)))[0]["frames"]), 256)
        with self.assertRaises(m.MovementError):
            m.parse_input(scenario(mode="travel", frames=",".join([bits(0.)] * 3 + ["0", "0"])))

    def test_output_rejects_index_order_count_types_and_lf(self):
        cases = m.parse_input(scenario())
        valid = f"test\t0\t{state()}\n".encode()
        self.assertEqual(len(m.parse_output(valid, cases, "test")), 1)
        for output in (valid[:-1], valid * 2, valid.replace(b"test\t0", b"test\t01"),
                       valid.replace(b"test", b"other"), valid.replace(b"\t0\t", b"\t1\t"), b"test\t0\tbad\n"):
            with self.subTest(output=output[:30]), self.assertRaises(m.MovementError):
                m.parse_output(output, cases, "test")

    def test_jump_audit_uses_grounded_state_before_each_frame(self):
        frame = ",".join([bits(0., 32)] * 3 + ["1", "0"])
        cases = m.parse_input(scenario(mode="travel", initial=state(129), frames=";".join([frame] * 3)))
        self.assertEqual(m.primary_invocations(cases, [state(128), state(129), state(128)]), {"travel": 3, "jump": 2})
        cases = m.parse_input(scenario())
        self.assertEqual(m.primary_invocations(cases, [state()]), {"move": 1})

    def test_saved_paths_are_bound_to_the_commands_and_sidecars(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            logs = {"rust_binary": str(directory / "movement_probe"),
                    "java_classes": str(directory / "classes"), "input": str(directory / "input.tsv")}
            for label in ("java-run", "java-repeat"):
                audit = directory / (label + "-audit.json")
                logs[label + "-audit"] = str(audit)
                logs[label + "-registry"] = str(audit) + ".registry.stdout"
            _validate_saved_paths(logs, directory)

            copied_binary = directory / "movement_probe-copy"
            copied_binary.write_bytes(b"retained copy")
            bad = dict(logs, rust_binary=str(copied_binary))
            with self.assertRaisesRegex(ValueError, "rust_binary path"):
                _validate_saved_paths(bad, directory)

            copied_classes = directory / "classes-copy"
            copied_classes.mkdir()
            bad = dict(logs, java_classes=str(copied_classes))
            with self.assertRaisesRegex(ValueError, "java_classes path"):
                _validate_saved_paths(bad, directory)

            bad = dict(logs, **{"java-run-audit": str(directory / "copied-audit.json")})
            with self.assertRaisesRegex(ValueError, "audit path"):
                _validate_saved_paths(bad, directory)

            bad = dict(logs, **{"java-repeat-registry": str(directory / "copied.registry.stdout")})
            with self.assertRaisesRegex(ValueError, "registry path"):
                _validate_saved_paths(bad, directory)

            bad = dict(logs, input=str(directory / "input-copy.tsv"))
            with self.assertRaisesRegex(ValueError, "input path"):
                _validate_saved_paths(bad, directory)

    def test_audit_pins_registry_capture_and_rejects_boolean_counts(self):
        pinned = json.loads((ROOT / m.FIXTURES / "audit.json").read_text())
        audit = copy.deepcopy(pinned)
        audit.update(case_count=1, state_count=1, invocations={"move": 1})
        cases = m.parse_input(scenario())
        jar = Path(__import__("urllib.parse", fromlist=["urlparse"]).urlparse(
            audit["code_sources"]["sn"]).path)
        hashes = audit["class_sha256"]
        with patch.object(m, "native_hashes", return_value=hashes):
            m.validate_audit(audit, cases, [state()], pinned, jar)

            for mutation in (
                {"registry_stdout_bytes": 1, "registry_stdout_sha256": "0" * 64},
                {"native_world_constructors": False},
            ):
                changed = {**audit, **mutation}
                with self.subTest(mutation=mutation), self.assertRaises(m.MovementError):
                    m.validate_audit(changed, cases, [state()], pinned, jar)

    def test_missing_original_is_error_and_saved_verifier_rejects_missing_process(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "config").mkdir()
            (root / "config/local.toml").write_text('[paths]\nartifacts="artifacts"\nclient_jar="missing.jar"\n')
            report, path = m.run_movement_suite(root=root)
            self.assertEqual(report["status"], "error")
            self.assertTrue(path.is_file())
            self.assertIn("missing.jar", report["reason"])
            with patch("movement_review.jar_path", return_value=Path("/tmp/fake.jar")):
                with self.assertRaises(SystemExit):
                    verify_movement(path, {"suite": "movement", "status": "pass", "environment": "synthetic-static-dry",
                                           "native_world_constructors": 0, "processes": {}, "log_paths": {}})

    @unittest.skipUnless((ROOT / m.FIXTURES / "input.tsv").is_file(), "C01 fixture not integrated yet")
    def test_pinned_native_corpus_shape_hashes_and_all_states(self):
        cases = m.parse_input((ROOT / m.FIXTURES / "input.tsv").read_bytes())
        reference = (ROOT / m.FIXTURES / "reference.tsv").read_bytes()
        values = m.parse_output(reference, cases, "native fixture")
        self.assertGreaterEqual(len(values), 5000)
        self.assertEqual(m.compare(reference, reference, cases)["mismatch_count"], 0)
        m.validate_pinned(ROOT, json.loads((ROOT / m.FIXTURES / "audit.json").read_text()))


@unittest.skipUnless(os.environ.get("BETA_LIVE_MOVEMENT") == "1", "opt-in native trace mutations: BETA_LIVE_MOVEMENT=1")
class LiveMovementTests(unittest.TestCase):
    def test_native_trace_and_retained_artifact_mutations(self):
        report, path = m.run_movement_suite()
        self.assertEqual(report["status"], "pass", report["reason"])
        self.assertEqual(verify_movement(path, report, protocol=True), 0)
        for mutate in (lambda r: r["processes"]["java-run"].update(command=["/bin/false"]),
                       lambda r: r["processes"]["rust-run"].update(returncode=1),
                       lambda r: r.update(rust_binary_sha256="0" * 64),
                       lambda r: r.update(adapter_class_hashes={}),
                       lambda r: r.update({"java-run-audit-sha256": "0" * 64}),
                       lambda r: r.update(source_hashes={}), lambda r: r.update(state_count=0),
                       lambda r: r.update(mismatch_count=1)):
            altered = copy.deepcopy(report)
            mutate(altered)
            with self.assertRaises(SystemExit):
                verify_movement(path, altered, protocol=True)
        for label in ("rust_binary", "java-run-audit", "java-run-registry", "input"):
            artifact = Path(report["log_paths"][label])
            original = artifact.read_bytes()
            try:
                artifact.write_bytes(original + b"changed")
                with patch("movement_review.subprocess.run") as run:
                    with self.assertRaises(SystemExit):
                        verify_movement(path, report, protocol=True)
                    run.assert_not_called()
            finally:
                artifact.write_bytes(original)
        copied_binary = Path(report["log_paths"]["rust_binary"]).with_name("movement_probe-copy")
        shutil.copy2(report["log_paths"]["rust_binary"], copied_binary)
        try:
            changed = copy.deepcopy(report)
            changed["log_paths"]["rust_binary"] = str(copied_binary)
            changed["rust_binary_sha256"] = m.sha256(copied_binary.read_bytes())
            with self.assertRaisesRegex(SystemExit, "rust_binary path"):
                verify_movement(path, changed)
        finally:
            copied_binary.unlink()

        copied_classes = Path(report["log_paths"]["java_classes"]).with_name("classes-copy")
        shutil.copytree(report["log_paths"]["java_classes"], copied_classes)
        try:
            changed = copy.deepcopy(report)
            changed["log_paths"]["java_classes"] = str(copied_classes)
            with self.assertRaisesRegex(SystemExit, "java_classes path"):
                verify_movement(path, changed)
        finally:
            shutil.rmtree(copied_classes)

        original_audits = []
        original_audit_hashes = {label: report[label + "-audit-sha256"]
                                 for label in ("java-run", "java-repeat")}
        try:
            for label in ("java-run", "java-repeat"):
                audit_path = Path(report["log_paths"][label + "-audit"])
                registry_path = Path(report["log_paths"][label + "-registry"])
                original_audits.append((audit_path, audit_path.read_bytes(), registry_path, registry_path.read_bytes()))
                audit = json.loads(audit_path.read_bytes())
                mutated_capture = registry_path.read_bytes() + b"mutated native registry capture"
                audit["registry_stdout_bytes"] = len(mutated_capture)
                audit["registry_stdout_sha256"] = __import__("hashlib").sha256(mutated_capture).hexdigest()
                audit_bytes = (json.dumps(audit, indent=2) + "\n").encode()
                audit_path.write_bytes(audit_bytes)
                registry_path.write_bytes(mutated_capture)
                report[label + "-audit-sha256"] = __import__("hashlib").sha256(audit_bytes).hexdigest()
            with self.assertRaisesRegex(SystemExit, "pinned manifest"):
                verify_movement(path, report)
        finally:
            for audit_path, audit_bytes, registry_path, registry_bytes in original_audits:
                audit_path.write_bytes(audit_bytes)
                registry_path.write_bytes(registry_bytes)
            for label, digest in original_audit_hashes.items():
                report[label + "-audit-sha256"] = digest
        with patch("movement_review.subprocess.run", return_value=subprocess.CompletedProcess([], 1, b"", b"")):
            with self.assertRaisesRegex(SystemExit, "omitted diagnostic"):
                verify_movement(path, report, protocol=True)


if __name__ == "__main__":
    unittest.main()
