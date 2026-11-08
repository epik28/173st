"""Strict, retained Beta block targeting parity runner (ray-only until actions exist)."""

from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import tomllib
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

try:
    import run as parity
except ImportError:
    from tools.parity import run as parity

try:
    from tools.oracle import block_corpus
except ImportError:
    from tools.oracle import block_corpus

ROOT = parity.ROOT
FIXTURE = Path("tests/fixtures/block/native.tsv")
AUDIT = Path("tests/fixtures/block/audit.json")
PROVENANCE = Path("tests/fixtures/block/provenance.md")
ORACLE = Path("tools/oracle/src/BlockOracle.java")
CORPUS = Path("tools/oracle/block_corpus.py")
PROBE = Path("crates/beta-core/examples/block_probe.rs")
JAVA_HEADER = b"#BLOCK_PROBE\t1\tray-only"
PROBE_FIELDS = ("id", "kind", "x", "y", "z", "side", "px", "py", "pz",
                "px_bits", "py_bits", "pz_bits")
PROBE_HEADER = b"#id\t" + b"\t".join(x.encode() for x in PROBE_FIELDS[1:])
MAX_INPUT_BYTES = 4096
MAX_OUTPUT_BYTES = 64 * 1024


class BlockRunError(parity.RunError):
    pass


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def probe_input():
    raw = (ROOT / "tests/fixtures/block/input.tsv").read_bytes()
    if len(raw) > MAX_INPUT_BYTES:
        raise BlockRunError("block probe input limit exceeded")
    try:
        block_corpus.parse_inputs(raw)
    except ValueError as error:
        raise BlockRunError(f"invalid pinned ray input fixture: {error}") from error
    return raw


def parse_probe(raw):
    if len(raw) > MAX_OUTPUT_BYTES:
        raise ValueError("block probe output limit exceeded")
    try:
        lines = raw.decode("ascii").splitlines(keepends=True)
    except UnicodeDecodeError as error:
        raise ValueError("block probe output must be ASCII") from error
    if len(lines) != len(block_corpus.EXPECTED_ORDER) + 2:
        raise ValueError("block probe output has wrong row count")
    if lines[0].rstrip("\n").encode() != JAVA_HEADER or not lines[0].endswith("\n"):
        raise ValueError("block probe output has wrong schema/version/capability header")
    if lines[1].rstrip("\n").encode() != PROBE_HEADER or not lines[1].endswith("\n"):
        raise ValueError("block probe output fields differ from ray-only schema")
    output = []
    for line, expected_id in zip(lines[2:], block_corpus.EXPECTED_ORDER, strict=True):
        if not line.endswith("\n") or "\r" in line:
            raise ValueError("block probe row has invalid line ending")
        cols = line[:-1].split("\t")
        if len(cols) != len(PROBE_FIELDS) or cols[0] != expected_id:
            raise ValueError("block probe row is malformed, extra, missing, or out of order")
        row = dict(zip(PROBE_FIELDS, cols, strict=True))
        if row["kind"] not in {"block", "miss"}:
            raise ValueError("block probe hit kind is invalid")
        if row["kind"] == "miss":
            if any(row[name] != "-" for name in PROBE_FIELDS[2:]):
                raise ValueError("block probe miss row contains hit fields")
        else:
            for name in ("x", "y", "z"):
                if not _canonical_integer(row[name]):
                    raise ValueError("block probe coordinate is not canonical signed decimal")
            if row["side"] not in {"0", "1", "2", "3", "4", "5"}:
                raise ValueError("block probe side is outside 0..5")
            for value_name, bits_name in (("px", "px_bits"), ("py", "py_bits"), ("pz", "pz_bits")):
                value, bits = row[value_name], row[bits_name]
                if len(bits) != 16 or any(c not in "0123456789abcdef" for c in bits):
                    raise ValueError("block probe raw double word must be 16 lowercase hex digits")
                try:
                    number = float(value)
                except ValueError as error:
                    raise ValueError("block probe point is not a decimal double") from error
                if not math.isfinite(number) or struct.pack(">d", number).hex() != bits:
                    raise ValueError("block probe decimal point disagrees with raw double word")
        output.append(row)
    return output


def _canonical_integer(value):
    if value == "0":
        return True
    if value.startswith("-"):
        return len(value) > 1 and value[1] in "123456789" and value[1:].isdigit()
    return bool(value) and value[0] in "123456789" and value.isdigit()


def compare_ray_outputs(native, actual):
    try:
        refs = block_corpus.parse(native)
        probe = parse_probe(actual)
    except (ValueError, UnicodeError) as error:
        return {"mismatch_count": 1, "first_mismatch": {"record": None, "error": str(error)}}
    names = ("id", "kind", "x", "y", "z", "side", "px", "py", "pz", "px_bits", "py_bits", "pz_bits")
    for i, (reference, observed) in enumerate(zip(refs, probe, strict=True)):
        expected = tuple(reference[key] for key in names)
        got = tuple(observed[key] for key in names)
        if got != expected:
            return {"mismatch_count": 1, "first_mismatch": {
                "record": i, "case_id": reference["id"],
                "reference": dict(zip(names, expected)), "probe": dict(zip(names, got))}}
    return {"mismatch_count": 0, "first_mismatch": None}


