"""Native Beta water geometry parity runner and retained evidence producer."""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib

try:
    import run as parity
except ImportError:
    from tools.parity import run as parity

try:
    from tools.oracle import fluid_corpus
except ImportError:
    sys.path.insert(0, str(parity.ROOT / "tools/oracle"))
    import fluid_corpus

ROOT = parity.ROOT
JAR_SHA256 = fluid_corpus.JAR_SHA
FIXTURE = Path("tests/fixtures/fluid/native.tsv")
INPUT = Path("tests/fixtures/fluid/input.tsv")
ORACLE = Path("tools/oracle/src/FluidOracle.java")
CORPUS = Path("tools/oracle/fluid_corpus.py")
PROBE = Path("crates/beta-viewer/examples/fluid_probe.rs")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def source_snapshot(root):
    paths = {FIXTURE, INPUT, Path("tests/fixtures/fluid/audit.json"),
             Path("tests/fixtures/fluid/provenance.md"), ORACLE, CORPUS,
             Path("tools/parity/run.py"), Path("tools/parity/fluid_run.py"),
             Path("tools/parity/fluid_review.py"), Path("config/local.toml"),
             Path("Cargo.toml"), Path("Cargo.lock"), Path("rust-toolchain.toml"), PROBE}
    paths.update(p.relative_to(root) for p in (root / "tests/fixtures/fluid").glob("*"))
    paths.update(p.relative_to(root) for p in (root / "tools/parity").glob("*.py"))
    return {str(path): sha256((root / path).read_bytes()) if (root / path).is_file() else None
            for path in sorted(paths)}


def geometry_rows(raw):
    """Validate the saved native stream and return case/vertex rows for Rust compare."""
    fluid_corpus.parse_native(raw)
    rows = []
    for line in raw.splitlines():
        if line.startswith(b"case\t"):
            fields = line.split(b"\t")
            rows.append(line + b"\n")
        elif line.startswith(b"vertex\t"):
            fields = line.split(b"\t")
            rows.append(b"\t".join(fields[:2] + fields[2:]) + b"\n")
    return b"".join(rows)


def parse_probe(raw):
    lines = raw.splitlines(keepends=True)
    if not lines:
        raise ValueError("fluid probe produced empty output")
    cases = []
    i = 0
    while i < len(lines):
        row = lines[i].rstrip(b"\n").split(b"\t")
        if not lines[i].endswith(b"\n") or len(row) != 6 or row[0] != b"case":
            raise ValueError("fluid probe case row has invalid shape")
        try:
            count = int(row[5])
        except ValueError as error:
            raise ValueError("fluid probe vertex count is not decimal") from error
        if count <= 0:
            raise ValueError("fluid probe case must emit vertices")
        cases.append(row[1])
        i += 1
        for _ in range(count):
            if i >= len(lines) or not lines[i].endswith(b"\n"):
                raise ValueError("fluid probe vertex section is truncated")
            vertex = lines[i].rstrip(b"\n").split(b"\t")
            if (len(vertex) != 10 or vertex[0] != b"vertex" or vertex[1] != row[1]
                    or any(len(word) != 8 or any(c not in b"0123456789abcdef" for c in word)
                           for word in vertex[2:])):
                raise ValueError("fluid probe vertex row is malformed or unordered")
            i += 1
    if cases != [name.encode("ascii") for name in fluid_corpus.SAMPLES]:
        raise ValueError("fluid probe case list/order differs from frozen C01 input")
    return b"".join(lines)


def compare_geometry(reference, actual):
    try:
        expected = geometry_rows(reference)
        observed = parse_probe(actual)
    except (ValueError, UnicodeError) as error:
        return {"mismatch_count": 1, "first_mismatch": {"record": None, "error": str(error)}}
    left, right = expected.splitlines(keepends=True), observed.splitlines(keepends=True)
    first = next((i for i in range(max(len(left), len(right)))
                  if i >= len(left) or i >= len(right) or left[i] != right[i]), None)
    return {"mismatch_count": int(first is not None),
            "first_mismatch": None if first is None else {
                "record": first,
                "reference": left[first].decode("ascii", "replace") if first < len(left) else None,
                "rust": right[first].decode("ascii", "replace") if first < len(right) else None}}


def _record(directory, report, label, command, timeout, root, input_data=None):
    result = parity.run_process(command, cwd=root, directory=directory, label=label,
                                timeout=timeout, input_data=input_data)
    report["processes"][label] = result
    if result["error"] or result["returncode"] != 0:
        raise parity.RunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
    return Path(result["stdout"]).read_bytes()


