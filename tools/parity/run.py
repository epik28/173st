#!/usr/bin/env python3
"""Live RNG-001 comparison. No saved or synthetic reference outputs are used."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path("tests/fixtures/rng/next-int.tsv")
ORACLE = Path("tools/oracle/src/RandomOracle.java")
PROBE = Path("crates/beta-core/examples/rng_probe.rs")


class RunError(Exception):
    pass


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def parse_fixture(data):
    cases, ids = [], set()
    for number, line in enumerate(data.decode("utf-8").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 3 or not re.fullmatch(r"[A-Za-z0-9_-]+", fields[0]):
            raise RunError(f"fixture line {number}: invalid fields/ID")
        case_id, seed, count = fields
        if (case_id in ids or not re.fullmatch(r"[+-]?[0-9]+", seed)
                or not re.fullmatch(r"\+?[0-9]+", count)):
            raise RunError(f"fixture line {number}: duplicate ID or invalid decimal")
        seed, count = int(seed), int(count)
        if not -(2**63) <= seed < 2**63 or not 1 <= count <= 100000:
            raise RunError(f"fixture line {number}: seed/count outside contract")
        ids.add(case_id)
        cases.append({"id": case_id, "seed": seed, "count": count})
    if not cases:
        raise RunError("fixture contains no cases")
    return cases


def parse_extended_fixture(data):
    cases, ids, total = [], set(), 0
    for number, line in enumerate(data.decode("utf-8").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 3 or not re.fullmatch(r"[A-Za-z0-9_-]+", fields[0]) or fields[0] in ids:
            raise RunError(f"fixture line {number}: invalid fields/ID")
        case_id, seed, tokens = fields
        if not re.fullmatch(r"[+-]?[0-9]+", seed) or not -(2**63) <= int(seed) < 2**63:
            raise RunError("invalid seed")
        operations = []
        for token in tokens.split(";"):
            parts = token.split(":")
            if len(parts) == 1 and token in {"int", "long", "boolean", "float", "double"}:
                operations.append((token, None))
            elif len(parts) == 2 and parts[0] in {"bound", "seed"}:
                if not re.fullmatch(r"[+-]?[0-9]+", parts[1]):
                    raise RunError("invalid operation argument")
                value = int(parts[1])
                bits = 32 if parts[0] == "bound" else 64
                if not -(2**(bits-1)) <= value < 2**(bits-1):
                    raise RunError("operation argument out of range")
                operations.append((parts[0], value))
            else:
                raise RunError(f"invalid operation: {token}")
        total += len(operations)
        if total > 100000:
            raise RunError("operation limit 100000 exceeded")
        ids.add(case_id)
        cases.append({"id": case_id, "seed": int(seed), "count": len(operations), "operations": operations})
    if not cases:
        raise RunError("fixture contains no cases")
    return cases


def extended_keys(cases):
    return [(case["id"], index, op, arg) for case in cases
            for index, (op, arg) in enumerate(case["operations"])]


def valid_extended_record(line, key):
    case_id, index, operation, argument = key
    prefix = f"{case_id}\t{index}\t{operation}\t".encode("ascii")
    if not line.startswith(prefix) or not line.endswith(b"\n"):
        return False
    value = line[len(prefix):-1]
    if operation == "seed":
        return value == b"ok"
    if operation == "boolean":
        return value in {b"0", b"1"}
    if operation in {"float", "double"}:
        width, maximum = (8, 0x3f7fffff) if operation == "float" else (16, 0x3fefffffffffffff)
        return bool(re.fullmatch(rb"[0-9a-f]{" + str(width).encode() + rb"}", value)) and int(value, 16) <= maximum
    if operation == "bound" and argument <= 0:
        return value == b"invalid_bound"
    if not re.fullmatch(rb"(?:0|-[1-9][0-9]*|[1-9][0-9]*)", value):
        return False
    number = int(value)
    if operation == "bound":
        return 0 <= number < argument
    bits = 64 if operation == "long" else 32
    return -(2**(bits-1)) <= number < 2**(bits-1)


def expected_keys(cases):
    return [(case["id"], index) for case in cases for index in range(case["count"])]


def valid_record(line, key):
    if key is None:
        return False
    if len(key) == 4:
        return valid_extended_record(line, key)
    prefix = f"{key[0]}\t{key[1]}\t".encode("ascii")
    if not line.startswith(prefix) or not line.endswith(b"\n"):
        return False
    value = line[len(prefix):-1]
    if not re.fullmatch(rb"(?:0|-[1-9][0-9]*|[1-9][0-9]*)", value):
        return False
    return -(2**31) <= int(value) < 2**31


def compare_outputs(reference, actual, keys):
    """Require valid, complete ordered records even if both outputs are empty."""
    left, right = reference.splitlines(keepends=True), actual.splitlines(keepends=True)
    count, first = 0, None
    for index in range(max(len(keys), len(left), len(right))):
        expected = left[index] if index < len(left) else None
        observed = right[index] if index < len(right) else None
        key = keys[index] if index < len(keys) else None
        if (expected != observed or not valid_record(expected or b"", key)
                or not valid_record(observed or b"", key)):
            count += 1
            if first is None:
                first = {"record": index, "expected_key": list(key) if key else None,
                         "reference": None if expected is None else expected.decode("utf-8", "replace"),
                         "rust": None if observed is None else observed.decode("utf-8", "replace")}
    return {"mismatch_count": count, "first_mismatch": first}


def run_process(command, *, cwd, directory, label, timeout, input_data=None):
    """Keep raw bytes, diagnostics and execution status even on failure/timeout."""
    stdout_path, stderr_path = directory / f"{label}.stdout", directory / f"{label}.stderr"
    started = time.monotonic()
    result = {"command": [str(part) for part in command], "cwd": str(cwd),
              "stdout": str(stdout_path), "stderr": str(stderr_path),
              "returncode": None, "timeout": False, "error": None}
    try:
        completed = subprocess.run(command, cwd=cwd, input=input_data,
                                   capture_output=True, timeout=timeout, check=False)
        stdout, stderr = completed.stdout, completed.stderr
        result["returncode"] = completed.returncode
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b"", error.stderr or b""
        result["timeout"] = True
        result["error"] = f"timeout after {timeout}s"
    except OSError as error:
        stdout, stderr = b"", str(error).encode()
        result["error"] = str(error)
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    result["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
    return result


def source_snapshot(root):
    paths = {FIXTURE, ORACLE, Path("tools/parity/run.py"), Path("Cargo.toml"),
             Path("Cargo.lock"), Path("rust-toolchain.toml")}
    for folder, pattern in (("tools/oracle", "*.java"), ("tools/parity", "*.py"), ("tests/fixtures/rng", "*.tsv")):
        paths.update(path.relative_to(root) for path in (root / folder).rglob(pattern))
    paths.update(path.relative_to(root) for path in (root / "crates").rglob("*.rs"))
    paths.update(path.relative_to(root) for path in (root / "crates").rglob("Cargo.toml"))
    return {str(path): sha256((root / path).read_bytes()) if (root / path).is_file() else None
            for path in sorted(paths)}


def artifacts_root(root):
    local = root / "config/local.toml"
    if local.exists():
        value = tomllib.loads(local.read_text()).get("paths", {}).get("artifacts", "artifacts")
        return root / value
    return root / "artifacts"


def exit_code(status, strict):
    return int(status in {"fail", "error"} or (strict and status != "pass"))


def run_rng(root=ROOT, timeout=60, suite="rng"):
    extended = suite == "rng-extended"
    fixture = Path("tests/fixtures/rng/extended.tsv") if extended else FIXTURE
    class_name = "RandomExtendedOracle" if extended else "RandomOracle"
    oracle = Path(f"tools/oracle/src/{class_name}.java") if extended else ORACLE
    example = "rng_extended_probe" if extended else "rng_probe"
    probe = Path(f"crates/beta-core/examples/{example}.rs") if extended else PROBE
    started = time.monotonic()
    parent = artifacts_root(root) / f"parity/{suite}"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {
        "schema_version": 1, "suite": suite, "scenario_id": "RNG-002-mixed" if extended else "RNG-001-nextInt",
        "started_at": datetime.now(timezone.utc).isoformat(),
        "status": "error", "reason": "comparison not completed",
        "scope": "Java RNG API: mixed methods and shared state; no Beta game integration" if extended else "Java RNG API: fresh java.util.Random(seed).nextInt(); no Beta game integration",
        "reference_side": "Java API", "reference_version": None,
        "reference_jar_sha256": "not_applicable: java.base API; no Minecraft JAR",
        "runtime_versions": {}, "platform": platform.platform(), "toolchain": {},
        "adapter_sha256": None, "adapter_class_sha256": None,
        "rust_revision": None, "rust_dirty": None, "rust_binary_sha256": None,
        "source_hashes": {}, "source_hashes_after": {}, "input_sha256": None,
        "fixture_origin": f"repository {fixture}; fixed inputs, no expected outputs",
        "seed": [], "event_schedule": "case order in saved input; fresh RNG per case; sequential operations and explicit reseeds",
        "comparison_policy": "exact ordered LF TSV; typed integers/raw IEEE bits; no tolerance or normalization",
        "case_count": 0, "value_count": 0, "mismatch_count": None, "first_mismatch": None,
        "duration_ms": None, "reference_output_sha256": None, "rust_output_sha256": None,
        "log_paths": {"input": str(directory / "input.tsv"), "report": str(report_path)},
        "processes": {},
    }

    def execute(label, command, input_data=None):
        result = run_process(command, cwd=root, directory=directory, label=label,
                             timeout=timeout, input_data=input_data)
        report["processes"][label] = result
        report["log_paths"][label] = {key: result[key] for key in ("stdout", "stderr")}
        if result["error"] or result["returncode"] != 0:
            raise RunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
        return Path(result["stdout"]).read_bytes()

    before = None
    try:
        before = source_snapshot(root)
        report["source_hashes"] = before
        report["adapter_sha256"] = before[str(oracle)]
        data = (root / fixture).read_bytes()
        (directory / "input.tsv").write_bytes(data)
        report["input_sha256"] = sha256(data)
        cases = parse_extended_fixture(data) if extended else parse_fixture(data)
        keys = extended_keys(cases) if extended else expected_keys(cases)
        report.update(case_count=len(cases), value_count=len(keys), seed=cases)
        report["rust_revision"] = execute("git-revision", ["git", "rev-parse", "HEAD"]).decode().strip()
        report["rust_dirty"] = bool(execute("git-dirty", ["git", "status", "--porcelain"]))
        for tool in ("java", "javac", "rustc", "cargo"):
            execute(f"version-{tool}", [tool, "-version" if tool in {"java", "javac"} else "--version"])
            process = report["processes"][f"version-{tool}"]
            version = (Path(process["stdout"]).read_bytes() + Path(process["stderr"]).read_bytes()).decode().strip()
            report["runtime_versions"][tool] = version
        report["reference_version"] = report["runtime_versions"]["java"]
        report["toolchain"] = {tool: report["runtime_versions"][tool] for tool in ("rustc", "cargo")}
        classes = directory / "classes"
        classes.mkdir()
        execute("java-build", ["javac", "-d", str(classes), str(root / oracle)])
        report["adapter_class_sha256"] = sha256((classes / f"{class_name}.class").read_bytes())
        reference = execute("java-run", ["java", "-cp", str(classes), class_name], data)
        report["reference_output_sha256"] = sha256(reference)
        if compare_outputs(reference, reference, keys)["mismatch_count"]:
            raise RunError("Java oracle output violates scenario protocol")
        if not (root / probe).is_file():
            report.update(status="not_implemented", reason=f"Rust {example} source is absent; Java oracle executed")
        else:
            build = execute("rust-build", ["cargo", "build", "--locked", "--offline", "-p", "beta-core",
                                            "--example", example, "--message-format=json-render-diagnostics"])
            binaries = []
            for line in build.splitlines():
                message = json.loads(line)
                if (message.get("reason") == "compiler-artifact"
                        and message.get("target", {}).get("name") == example
                        and message.get("executable")):
                    binaries.append(Path(message["executable"]))
            if len(binaries) != 1:
                raise RunError(f"cargo did not identify exactly one {example} executable")
            binary = directory / example
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            actual = execute("rust-run", [str(binary)], data)
            report["rust_output_sha256"] = sha256(actual)
            comparison = compare_outputs(reference, actual, keys)
            report.update(comparison)
            report.update(status="fail" if comparison["mismatch_count"] else "pass",
                          reason="output mismatch" if comparison["mismatch_count"] else "all records match exactly")
    except (RunError, OSError, ValueError) as error:
        report.update(status="error", reason=str(error))
    finally:
        try:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if before is not None and before != after:
                report.update(status="error", reason="inputs/sources changed during comparison")
            saved = directory / "input.tsv"
            if saved.exists() and sha256(saved.read_bytes()) != report["input_sha256"]:
                report.update(status="error", reason="saved input changed during comparison")
        except OSError as error:
            report.update(status="error", reason=f"cannot verify sources after execution: {error}")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("group", choices=("list", "rng", "rng-extended"))
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--timeout", type=float, default=60, help="per-process timeout in seconds")
    args = parser.parse_args(argv)
    if not 0 < args.timeout < float("inf"):
        parser.error("--timeout must be finite and positive")
    if args.group == "list":
        print("rng: RNG-001 nextInt; requires java, javac, rustc, cargo and rng_probe")
        print("rng-extended: RNG-002 mixed Random methods; requires Java and rng_extended_probe")
        return 0
    try:
        report, path = run_rng(timeout=args.timeout, suite=args.group)
    except (OSError, ValueError) as error:
        print(f"error: {error}")
        return 1
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return exit_code(report["status"], args.strict)


if __name__ == "__main__":
    raise SystemExit(main())