def source_snapshot(root):
    paths = {FIXTURE, AUDIT, PROVENANCE, ORACLE, CORPUS, Path("tools/parity/run.py"),
             Path("tools/parity/block_run.py"), Path("tools/parity/block_review.py"),
             Path("config/local.toml"), Path("Cargo.toml"), Path("Cargo.lock"),
             Path("rust-toolchain.toml"), PROBE}
    paths.update(p.relative_to(root) for p in (root / "tools/parity").glob("*.py"))
    paths.update(p.relative_to(root) for p in (root / "tests/fixtures/block").glob("*"))
    return {str(path): sha256((root / path).read_bytes()) if (root / path).is_file() else None
            for path in sorted(paths)}


def _record(directory, report, label, command, timeout, root, input_data=None):
    result = parity.run_process(command, cwd=root, directory=directory, label=label,
                                timeout=timeout, input_data=input_data)
    report["processes"][label] = result
    if result["error"] or result["returncode"] != 0:
        raise BlockRunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
    return Path(result["stdout"]).read_bytes()


def _native_capture(directory, report, run_name, jar, timeout, root):
    classes = directory / run_name / "classes"
    classes.mkdir(parents=True)
    _record(directory, report, run_name + "-javac",
            ["javac", "-cp", str(jar), "-d", str(classes), str(root / ORACLE)],
            min(timeout, 40), root)
    output = _record(directory, report, run_name,
                     ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
                      str(classes) + os.pathsep + str(jar), "BlockOracle", str(jar)], timeout, root)
    stderr = Path(report["processes"][run_name]["stderr"]).read_bytes()
    try:
        audit = json.loads(stderr.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise BlockRunError(f"{run_name}: native audit must be one JSON record") from error
    if audit.get("jar_sha256") != block_corpus.JAR_SHA or audit.get("classes") != block_corpus.CLASS_SHA:
        raise BlockRunError(f"{run_name}: pinned native class/JAR audit mismatch")
    if audit.get("world_constructors") != 0 or audit.get("native_cases") != len(block_corpus.EXPECTED_ORDER):
        raise BlockRunError(f"{run_name}: native world/case audit mismatch")
    inputs = _record(directory, report, run_name + "-inputs",
                     ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
                      str(classes) + os.pathsep + str(jar), "BlockOracle", str(jar), "--inputs"], timeout, root)
    if Path(report["processes"][run_name + "-inputs"]["stderr"]).read_bytes():
        raise BlockRunError(f"{run_name}: native input generator wrote unexpected stderr")
    try:
        block_corpus.parse_inputs(inputs)
    except ValueError as error:
        raise BlockRunError(f"{run_name}: Java emitted invalid ray inputs: {error}") from error
    block_corpus.parse(output)
    saved = directory / f"{run_name}.native.tsv"
    saved.write_bytes(output)
    report["log_paths"][run_name + "_output"] = str(saved)
    adapter = classes / "BlockOracle.class"
    if not adapter.is_file():
        raise BlockRunError("javac did not retain BlockOracle.class")
    return output, inputs, audit, sha256(adapter.read_bytes())


