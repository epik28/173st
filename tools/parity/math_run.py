"""Run live Beta 1.7.3 MathHelper ↔ Rust differential comparisons."""
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

from run import ROOT, artifacts_root, run_process, sha256

JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASS_SHA = "8693f3179fb0ff1df382e1d3388d23ab688ad3e26415454655da88f10b054f63"
FIXTURE = Path("tests/fixtures/math/input.tsv")
REFERENCE_FIXTURE = Path("tests/fixtures/math/reference.tsv")
ORACLE = Path("tools/oracle/src/MathOracle.java")
PROBE = Path("crates/beta-core/examples/math_probe.rs")
EXAMPLE = "math_probe"
FLOAT_OPS = {"sin", "cos", "sqrt_f32", "sqrt_f64_to_f32", "abs_f32"}
INT_OPS = {"floor_f32", "floor_f64", "floor_div_i32"}
OPS = FLOAT_OPS | INT_OPS | {"abs_max_f64", "null_or_empty"}


class MathRunError(Exception):
    pass


def _strict_ascii(data, label):
    try:
        return data.decode("ascii")
    except UnicodeDecodeError as error:
        raise MathRunError(f"{label} is not ASCII") from error


def parse_math_input(data):
    text = _strict_ascii(data, "math input")
    if not text.endswith("\n"):
        raise MathRunError("math input requires final LF")
    lines = text[:-1].split("\n")
    cases, seen = [], set()
    for number, line in enumerate(lines, 1):
        fields = line.split("\t")
        if len(fields) < 3 or not re.fullmatch(r"[a-zA-Z0-9_.:-]{1,96}", fields[0]):
            raise MathRunError(f"bad math input line {number}")
        case_id, op, *args = fields
        if case_id in seen or op not in OPS:
            raise MathRunError(f"duplicate ID or unknown operation at line {number}")
        seen.add(case_id)
        arity = 2 if op == "abs_max_f64" or op == "floor_div_i32" else 1
        if len(args) != arity:
            raise MathRunError(f"wrong arity at line {number}")
        if op in {"sin", "cos", "sqrt_f32", "floor_f32", "abs_f32"}:
            if not re.fullmatch(r"[0-9a-f]{8}", args[0]):
                raise MathRunError(f"bad f32 bits at line {number}")
        if op in {"sqrt_f64_to_f32", "floor_f64"}:
            if not re.fullmatch(r"[0-9a-f]{16}", args[0]):
                raise MathRunError(f"bad f64 bits at line {number}")
        if op == "abs_max_f64":
            if any(not re.fullmatch(r"[0-9a-f]{16}", arg) for arg in args):
                raise MathRunError(f"bad f64 bits at line {number}")
        if op == "floor_div_i32":
            for arg in args:
                if not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", arg) or not -(1 << 31) <= int(arg) < (1 << 31):
                    raise MathRunError(f"bad i32 at line {number}")
        if op == "null_or_empty":
            if args[0] != "null" and (len(args[0]) % 2 or not re.fullmatch(r"[0-9a-f]*", args[0])):
                raise MathRunError(f"bad UTF-8 hex at line {number}")
            if args[0] != "null":
                try:
                    bytes.fromhex(args[0]).decode("utf-8")
                except UnicodeDecodeError as error:
                    raise MathRunError(f"invalid UTF-8 at line {number}") from error
        cases.append({"id": case_id, "op": op, "args": args})
    if not cases or len(cases) > 200000:
        raise MathRunError(f"math input row count outside limit: {len(cases)}")
    return cases


def _valid_value(op, value):
    if op in FLOAT_OPS:
        return re.fullmatch(r"[0-9a-f]{8}", value) is not None
    if op in INT_OPS:
        if op == "floor_div_i32" and value == "error_arithmetic":
            return True
        return re.fullmatch(r"-?(?:0|[1-9][0-9]*)", value) is not None and -(1 << 31) <= int(value) < (1 << 31)
    if op == "abs_max_f64":
        return re.fullmatch(r"[0-9a-f]{16}", value) is not None
    return value in {"0", "1"}


def parse_math_output(data, cases, label):
    text = _strict_ascii(data, label)
    if not text.endswith("\n"):
        raise MathRunError(f"{label} requires final LF")
    lines = text[:-1].split("\n")
    if len(lines) != len(cases):
        raise MathRunError(f"{label} row count mismatch: expected {len(cases)}, got {len(lines)}")
    parsed = []
    for index, (line, case) in enumerate(zip(lines, cases)):
        fields = line.split("\t")
        if len(fields) != 2 or fields[0] != case["id"] or not _valid_value(case["op"], fields[1]):
            raise MathRunError(f"{label} malformed or out-of-order row {index} ({case['id']})")
        parsed.append(fields[1])
    return parsed