def run_fluid(root=ROOT, timeout=120):
    parent = parity.artifacts_root(root) / "parity/fluid"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {
        "schema_version": 1, "suite": "fluid", "scenario_id": "FLUID-001-water-native-geometry",
        "repository_root": str(root.resolve()), "started_at": datetime.now(timezone.utc).isoformat(),
        "status": "error", "reason": "comparison not completed",
        "reference_side": "Pinned original Beta 1.7.3 cv renderer / nw.g Tessellator",
        "reference_jar_path": None, "reference_jar_sha256": None,
        "reference_class_sha256": {}, "native_audit": {}, "rust_revision": None,
        "rust_dirty": None, "rust_binary_sha256": None, "source_hashes": {},
        "source_hashes_after": {}, "input_sha256": None, "expected_fixture_sha256": None,
        "reference_output_sha256": None, "rust_output_sha256": None,
        "case_count": 0, "vertex_count": 0, "mismatch_count": None,
        "first_mismatch": None, "duration_ms": None, "processes": {},
        "log_paths": {"report": str(report_path)}, "platform": platform.platform(),
        "comparison_policy": "exact ordered case/vertex TSV; all eight native 32-bit words compare exactly",
        "scope": "water IDs 8/9 geometry only; no lava/flow simulation/GPU pixel parity",
    }
    initial = None
    jar = None
    started = time.monotonic()
    try:
        paths = tomllib.loads((root / "config/local.toml").read_text())["paths"]
        jar = (root / paths["client_jar"]).resolve(strict=True)
        if sha256(jar.read_bytes()) != JAR_SHA256:
            raise parity.RunError("configured client JAR SHA-256 differs from pinned Beta 1.7.3")
        report.update(reference_jar_path=str(jar), reference_jar_sha256=JAR_SHA256)
        initial = source_snapshot(root)
        report["source_hashes"] = initial
        expected = (root / FIXTURE).read_bytes()
        input_data = (root / INPUT).read_bytes()
        report["input_sha256"] = sha256(input_data)
        report["expected_fixture_sha256"] = sha256(expected)
        report["log_paths"].update(input=str(root / INPUT), expected_output=str(root / FIXTURE))
        fluid_corpus.parse_native(expected)
        (directory / "expected-native.tsv").write_bytes(expected)
        report["log_paths"]["retained_expected"] = str(directory / "expected-native.tsv")

        outputs = []
        for run_name in ("native-run", "native-repeat"):
            class_dir = directory / run_name / "classes"
            class_dir.mkdir(parents=True)
            _record(directory, report, run_name + "-javac",
                    ["javac", "-cp", str(jar), "-d", str(class_dir), str(root / ORACLE)],
                    min(timeout, 40), root)
            java_output = _record(directory, report, run_name,
                                 ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
                                  str(class_dir) + os.pathsep + str(jar),
                                  "FluidOracle", str(jar)], timeout, root)
            stderr = Path(report["processes"][run_name]["stderr"]).read_bytes()
            if stderr:
                raise parity.RunError(f"{run_name}: unexpected native stderr output")
            fluid_corpus.parse_native(java_output)
            saved = directory / f"{run_name}.native.tsv"
            saved.write_bytes(java_output)
            report["log_paths"][run_name + "_output"] = str(saved)
            outputs.append(java_output)
            class_file = class_dir / "FluidOracle.class"
            if not class_file.is_file():
                raise parity.RunError("javac did not retain FluidOracle.class")
            report["adapter_class_sha256"] = sha256(class_file.read_bytes())
        if outputs[0] != outputs[1] or outputs[0] != expected:
            raise parity.RunError("repeated native runs or C01 reference fixture differ")
        report["reference_output_sha256"] = sha256(outputs[0])
        parsed = fluid_corpus.parse_native(outputs[0])
        report.update(native_audit={"native_sha256": parsed["native_sha256"],
                                    "native_calls": parsed["native_calls"],
                                    "repeatable": True},
                      reference_class_sha256=fluid_corpus.EXPECTED_CLASSES,
                      case_count=parsed["cases"], vertex_count=parsed["vertices"])

        if not (root / PROBE).is_file():
            report.update(status="not_implemented",
                          reason="Rust fluid_probe source is absent; native C01 oracle ran twice")
        else:
            report["rust_revision"] = _record(directory, report, "git-revision",
                                                ["git", "rev-parse", "HEAD"], timeout, root).decode().strip()
            dirty = _record(directory, report, "git-dirty", ["git", "status", "--porcelain"], timeout, root)
            report["rust_dirty"] = bool(dirty.strip())
            build = _record(directory, report, "rust-build",
                            ["cargo", "build", "--locked", "--offline", "-p", "beta-viewer",
                             "--example", "fluid_probe", "--message-format=json-render-diagnostics"], timeout, root)
            binaries = [Path(obj["executable"]) for line in build.splitlines()
                        if (obj := json.loads(line)).get("reason") == "compiler-artifact"
                        and obj.get("target", {}).get("name") == "fluid_probe" and obj.get("executable")]
            if len(binaries) != 1:
                raise parity.RunError("cargo did not identify exactly one fluid_probe executable")
            binary = directory / "fluid_probe"
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            report["log_paths"]["rust_binary"] = str(binary)
            rust = _record(directory, report, "rust-run", [str(binary)], timeout, root,
                           input_data=input_data)
            report["rust_output_sha256"] = sha256(rust)
            report["log_paths"]["rust_output"] = report["processes"]["rust-run"]["stdout"]
            comparison = compare_geometry(outputs[0], rust)
            report.update(comparison,
                          status="pass" if comparison["mismatch_count"] == 0 else "fail",
                          reason="all captured native water vertices match exactly" if comparison["mismatch_count"] == 0
                          else "native water geometry mismatch")
        report["log_paths"].update(reference_stdout=report["processes"]["native-run"]["stdout"],
                                   repeat_stdout=report["processes"]["native-repeat"]["stdout"])
        if sha256(jar.read_bytes()) != JAR_SHA256:
            raise parity.RunError("client JAR changed during native fluid run")
    except (parity.RunError, OSError, ValueError, KeyError, TypeError,
            json.JSONDecodeError, subprocess.SubprocessError, RuntimeError) as error:
        report.update(status="error", reason=str(error))
    finally:
        if initial is not None:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if after != initial:
                report.update(status="error", reason="tracked inputs changed during fluid parity run")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("--protocol", action="store_true", help="verify saved report after producing it")
    args = parser.parse_args(argv)
    report, path = run_fluid(timeout=args.timeout)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    if args.protocol:
        try:
            from fluid_review import verify_report
        except ImportError:
            from tools.parity.fluid_review import verify_report
        verify_report(path)
        print("saved fluid report protocol: PASS")
    return parity.exit_code(report["status"], strict=True)


if __name__ == "__main__":
    raise SystemExit(main())