def run_block(root=ROOT, timeout=120):
    parent = parity.artifacts_root(root) / "parity/block"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {
        "schema_version": 1, "suite": "block", "scenario_id": "BLOCK-001-ray-only-v1",
        "repository_root": str(root.resolve()), "started_at": datetime.now(timezone.utc).isoformat(),
        "status": "error", "reason": "comparison not completed", "scope": "ray-only",
        "action_status": "not_implemented", "actions_compared": False,
        "reference_source": "live pinned Beta 1.7.3 client oracle",
        "reference_jar_path": None, "reference_jar_sha256": None,
        "reference_class_sha256": {}, "native_audit": {}, "rust_revision": None,
        "rust_dirty": None, "rust_binary_sha256": None, "source_hashes": {},
        "source_hashes_after": {}, "expected_fixture_sha256": None, "reference_output_sha256": None,
        "probe_input_sha256": None, "probe_output_sha256": None, "case_count": 0,
        "mismatch_count": None, "first_mismatch": None, "duration_ms": None,
        "processes": {}, "log_paths": {"report": str(report_path)}, "platform": platform.platform(),
        "comparison_policy": "exact ordered ray rows; hit side/block coordinates and decimal point plus all raw IEEE-754 binary64 words",
        "limits": {"probe_input_bytes": MAX_INPUT_BYTES, "probe_output_bytes": MAX_OUTPUT_BYTES,
                   "max_cases": len(block_corpus.EXPECTED_ORDER)},
        "protocol_tests": "synthetic malformed/mutation test streams are separate from native live TSV rows",
        "action_limitation": "C01 has no native break/place snapshots; this runner cannot claim action parity.",
    }
    initial = None
    jar = None
    started = time.monotonic()
    try:
        paths = tomllib.loads((root / "config/local.toml").read_text())["paths"]
        jar = (root / paths["client_jar"]).resolve(strict=True)
        if sha256(jar.read_bytes()) != block_corpus.JAR_SHA:
            raise BlockRunError("configured client JAR SHA-256 differs from pinned Beta 1.7.3")
        report.update(reference_jar_path=str(jar), reference_jar_sha256=block_corpus.JAR_SHA)
        initial = source_snapshot(root)
        report["source_hashes"] = initial
        expected = (root / FIXTURE).read_bytes()
        block_corpus.parse(expected)
        expected_input = (root / "tests/fixtures/block/input.tsv").read_bytes()
        block_corpus.parse_inputs(expected_input)
        report["input_fixture_sha256"] = sha256(expected_input)
        report["expected_fixture_sha256"] = sha256(expected)
        (directory / "expected-native.tsv").write_bytes(expected)
        report["log_paths"]["retained_expected"] = str(directory / "expected-native.tsv")
        reference, input_first, audit, adapter_sha = _native_capture(directory, report, "native-run", jar, timeout, root)
        repeat, input_repeat, repeat_audit, repeat_adapter_sha = _native_capture(directory, report, "native-repeat", jar, timeout, root)
        if (reference != repeat or reference != expected or input_first != input_repeat or input_first != expected_input
                or audit != repeat_audit or adapter_sha != repeat_adapter_sha):
            raise BlockRunError("repeat native captures, C01 ray/input fixtures, audit, or adapter hashes differ")
        report.update(reference_output_sha256=sha256(reference), reference_class_sha256=audit["classes"],
                      native_audit={k: v for k, v in audit.items() if k != "classes"} | {"repeatable": True},
                      adapter_class_sha256=adapter_sha, case_count=len(block_corpus.parse(reference)))

        request = probe_input()
        report["probe_input_sha256"] = sha256(request)
        request_path = directory / "probe-input.tsv"
        request_path.write_bytes(request)
        report["log_paths"]["probe_input"] = str(request_path)
        report["log_paths"].update(reference_stdout=report["processes"]["native-run"]["stdout"],
                                   repeat_stdout=report["processes"]["native-repeat"]["stdout"])
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust block_probe source is absent; native C01 ray/input oracle ran twice")
        else:
            report["rust_revision"] = _record(directory, report, "git-revision",
                                                ["git", "rev-parse", "HEAD"], timeout, root).decode().strip()
            dirty = _record(directory, report, "git-dirty", ["git", "status", "--porcelain"], timeout, root)
            report["rust_dirty"] = bool(dirty.strip())
            if report["rust_dirty"]:
                raise BlockRunError("strict block parity requires a committed clean Rust source revision")
            build = _record(directory, report, "rust-build",
                            ["cargo", "build", "--locked", "--offline", "-p", "beta-core",
                             "--example", "block_probe", "--message-format=json-render-diagnostics"], timeout, root)
            binaries = [Path(obj["executable"]) for line in build.splitlines()
                        if (obj := json.loads(line)).get("reason") == "compiler-artifact"
                        and obj.get("target", {}).get("name") == "block_probe" and obj.get("executable")]
            if len(binaries) != 1:
                raise BlockRunError("cargo did not identify exactly one block_probe executable")
            binary = directory / "block_probe"
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            report["log_paths"]["rust_binary"] = str(binary)
            rust = _record(directory, report, "rust-run", [str(binary)], timeout, root, request)
            if Path(report["processes"]["rust-run"]["stderr"]).read_bytes():
                raise BlockRunError("block probe wrote unexpected stderr")
            report["probe_output_sha256"] = sha256(rust)
            report["log_paths"]["probe_output"] = report["processes"]["rust-run"]["stdout"]
            comparison = compare_ray_outputs(reference, rust)
            report.update(comparison,
                          status="pass" if comparison["mismatch_count"] == 0 else "fail",
                          reason="all 17 native ray cases match exactly" if comparison["mismatch_count"] == 0
                          else "native ray targeting mismatch")
        if sha256(jar.read_bytes()) != block_corpus.JAR_SHA:
            raise BlockRunError("client JAR changed during native block run")
    except (parity.RunError, BlockRunError, OSError, ValueError, KeyError, TypeError,
            json.JSONDecodeError, subprocess.SubprocessError, RuntimeError) as error:
        report.update(status="error", reason=str(error))
    finally:
        if initial is not None:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if after != initial:
                report.update(status="error", reason="tracked inputs changed during block parity run")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout", type=float, default=120)
    args = parser.parse_args(argv)
    if not 0 < args.timeout < float("inf"):
        parser.error("--timeout must be finite and positive")
    report, path = run_block(timeout=args.timeout)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return parity.exit_code(report["status"], strict=True)


if __name__ == "__main__":
    raise SystemExit(main())
