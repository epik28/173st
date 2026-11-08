"""Retain strict original-JAR evidence for Beta 1.7.3 saved-light parity."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import shutil
import struct
import tempfile
import time
import tomllib
from urllib.parse import unquote, urlparse
import zipfile

from run import ROOT, artifacts_root, exit_code, run_process, sha256

JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
FIXTURES = Path("tests/fixtures/light")
ORACLE = Path("tools/oracle/src/LightOracle.java")
CORPUS = Path("tools/oracle/light_corpus.py")
PROBE = Path("crates/beta-core/examples/light_probe.rs")
EXAMPLE = "light_probe"
REQUIRED = ("git-revision", "git-dirty", "version-java", "version-javac", "version-rustc",
            "version-cargo", "java-build", "java-run", "java-repeat", "rust-build", "rust-run")


class LightError(ValueError):
    """Malformed LIGHT input, output, audit or retained execution evidence."""


def require_int_counts(actual, expected, label):
    if (not isinstance(actual, dict) or set(actual) != set(expected)
            or any(type(value) is not int for value in actual.values()) or actual != expected):
        raise LightError(label + " must contain exactly the expected integer counts")


def validate_report_counts(report, cases):
    from collections import Counter
    operations = dict(Counter(case["op"] for case in cases))
    if type(report.get("case_count")) is not int or report["case_count"] != len(cases):
        raise LightError("case count differs or is not a JSON integer")
    require_int_counts(report.get("operation_counts"), operations, "operation_counts")


def decode_lines(data, label):
    if not data or len(data) > 16 * 1024 * 1024:
        raise LightError(f"{label} must be nonempty and <=16MiB")
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise LightError(f"{label} must be ASCII") from error
    if "\r" in text or not text.endswith("\n"):
        raise LightError(f"{label} requires LF-only rows and final LF")
    rows = text[:-1].split("\n")
    if len(rows) > 100_000 or any(not row for row in rows):
        raise LightError(f"{label} has blank rows or exceeds 100000 rows")
    return rows


def canonical_int(text, low, high):
    if not re.fullmatch(r"(?:0|-?[1-9][0-9]*)", text):
        raise LightError("integer is not canonical signed decimal")
    try:
        value = int(text)
    except ValueError as error:
        raise LightError("invalid integer") from error
    if not low <= value <= high:
        raise LightError("integer outside contract domain")
    return value


def dimension(text):
    value = canonical_int(text, -1, 0)
    if value not in (-1, 0):
        raise LightError("dimension must be 0 or -1")
    return value


def raw_f32(bits, *, partial=False):
    if not re.fullmatch(r"[0-9a-f]{8}", bits):
        raise LightError("expected lowercase raw f32 hex8")
    value = struct.unpack(">f", bytes.fromhex(bits))[0]
    if not math.isfinite(value):
        raise LightError("raw f32 must be finite")
    if partial and not 0.0 <= value <= 1.0:
        raise LightError("partial must be in [0,1]")
    return value


def parse_input(data):
    result, seen = [], set()
    for number, line in enumerate(decode_lines(data, "input"), 1):
        fields = line.split("\t")
        if (len(fields) < 2 or not re.fullmatch(r"[A-Za-z0-9_.:-]{1,96}", fields[0])
                or fields[0] in seen):
            raise LightError(f"row {number}: invalid or duplicate ID")
        seen.add(fields[0])
        op = fields[1]
        if op == "sample" and len(fields) == 7:
            row = {"id": fields[0], "op": op, "dimension": dimension(fields[2]),
                   "block": canonical_int(fields[3], 0, 15), "sky": canonical_int(fields[4], 0, 15),
                   "subtraction": canonical_int(fields[5], 0, 15), "minimum": canonical_int(fields[6], 0, 15)}
        elif op == "time" and len(fields) == 5:
            raw_f32(fields[4], partial=True)
            row = {"id": fields[0], "op": op, "dimension": dimension(fields[2]),
                   "time": canonical_int(fields[3], -(1 << 63), (1 << 63) - 1), "partial": fields[4]}
        elif op == "emission" and len(fields) == 3:
            row = {"id": fields[0], "op": op, "block_id": canonical_int(fields[2], 0, 255)}
        else:
            raise LightError(f"row {number}: unknown operation or wrong field count")
        result.append(row)
    return result


def parse_output(data, cases, label="output"):
    result = []
    lines = decode_lines(data, label)
    if len(lines) != len(cases):
        raise LightError(f"{label} record count mismatch")
    for number, (line, case) in enumerate(zip(lines, cases), 1):
        fields = line.split("\t")
        if len(fields) < 2 or fields[:2] != [case["id"], case["op"]]:
            raise LightError(f"{label} row {number}: ID/operation mismatch")
        if case["op"] == "sample" and len(fields) == 4:
            canonical_int(fields[2], 0, 15)
            raw_f32(fields[3])
            parsed = fields[2:]
        elif case["op"] == "time" and len(fields) == 4:
            raw_f32(fields[2])  # Angle may be outside [0,1] for negative time; only finiteness is constrained.
            canonical_int(fields[3], 0, 11)
            parsed = fields[2:]
        elif case["op"] == "emission" and len(fields) == 3:
            canonical_int(fields[2], 0, 15)
            parsed = fields[2:]
        else:
            raise LightError(f"{label} row {number}: invalid output arity or value")
        result.append([case["id"], case["op"], *parsed])
    if len(result) != len(cases):
        raise LightError(f"{label} record count mismatch")
    return result


def compare(reference, actual, cases):
    expected, observed = parse_output(reference, cases, "native output"), parse_output(actual, cases, "Rust output")
    mismatches, first = 0, None
    for index, (left, right) in enumerate(zip(expected, observed)):
        if left != right:
            mismatches += 1
            if first is None:
                field = next(i for i, (a, b) in enumerate(zip(left[2:], right[2:])) if a != b)
                first = {"record": index, "case": left[0], "operation": left[1],
                         "component": field, "reference": left[field + 2], "rust": right[field + 2]}
    return {"mismatch_count": mismatches, "first_mismatch": first}


def strict_json(data, label):
    def pairs(items):
        out = {}
        for key, value in items:
            if key in out:
                raise LightError(f"{label}: duplicate JSON key {key}")
            out[key] = value
        return out
    try:
        value = json.loads(data, object_pairs_hook=pairs)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise LightError(f"{label}: invalid JSON") from error
    if not isinstance(value, dict):
        raise LightError(f"{label}: JSON object required")
    return value


def native_hashes(jar, names):
    with zipfile.ZipFile(jar) as archive:
        return {name: sha256(archive.read(name + ".class")) for name in names}


def validate_audit(audit, cases, jar, pinned, input_bytes=None, output_bytes=None):
    static_keys = ("jar_sha256", "classes", "bootstrap_base64", "bootstrap_sha256", "internal_calls",
                   "weather_state", "provider_dimensions", "special_light_query_ids_excluded", "light_used_static_flag")
    for key in static_keys:
        if audit.get(key) != pinned.get(key):
            raise LightError("native pinned audit field differs: " + key)
    if audit.get("jar_sha256") != JAR_SHA or sha256(jar.read_bytes()) != JAR_SHA:
        raise LightError("native JAR hash differs")
    classes = audit.get("classes")
    if not isinstance(classes, dict) or not classes or classes != native_hashes(jar, classes):
        raise LightError("native class hashes differ from original JAR")
    if set(classes) != set(pinned.get("classes", {})):
        raise LightError("native class inventory differs from pinned audit")
    source = audit.get("code_source")
    if not isinstance(source, str) or Path(source).resolve() != jar.resolve():
        raise LightError("native code source differs from configured client JAR")
    import base64
    try:
        bootstrap = base64.b64decode(audit["bootstrap_base64"], validate=True)
    except (KeyError, ValueError) as error:
        raise LightError("native bootstrap capture is malformed") from error
    if sha256(bootstrap) != audit.get("bootstrap_sha256"):
        raise LightError("native bootstrap hash differs")
    if input_bytes is not None and audit.get("input_sha256") != sha256(input_bytes):
        raise LightError("native audit input hash differs")
    if output_bytes is not None and audit.get("output_sha256") != sha256(output_bytes):
        raise LightError("native audit output hash differs")
    direct = audit.get("direct_counts")
    sample_count = sum(case["op"] == "sample" for case in cases)
    time_count = sum(case["op"] == "time" for case in cases)
    emission_count = sum(case["op"] == "emission" for case in cases)
    worlds = {case["dimension"] for case in cases if case["op"] != "emission"}
    expected = {"lm_c_explicit": sample_count, "fd_a_IIII": sample_count,
                "xa_a_JF": time_count, "fd_a_F": time_count,
                "xa_factory": len(worlds), "xa_e": len(worlds), "ei_a_J": time_count,
                "uu_s_reads": emission_count}
    require_int_counts(direct, expected, "native direct_counts")
    constructors = audit.get("constructors")
    require_int_counts(constructors, {"fd": 0, "synthetic_fd": 0, "lm": len(worlds)}, "native constructors")


def validate_pinned(root, pinned):
    jar = jar_path(root)
    expected = {"jar_sha256": JAR_SHA,
                "input_sha256": sha256((root / FIXTURES / "input.tsv").read_bytes()),
                "output_sha256": sha256((root / FIXTURES / "reference.tsv").read_bytes())}
    if any(pinned.get(key) != value for key, value in expected.items()):
        raise LightError("committed native fixture provenance hash mismatch")
    if pinned.get("code_source") != "<configured-original-jar>":
        raise LightError("fixture code-source placeholder differs")
    if pinned.get("classes") != native_hashes(jar, pinned.get("classes", {})):
        raise LightError("committed class hashes differ from configured JAR")
    counts = pinned.get("direct_counts", {})
    require_int_counts(counts, {"lm_c_explicit": 24608, "fd_a_IIII": 24608, "xa_a_JF": 48064,
                                "fd_a_F": 48064, "xa_factory": 2, "xa_e": 2, "ei_a_J": 48064,
                                "uu_s_reads": 256}, "pinned direct_counts")


def source_snapshot(root=ROOT):
    paths = {Path("Cargo.toml"), Path("Cargo.lock"), Path("rust-toolchain.toml"),
             Path(".cargo/config"), Path(".cargo/config.toml"), Path("config/local.toml"),
             Path("config/local.example.toml"),
             Path("tools/parity/geometry_run.py"),
             ORACLE, CORPUS, PROBE, Path("tools/parity/run.py"), Path("tools/parity/verify_report.py"),
             Path("tools/parity/light_run.py"), Path("tools/parity/light_review.py"),
             Path("tests/python/test_light_runner.py")}
    for folder, patterns in (("crates/beta-core", ("*.rs", "Cargo.toml", "build.rs")),
                             (str(FIXTURES), ("*",))):
        for pattern in patterns:
            paths.update(p.relative_to(root) for p in (root / folder).rglob(pattern) if p.is_file())
    return {str(p): sha256((root / p).read_bytes()) if (root / p).is_file() else None for p in sorted(paths)}


def jar_path(root):
    from geometry_run import jar_path as configured_jar
    return configured_jar(root)


def expected_commands(root, directory, jar):
    classes = directory / "classes"
    commands = {"git-revision": ["git", "rev-parse", "HEAD"],
                "git-dirty": ["git", "status", "--porcelain"],
                "java-build": ["javac", "-encoding", "UTF-8", "-cp", str(jar), "-d", str(classes), str(root / ORACLE)],
                "rust-build": ["cargo", "build", "--locked", "--offline", "-p", "beta-core", "--example", EXAMPLE,
                               "--message-format=json-render-diagnostics"],
                "rust-run": [str(directory / EXAMPLE)]}
    for tool in ("java", "javac", "rustc", "cargo"):
        commands["version-" + tool] = [tool, "-version" if tool in {"java", "javac"} else "--version"]
    for label in ("java-run", "java-repeat"):
        commands[label] = ["java", "-XX:-UsePerfData", "-cp", f"{classes}:{jar}", "LightOracle", str(jar),
                           str(directory / (label + "-audit.json"))]
    return commands


def run_light_suite(root=ROOT, timeout=90, input_path=None):
    root = Path(root).resolve()
    started = time.monotonic()
    parent = artifacts_root(root) / "parity/light"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {"schema_version": 1, "suite": "light", "status": "error", "reason": "comparison not completed",
              "scenario": "custom" if input_path else "pinned", "processes": {}, "log_paths": {},
              "mismatch_count": None, "first_mismatch": None,
              "comparison_policy": "ordered TSV: exact integers and raw f32 bits",
              "started_at": datetime.now(timezone.utc).isoformat()}
    before = source_snapshot(root)
    report["source_hashes"] = before

    def execute(label, command, data=None):
        proc = run_process(command, cwd=root, directory=directory, label=label, timeout=timeout, input_data=data)
        report["processes"][label] = proc
        report["log_paths"][label] = {key: proc[key] for key in ("stdout", "stderr")}
        if proc["error"] or proc["returncode"] != 0:
            raise LightError(f"{label}: {proc['error'] or 'exit ' + str(proc['returncode'])}")
        return Path(proc["stdout"]).read_bytes()

    try:
        jar = jar_path(root)
        if sha256(jar.read_bytes()) != JAR_SHA:
            raise LightError("configured client JAR SHA-256 differs")
        report.update(jar_path=str(jar), jar_sha256=JAR_SHA)
        native_names = list(strict_json((root / FIXTURES / "audit.json").read_bytes(), "pinned audit")["classes"])
        report["native_class_sha256"] = native_hashes(jar, native_names)
        pinned = strict_json((root / FIXTURES / "audit.json").read_bytes(), "pinned audit")
        validate_pinned(root, pinned)
        raw = Path(input_path).read_bytes() if input_path else (root / FIXTURES / "input.tsv").read_bytes()
        cases = parse_input(raw)
        (directory / "input.tsv").write_bytes(raw)
        report["log_paths"]["input"] = str(directory / "input.tsv")
        report.update(input_sha256=sha256(raw), case_count=len(cases),
                      operation_counts=dict(Counter(case["op"] for case in cases)))
        commands = expected_commands(root, directory, jar)
        report["rust_revision"] = execute("git-revision", commands["git-revision"]).decode().strip()
        report["rust_dirty"] = bool(execute("git-dirty", commands["git-dirty"]))
        report["runtime_versions"] = {}
        for tool in ("java", "javac", "rustc", "cargo"):
            execute("version-" + tool, commands["version-" + tool])
            proc = report["processes"]["version-" + tool]
            report["runtime_versions"][tool] = (Path(proc["stdout"]).read_bytes() + Path(proc["stderr"]).read_bytes()).decode().strip()
        (directory / "classes").mkdir()
        execute("java-build", commands["java-build"])
        report["log_paths"]["java_classes"] = str(directory / "classes")
        report["adapter_class_sha256"] = {str(path.relative_to(directory / "classes")): sha256(path.read_bytes())
                                           for path in (directory / "classes").rglob("*.class")}
        outputs, audits = [], []
        for label in ("java-run", "java-repeat"):
            output = execute(label, commands[label], raw)
            parse_output(output, cases, label)
            audit_path = directory / (label + "-audit.json")
            audit_bytes = audit_path.read_bytes()
            audit = strict_json(audit_bytes, label + " audit")
            validate_audit(audit, cases, jar, pinned, raw, output)
            outputs.append(output)
            audits.append(audit)
            report["log_paths"][label + "-audit"] = str(audit_path)
            report[label + "-audit-sha256"] = sha256(audit_bytes)
        if outputs[0] != outputs[1] or audits[0] != audits[1]:
            raise LightError("native outputs or audits are not repeatable")
        report["reference_output_sha256"] = sha256(outputs[0])
        report["reference_repeat_output_sha256"] = sha256(outputs[1])
        if not input_path and outputs[0] != (root / FIXTURES / "reference.tsv").read_bytes():
            raise LightError("native output differs from committed fixture")
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust light_probe missing; native capture completed twice")
        else:
            records = [json.loads(line) for line in execute("rust-build", commands["rust-build"]).splitlines()]
            binaries = [Path(item["executable"]) for item in records if item.get("reason") == "compiler-artifact"
                        and item.get("target", {}).get("name") == EXAMPLE and item.get("executable")]
            if len(binaries) != 1:
                raise LightError("cargo did not identify exactly one light probe executable")
            shutil.copy2(binaries[0], directory / EXAMPLE)
            report["log_paths"]["rust_binary"] = str(directory / EXAMPLE)
            report["rust_binary_sha256"] = sha256((directory / EXAMPLE).read_bytes())
            actual = execute("rust-run", commands["rust-run"], raw)
            report["rust_output_sha256"] = sha256(actual)
            report.update(compare(outputs[0], actual, cases))
            report.update(status="fail" if report["mismatch_count"] else "pass",
                          reason="light mismatch" if report["mismatch_count"] else "all native light results match exactly")
        if sha256(jar.read_bytes()) != report["jar_sha256"]:
            raise LightError("configured client JAR changed during run")
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, json.JSONDecodeError) as error:
        report.update(status="error", reason=str(error))
    finally:
        report["source_hashes_after"] = source_snapshot(root)
        if report["source_hashes"] != report["source_hashes_after"]:
            report.update(status="error", reason="source inputs changed during run")
        for relative, digest in report["source_hashes"].items():
            if digest is not None and (root / relative).is_file():
                dest = directory / "sources" / relative
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes((root / relative).read_bytes())
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, help="custom valid LIGHT TSV, retained with the report")
    parser.add_argument("--timeout", type=float, default=90)
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()
    if not 0 < args.timeout < float("inf"):
        parser.error("timeout must be positive and finite")
    report, path = run_light_suite(timeout=args.timeout, input_path=args.input)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return exit_code(report["status"], args.strict)


if __name__ == "__main__":
    raise SystemExit(main())