def compare_math_outputs(reference, actual, cases):
    left = parse_math_output(reference, cases, "original output")
    right = parse_math_output(actual, cases, "Rust output")
    first = None
    mismatch_count = 0
    for index, (expected, observed) in enumerate(zip(left, right)):
        if expected != observed:
            mismatch_count += 1
            if first is None:
                first = {"record": index, "case": cases[index]["id"], "operation": cases[index]["op"],
                         "reference": expected, "rust": observed}
    return {"mismatch_count": mismatch_count, "first_mismatch": first}


def source_snapshot(root=ROOT):
    fixed = {Path("Cargo.toml"), Path("Cargo.lock"), Path("rust-toolchain.toml"),
             Path("config/local.toml"), FIXTURE, REFERENCE_FIXTURE,
             Path("tests/fixtures/math/audit.json"), ORACLE,
             Path("tests/python/test_math_runner.py"),
             Path("tools/oracle/math_corpus.py"), Path("tools/parity/run.py"),
             Path("tools/parity/math_run.py"), Path("tools/parity/verify_report.py"),
             PROBE, Path("crates/beta-core/src/math.rs"), Path(".cargo/config"), Path(".cargo/config.toml")}
    paths = set(fixed)
    paths.update(path.relative_to(root) for path in (root / "crates/beta-core").rglob("Cargo.toml"))
    for folder in (root / "tools/parity", root / "tools/oracle", root / "crates/beta-core"):
        if folder.exists():
            paths.update(path.relative_to(root) for path in folder.rglob("*.py" if "parity" in str(folder) else ("*.java" if "oracle" in str(folder) else "*.rs")))
    return {str(path): sha256((root / path).read_bytes()) if (root / path).is_file() else None
            for path in sorted(paths)}


