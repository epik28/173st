import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIX = ROOT / "tests/fixtures/light_update"
SCRIPT = ROOT / "tools/oracle/light_update_audit.py"

spec = importlib.util.spec_from_file_location("light_update_audit", SCRIPT)
audit_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit_module)


class LightUpdateFixtureTests(unittest.TestCase):
    def test_manifest_and_native_sky_deltas(self):
        output = (FIX / "native.tsv").read_bytes()
        rows = [line for line in output.decode().splitlines() if line.count("\t") == 11]
        self.assertEqual(len(rows), 126)
        self.assertIn("edge\tafter\t15\t63\t8", "\n".join(rows))
        self.assertIn("edge-x31\tafter\t32\t63\t8\tUNLOADED", "\n".join(rows))
        audit_module.validate_deltas(output)

        audit = json.loads((FIX / "audit.json").read_text())
        for field in audit_module.MANIFEST_FIELDS:
            self.assertIn(field, audit)
        self.assertEqual(audit["jar_sha256"], audit_module.JAR_SHA)
        self.assertEqual(audit["emission"].split(":", 1)[0], "excluded")
        self.assertEqual(audit["seed_schema"]["loaded_chunk_coordinates"], {"x": [-1, 0, 1], "z": [-1, 0, 1]})
        self.assertIn("native lm.c()", audit["seed_schema"]["initial_sky_light"])

    def test_delta_validator_rejects_missing_or_false_sky_rows(self):
        output = (FIX / "native.tsv").read_bytes()
        missing = output.replace(b"remove\tafter\t8\t63\t8\t0\t0\t0\t0\t15\t0\t15\n", b"", 1)
        with self.assertRaisesRegex(ValueError, "missing required"):
            audit_module.validate_deltas(missing)
        false = output.replace(b"place\tafter\t8\t63\t8\t0\t0\t0\t0\t14\t0\t14", b"place\tafter\t8\t63\t8\t0\t0\t0\t0\t15\t0\t15", 1)
        with self.assertRaisesRegex(ValueError, "false sky/combined-light"):
            audit_module.validate_deltas(false)

    def test_domain_rows_reject_wrong_profile_and_redundant_exceptions(self):
        output = (FIX / "native.tsv").read_bytes()
        wrong_profile = output.replace(
            b"default_sky=0@y0;15@y1..127(native-lm.c-all-air)",
            b"default_sky=15(native-lm.c-all-air)",
            1,
        )
        with self.assertRaisesRegex(ValueError, "bounded default schema"):
            audit_module.validate_domain_records(wrong_profile)

        redundant = output.replace(
            b"place\tbefore\t8\t66\t8",
            b"state\tplace\t8\t1\t8\t0\t0\t0\t15\nplace\tbefore\t8\t66\t8",
            1,
        )
        with self.assertRaisesRegex(ValueError, "redundant native state exception"):
            audit_module.validate_domain_records(redundant)

    def test_boundary_validator_rejects_missing_or_air_unloaded_cell(self):
        output = (FIX / "native.tsv").read_bytes()
        sentinel = b"edge-x31\tafter\t32\t63\t8\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED\n"
        with self.assertRaisesRegex(ValueError, "missing required"):
            audit_module.validate_deltas(output.replace(sentinel, b"", 1))

        air = b"edge-x31\tafter\t32\t63\t8\t0\t0\t0\t0\t0\t0\t0\n"
        with self.assertRaisesRegex(ValueError, "explicitly UNLOADED"):
            audit_module.validate_deltas(output.replace(sentinel, air, 1))

        false_sky = output.replace(
            b"edge-x31\tafter\t31\t63\t8\t0\t0\t0\t0\t0\t0\t0",
            b"edge-x31\tafter\t31\t63\t8\t0\t0\t0\t0\t15\t0\t15",
            1,
        )
        with self.assertRaisesRegex(ValueError, "native sky/block/combined light must be 0"):
            audit_module.validate_deltas(false_sky)

    def test_wrong_jar_is_rejected_by_cli_without_output(self):
        with tempfile.NamedTemporaryFile() as wrong_jar:
            wrong_jar.write(b"not the pinned client jar")
            wrong_jar.flush()
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--jar", wrong_jar.name],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"wrong or missing pinned client JAR", result.stderr)

    def test_evidence_validator_rejects_hash_and_class_mutations(self):
        manifest = json.loads((FIX / "audit.json").read_text())
        output = (FIX / "native.tsv").read_bytes()
        input_bytes = (FIX / "input.tsv").read_bytes()
        source_bytes = (ROOT / "tools/oracle/src/LightUpdateOracle.java").read_bytes()
        with tempfile.TemporaryDirectory() as temp:
            jar_path = pathlib.Path(temp) / "synthetic-client.jar"
            # Use tiny synthetic class files so this pure validator test does
            # not need javac; the live runner separately verifies real classes.
            names = tuple(manifest.get("class_sha256", {})) or ("fd", "lm", "wi", "uu", "eb", "st")
            manifest["class_sha256"] = {}
            with zipfile.ZipFile(jar_path, "w") as archive:
                for name in names:
                    content = f"class fixture {name}".encode()
                    archive.writestr(name.replace(".", "/") + ".class", content)
                    manifest["class_sha256"][name] = audit_module.sha(content)
            manifest["jar_sha256"] = audit_module.sha(jar_path.read_bytes())
            manifest["input_sha256"] = audit_module.sha(input_bytes)
            manifest["output_sha256"] = audit_module.sha(output)
            manifest["source_sha256"] = audit_module.sha(source_bytes)
            manifest.setdefault("seed_schema", {"test": "synthetic"})
            runtime = [dict(manifest), dict(manifest)]
            audit_module.validate_evidence(
                [output, output], runtime, output, input_bytes, manifest, source_bytes, jar_path
            )

            wrong_output = [dict(manifest), dict(manifest)]
            wrong_output[0]["output_sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "output_sha256"):
                audit_module.validate_evidence(
                    [output, output], wrong_output, output, input_bytes, manifest, source_bytes, jar_path
                )

            wrong_source = dict(manifest)
            wrong_source["source_sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "source hash"):
                audit_module.validate_evidence(
                    [output, output], runtime, output, input_bytes, wrong_source, source_bytes, jar_path
                )

            wrong_class = dict(manifest)
            wrong_class["class_sha256"] = dict(manifest["class_sha256"])
            name = next(iter(wrong_class["class_sha256"]))
            wrong_class["class_sha256"][name] = "0" * 64
            with self.assertRaisesRegex(ValueError, "pinned JAR class hash"):
                audit_module.validate_evidence(
                    [output, output], runtime, output, input_bytes, wrong_class, source_bytes, jar_path
                )

            missing_class = dict(manifest)
            missing_class["class_sha256"] = dict(manifest["class_sha256"])
            del missing_class["class_sha256"]["fd"]
            with self.assertRaisesRegex(ValueError, "exactly fd/lm/wi/uu/eb/st"):
                audit_module.validate_evidence(
                    [output, output], runtime, output, input_bytes, missing_class, source_bytes, jar_path
                )

            wrong_runtime_class = [dict(manifest), dict(manifest)]
            wrong_runtime_class[1]["class_sha256"] = dict(manifest["class_sha256"])
            wrong_runtime_class[1]["class_sha256"][name] = "0" * 64
            with self.assertRaisesRegex(ValueError, "capture 2 audit class_sha256"):
                audit_module.validate_evidence(
                    [output, output], wrong_runtime_class, output, input_bytes, manifest, source_bytes, jar_path
                )

            wrong_runtime_source = [dict(manifest), dict(manifest)]
            wrong_runtime_source[1]["source_sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "capture 2 audit source_sha256"):
                audit_module.validate_evidence(
                    [output, output], wrong_runtime_source, output, input_bytes, manifest, source_bytes, jar_path
                )

            missing_schema = dict(manifest)
            del missing_schema["seed_schema"]
            with self.assertRaisesRegex(ValueError, "missing fields: seed_schema"):
                audit_module.validate_evidence(
                    [output, output], runtime, output, input_bytes, missing_schema, source_bytes, jar_path
                )

            changed_schema = dict(manifest)
            changed_schema["seed_schema"] = dict(manifest["seed_schema"])
            changed_schema["seed_schema"]["block_domain"] = {"y_min": 0, "y_max_inclusive": 126}
            with self.assertRaisesRegex(ValueError, "seed_schema does not match"):
                audit_module.validate_evidence(
                    [output, output], runtime, output, input_bytes, changed_schema, source_bytes, jar_path
                )


if __name__ == "__main__":
    unittest.main()
