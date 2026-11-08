"""Run exact parity checks for Java-compatible primitive conversions and hashing."""
from datetime import datetime, timezone
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile
import time

from run import ROOT, artifacts_root, exit_code, run_process, sha256

ORACLE = Path("tools/oracle/src/JavaCompatOracle.java")
CORPUS = Path("tools/oracle/java_compat_corpus.py")
INPUT = Path("tests/fixtures/java/input.tsv")
REFERENCE = Path("tests/fixtures/java/reference.tsv")
AUDIT = Path("tests/fixtures/java/audit.json")
PROBE = Path("crates/beta-core/examples/java_compat_probe.rs")
EXAMPLE = "java_compat_probe"
OPS = {"f32_to_i32": 32, "f64_to_i32": 32, "f32_to_i64": 64,
       "f64_to_i64": 64, "string_hash_code": 32}
RAW_WIDTH = {"f32_to_i32": 8, "f64_to_i32": 16,
             "f32_to_i64": 8, "f64_to_i64": 16}


class JavaRunError(Exception):
    pass


def parse_input(data):
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise JavaRunError("input is not ASCII") from error
    if not text.endswith("\n"):
        raise JavaRunError("input requires final LF")
    cases, seen = [], set()
    for line_no, line in enumerate(text[:-1].split("\n"), 1):
        fields = line.split("\t")
        if len(fields) != 3 or not re.fullmatch(r"[A-Za-z0-9_.:-]{1,96}", fields[0]):
            raise JavaRunError(f"malformed input line {line_no}")
        case_id, op, arg = fields
        if case_id in seen or op not in OPS:
            raise JavaRunError(f"duplicate ID or unknown operation at line {line_no}")
        seen.add(case_id)
        if op == "string_hash_code":
            if len(arg) % 2 or not re.fullmatch(r"[0-9a-f]*", arg):
                raise JavaRunError(f"invalid UTF-8 hex at line {line_no}")
            try:
                bytes.fromhex(arg).decode("utf-8")
            except UnicodeDecodeError as error:
                raise JavaRunError(f"invalid UTF-8 at line {line_no}") from error
        elif len(arg) != RAW_WIDTH[op] or not re.fullmatch(r"[0-9a-f]+", arg):
            raise JavaRunError(f"invalid raw bits at line {line_no}")
        cases.append({"id": case_id, "op": op, "arg": arg})
    if not cases or len(cases) > 100_000:
        raise JavaRunError(f"row count outside limit: {len(cases)}")
    return cases


def parse_output(data, cases, label):
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise JavaRunError(f"{label} is not ASCII") from error
    if not text.endswith("\n"):
        raise JavaRunError(f"{label} requires final LF")
    lines = text[:-1].split("\n")
    if len(lines) != len(cases):
        raise JavaRunError(f"{label} row count mismatch")
    values = []
    for index, (line, case) in enumerate(zip(lines, cases)):
        fields = line.split("\t")
        if len(fields) != 2 or fields[0] != case["id"] or not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", fields[1]):
            raise JavaRunError(f"{label} malformed or out-of-order row {index}")
        width = OPS[case["op"]]
        value = int(fields[1])
        if not -(1 << (width - 1)) <= value < (1 << (width - 1)):
            raise JavaRunError(f"{label} integer outside {width}-bit range at row {index}")
        values.append(fields[1])
    return values


def compare(reference, actual, cases):
    left, right = parse_output(reference, cases, "Java output"), parse_output(actual, cases, "Rust output")
    mismatch = None
    count = 0
    for index, (expected, observed) in enumerate(zip(left, right)):
        if expected != observed:
            count += 1
            if mismatch is None:
                mismatch = {"record": index, "case": cases[index]["id"],
                            "operation": cases[index]["op"],
                            "reference": expected, "rust": observed}
    return {"mismatch_count": count, "first_mismatch": mismatch}


def source_snapshot(root=ROOT):
    fixed = {Path("Cargo.toml"), Path("Cargo.lock"), Path("rust-toolchain.toml"),
             Path("config/local.toml"), ORACLE, CORPUS, INPUT, REFERENCE, AUDIT, PROBE,
             Path("tests/python/test_java_compat_corpus.py"),
             Path("tests/python/test_java_runner.py"),
             Path("tools/parity/run.py"), Path("tools/parity/java_run.py"),
             Path("tools/parity/java_review.py"), Path("tools/parity/verify_report.py"),
             Path("crates/beta-core/src/java_compat.rs")}
    return {str(path): sha256((root / path).read_bytes()) if (root / path).is_file() else None
            for path in sorted(fixed)}


def run_java(root, directory, label, command, data=None, timeout=60, processes=None):
    record = run_process(command, cwd=root, directory=directory, label=label,
                         timeout=timeout, input_data=data)
    if processes is not None:
        processes[label] = record
    if record["error"] or record["returncode"] != 0:
        raise JavaRunError(f"{label}: {record['error'] or 'exit ' + str(record['returncode'])}")
    return Path(record["stdout"]).read_bytes()