def run_math(root=ROOT, timeout=60):
    started = time.monotonic()
    parent = artifacts_root(root) / "parity/math"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {
        "schema_version": 1, "suite": "math", "scenario_id": "MATH-001-all-functions",
        "started_at": datetime.now(timezone.utc).isoformat(), "status": "error",
        "reason": "comparison not completed", "scope": "Native Beta 1.7.3 MathHelper class in; no player/game-loop integration",
        "reference_side": "Original Beta 1.7.3 client MathHelper class in", "reference_version": None,
        "reference_jar_sha256": JAR_SHA, "reference_jar_path": None, "reference_class_sha256": CLASS_SHA,
        "runtime_versions": {}, "platform": platform.platform(), "toolchain": {},
        "adapter_sha256": None, "adapter_class_sha256": None,
        "rust_revision": None, "rust_dirty": None, "rust_binary_sha256": None,
        "source_hashes": {}, "source_hashes_after": {}, "input_sha256": None,
        "fixture_origin": "committed tests/fixtures/math/input.tsv; outputs generated by the original JAR",
        "case_count": 0, "operation_counts": {}, "lut_coverage": {"sin": 65536, "cos": 65536},
        "mismatch_count": None, "first_mismatch": None, "duration_ms": None,
        "reference_output_sha256": None, "reference_repeat_output_sha256": None,
        "rust_output_sha256": None,
        "comparison_policy": "exact ordered LF TSV; float and double results use raw IEEE-754 bits",
        "log_paths": {"input": str(directory / "input.tsv"), "report": str(report_path)},
        "processes": {},
    }
    jar = None
    before = None
    input_data = None

    def execute(label, command, data=None):
        result = run_process(command, cwd=root, directory=directory, label=label,
                             timeout=timeout, input_data=data)
        report["processes"][label] = result
        report["log_paths"][label] = {key: result[key] for key in ("stdout", "stderr")}
        if result["error"] or result["returncode"] != 0:
            raise MathRunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
        return Path(result["stdout"]).read_bytes()

    try:
        cfg = tomllib.loads((root / "config/local.toml").read_text()).get("paths", {})
        if not cfg.get("client_jar"):
            raise MathRunError("paths.client_jar is not configured")
        jar = (root / cfg["client_jar"]).resolve(strict=True)
        if sha256(jar.read_bytes()) != JAR_SHA:
            raise MathRunError("unexpected client JAR SHA-256")
        with __import__("zipfile").ZipFile(jar) as archive:
            class_bytes = archive.read("in.class")
        if sha256(class_bytes) != CLASS_SHA:
            raise MathRunError("original MathHelper class hash mismatch")
        before = source_snapshot(root)
        report["source_hashes"] = before
        report["adapter_sha256"] = before.get(str(ORACLE))
        input_data = (root / FIXTURE).read_bytes()
        (directory / "input.tsv").write_bytes(input_data)
        report["input_sha256"] = sha256(input_data)
        cases = parse_math_input(input_data)
        if len(cases) != 131408:
            raise MathRunError(f"committed math corpus row count mismatch: {len(cases)}")
        report["case_count"] = len(cases)
        for case in cases:
            report["operation_counts"][case["op"]] = report["operation_counts"].get(case["op"], 0) + 1
        report["reference_invocations"] = dict(report["operation_counts"])
        report["rust_revision"] = execute("git-revision", ["git", "rev-parse", "HEAD"]).decode().strip()
        report["rust_dirty"] = bool(execute("git-dirty", ["git", "status", "--porcelain"]))
        for tool, args in (("java", ["java", "-version"]), ("javac", ["javac", "-version"]),
                           ("rustc", ["rustc", "--version"]), ("cargo", ["cargo", "--version"])):
            execute(f"version-{tool}", args)
            item = report["processes"][f"version-{tool}"]
            report["runtime_versions"][tool] = (Path(item["stdout"]).read_bytes() + Path(item["stderr"]).read_bytes()).decode().strip()
        report["reference_version"] = "Minecraft Beta 1.7.3 client; " + report["runtime_versions"]["java"]
        report["toolchain"] = {key: report["runtime_versions"][key] for key in ("rustc", "cargo")}
        classes = directory / "classes"
        classes.mkdir()
        execute("java-build", ["javac", "-cp", str(jar), "-d", str(classes), str(root / ORACLE)])
        report["adapter_class_sha256"] = sha256((classes / "MathOracle.class").read_bytes())
        (directory / "classes.path").write_text(str(classes) + "\n")
        report["log_paths"]["java_classes"] = str(classes)
        java = ["java", "-cp", f"{classes}:{jar}", "MathOracle", str(jar)]
        reference = execute("java-run", java, input_data)
        repeated = execute("java-repeat", java, input_data)
        if reference != repeated:
            raise MathRunError("original MathHelper produced different repeated outputs")
        expected_fixture = (root / REFERENCE_FIXTURE).read_bytes()
        if reference != expected_fixture:
            raise MathRunError("live original output differs from committed reference fixture")
        parse_math_output(reference, cases, "original output")
        report["reference_output_sha256"] = sha256(reference)
        report["reference_repeat_output_sha256"] = sha256(repeated)
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason=f"Rust {EXAMPLE} source is absent; original oracle executed")
        else:
            build = execute("rust-build", ["cargo", "build", "--locked", "--offline", "-p", "beta-core",
                                            "--example", EXAMPLE, "--message-format=json-render-diagnostics"])
            executables = []
            for line in build.splitlines():
                obj = json.loads(line)
                if obj.get("reason") == "compiler-artifact" and obj.get("target", {}).get("name") == EXAMPLE and obj.get("executable"):
                    executables.append(Path(obj["executable"]))
            if len(executables) != 1:
                raise MathRunError(f"Cargo did not identify exactly one {EXAMPLE} binary")
            binary = directory / EXAMPLE
            shutil.copy2(executables[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            report["log_paths"]["rust_binary"] = str(binary)
            actual = execute("rust-run", [str(binary)], input_data)
            report["rust_output_sha256"] = sha256(actual)
            comparison = compare_math_outputs(reference, actual, cases)
            report.update(comparison)
            report.update(status="fail" if comparison["mismatch_count"] else "pass",
                          reason="output mismatch" if comparison["mismatch_count"] else "all records match exactly")
    except (MathRunError, OSError, ValueError, KeyError) as error:
        report.update(status="error", reason=str(error))
    finally:
        try:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if jar is not None and sha256(jar.read_bytes()) != JAR_SHA:
                report.update(status="error", reason="original JAR changed during comparison")
            if before is not None and before != after:
                report.update(status="error", reason="source files changed during comparison")
            saved_input = directory / "input.tsv"
            if saved_input.exists() and sha256(saved_input.read_bytes()) != report["input_sha256"]:
                report.update(status="error", reason="saved input changed during comparison")
        except (OSError, ValueError) as error:
            report.update(status="error", reason=f"cannot verify math sources after run: {error}")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path