def run_java_suite(root=ROOT, timeout=60):
    started = time.monotonic()
    parent = artifacts_root(root) / "parity/java"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {
        "schema_version": 1, "suite": "java", "scenario_id": "JAVA-001-all-operations",
        "started_at": datetime.now(timezone.utc).isoformat(), "status": "error",
        "reason": "comparison not completed", "reference_side": "direct JVM casts and String.hashCode",
        "runtime_versions": {}, "platform": platform.platform(), "toolchain": {},
        "oracle_sha256": None, "source_hashes": {}, "source_hashes_after": {},
        "input_sha256": None, "case_count": 0, "operation_counts": {},
        "mismatch_count": None, "first_mismatch": None,
        "reference_output_sha256": None, "reference_repeat_output_sha256": None,
        "rust_output_sha256": None, "rust_revision": None, "rust_dirty": None,
        "rust_binary_sha256": None, "comparison_policy": "exact ordered decimal TSV",
        "log_paths": {"input": str(directory / "input.tsv"), "report": str(report_path)},
        "processes": {},
    }
    before = None
    input_data = None

    def execute(label, command, data=None):
        result = run_process(command, cwd=root, directory=directory, label=label,
                             timeout=timeout, input_data=data)
        report["processes"][label] = result
        report["log_paths"][label] = {key: result[key] for key in ("stdout", "stderr")}
        if result["error"] or result["returncode"] != 0:
            raise JavaRunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
        return Path(result["stdout"]).read_bytes()

    try:
        before = source_snapshot(root)
        report["source_hashes"] = before
        report["oracle_sha256"] = sha256((root / ORACLE).read_bytes())
        input_data = (root / INPUT).read_bytes()
        (directory / "input.tsv").write_bytes(input_data)
        report["input_sha256"] = sha256(input_data)
        cases = parse_input(input_data)
        report["case_count"] = len(cases)
        for case in cases:
            report["operation_counts"][case["op"]] = report["operation_counts"].get(case["op"], 0) + 1
        execute("git-revision", ["git", "rev-parse", "HEAD"])
        report["rust_revision"] = Path(report["processes"]["git-revision"]["stdout"]).read_text().strip()
        report["rust_dirty"] = bool(Path(execute("git-dirty", ["git", "status", "--porcelain"])).read_bytes())
        for tool, args in (("java", ["java", "-version"]), ("javac", ["javac", "-version"]),
                           ("rustc", ["rustc", "--version"]), ("cargo", ["cargo", "--version"])):
            execute(f"version-{tool}", args)
            item = report["processes"][f"version-{tool}"]
            report["runtime_versions"][tool] = (Path(item["stdout"]).read_bytes()
                                                   + Path(item["stderr"]).read_bytes()).decode().strip()
        report["toolchain"] = {key: report["runtime_versions"][key] for key in ("rustc", "cargo")}
        classes = directory / "classes"
        classes.mkdir()
        execute("java-build", ["javac", "-encoding", "UTF-8", "-d", str(classes), str(root / ORACLE)])
        report["oracle_class_sha256"] = sha256((classes / "JavaCompatOracle.class").read_bytes())
        report["log_paths"]["java_classes"] = str(classes)
        command = ["java", "-cp", str(classes), "JavaCompatOracle"]
        reference = execute("java-run", command, input_data)
        repeated = execute("java-repeat", command, input_data)
        if reference != repeated or reference != (root / REFERENCE).read_bytes():
            raise JavaRunError("original JVM outputs are not repeatable or differ from fixture")
        parse_output(reference, cases, "Java output")
        report["reference_output_sha256"] = sha256(reference)
        report["reference_repeat_output_sha256"] = sha256(repeated)
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust java_compat_probe source is absent; Java oracle executed")
        else:
            build = execute("rust-build", ["cargo", "build", "--locked", "--offline", "-p", "beta-core",
                                             "--example", EXAMPLE, "--message-format=json-render-diagnostics"])
            artifacts = []
            for line in build.splitlines():
                event = json.loads(line)
                if event.get("reason") == "compiler-artifact" and event.get("target", {}).get("name") == EXAMPLE and event.get("executable"):
                    artifacts.append(Path(event["executable"]))
            if len(artifacts) != 1:
                raise JavaRunError("Cargo did not identify exactly one Java probe executable")
            binary = directory / EXAMPLE
            shutil.copy2(artifacts[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            report["log_paths"]["rust_binary"] = str(binary)
            actual = execute("rust-run", [str(binary)], input_data)
            report["rust_output_sha256"] = sha256(actual)
            result = compare(reference, actual, cases)
            report.update(result)
            report.update(status="fail" if result["mismatch_count"] else "pass",
                          reason="output mismatch" if result["mismatch_count"] else "all records match exactly")
    except (JavaRunError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        report.update(status="error", reason=str(error))
    finally:
        try:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if before is not None and before != after:
                report.update(status="error", reason="source files changed during comparison")
            if input_data is not None and sha256((directory / "input.tsv").read_bytes()) != report["input_sha256"]:
                report.update(status="error", reason="saved input changed during comparison")
        except (OSError, ValueError) as error:
            report.update(status="error", reason=f"source verification failed: {error}")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--timeout", type=float, default=60)
    args = parser.parse_args(argv)
    report, path = run_java_suite(timeout=args.timeout)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return exit_code(report["status"], args.strict)


if __name__ == "__main__":
    raise SystemExit(main())
