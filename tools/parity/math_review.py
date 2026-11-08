"""Verify saved MATH-001 parity evidence and exercise rejection paths."""
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

from math_run import (CLASS_SHA, JAR_SHA, ORACLE, ROOT, compare_math_outputs,
                      MathRunError, parse_math_input, parse_math_output, source_snapshot)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def _require(condition, message):
    if not condition:
        raise SystemExit(message)


def _saved_processes(report, expected):
    processes = report.get("processes")
    _require(isinstance(processes, dict), "saved process records are missing or invalid")
    logs = report.get("log_paths")
    _require(isinstance(logs, dict), "saved log_paths are missing or invalid")
    for label in expected:
        process = processes.get(label)
        _require(isinstance(process, dict), f"saved process record {label} is missing")
        _require(process.get("returncode") == 0 and process.get("timeout") is False
                 and process.get("error") is None,
                 f"saved process {label} did not complete successfully")
        for stream in ("stdout", "stderr"):
            saved = process.get(stream)
            _require(isinstance(saved, str) and saved,
                     f"saved {label} {stream} path is missing")
            expected_path = logs.get(label, {}).get(stream)
            _require(expected_path == saved,
                     f"saved {label} {stream} path disagrees with log_paths")
            path = Path(saved)
            _require(path.is_file() and not path.is_symlink(),
                     f"saved {label} {stream} log is missing or invalid: {path}")
            _require(sha(path.read_bytes()) == process.get(f"{stream}_sha256"),
                     f"saved {label} {stream} log hash mismatch")
    return processes


def _verify_artifacts(report, log, processes):
    classes = Path(log.get("java_classes", ""))
    adapter = classes / "MathOracle.class"
    _require(adapter.is_file() and not adapter.is_symlink(),
             f"saved Java adapter class is missing: {adapter}")
    _require(sha(adapter.read_bytes()) == report.get("adapter_class_sha256"),
             "saved Java adapter class hash mismatch")
    binary = Path(log.get("rust_binary", ""))
    _require(binary.is_file() and not binary.is_symlink(),
             f"saved Rust executable is missing: {binary}")
    _require(sha(binary.read_bytes()) == report.get("rust_binary_sha256"),
             "saved Rust executable hash mismatch")
    command = processes["rust-run"].get("command")
    _require(isinstance(command, list) and command and Path(command[0]) == binary,
             "saved rust-run executable path disagrees with rust_binary")


def _verify_revision(report, processes):
    revision = Path(processes["git-revision"]["stdout"]).read_text().strip()
    _require(revision and revision == report.get("rust_revision"),
             "saved git revision disagrees with report")
    dirty = Path(processes["git-dirty"]["stdout"]).read_bytes()
    _require(not dirty and report.get("rust_dirty") is False,
             "saved source revision is dirty or disagrees with report")


def _verify_comparison_summary(report, comparison):
    _require(comparison["mismatch_count"] == report.get("mismatch_count")
             and comparison["first_mismatch"] == report.get("first_mismatch"),
             "saved mismatch summary disagrees with outputs")


def _verify_math(report_path, report, protocol=False):
    _require(report.get("suite") == "math" and report.get("status") == "pass",
             "requires a successful math live report")
    expected = ("git-revision", "git-dirty", "version-java", "version-javac",
                "version-rustc", "version-cargo", "java-build", "java-run",
                "java-repeat", "rust-build", "rust-run")
    processes = _saved_processes(report, expected)
    log = report["log_paths"]
    _verify_artifacts(report, log, processes)
    _verify_revision(report, processes)
    _require(isinstance(log.get("input"), str), "saved input path is missing")
    paths = {"input": Path(log["input"])}
    paths.update({key: Path(processes[key]["stdout"])
                  for key in ("java-run", "java-repeat", "rust-run")})
    try:
        raw = {key: path.read_bytes() for key, path in paths.items()}
    except OSError as error:
        raise SystemExit(f"cannot read saved comparison artifact: {error}") from error
    for key, field in (("input", "input_sha256"), ("java-run", "reference_output_sha256"),
                       ("java-repeat", "reference_repeat_output_sha256"),
                       ("rust-run", "rust_output_sha256")):
        if sha(raw[key]) != report[field]:
            raise SystemExit(f"saved {key} bytes do not match report")
    if raw["java-run"] != raw["java-repeat"]:
        raise SystemExit("saved original outputs are not repeatable")
    cases = parse_math_input(raw["input"])
    if len(cases) != report["case_count"]:
        raise SystemExit("saved case count mismatch")
    compare = compare_math_outputs(raw["java-run"], raw["rust-run"], cases)
    _verify_comparison_summary(report, compare)
    if compare["mismatch_count"]:
        raise SystemExit("saved outputs no longer match")

    fixture = ROOT / "tests/fixtures/math/reference.tsv"
    if raw["java-run"] != fixture.read_bytes():
        raise SystemExit("saved original output no longer matches committed oracle fixture")
    config = tomllib.loads((ROOT / "config/local.toml").read_text()).get("paths", {})
    jar = (ROOT / config["client_jar"]).resolve(strict=True)
    if sha(jar.read_bytes()) != JAR_SHA or report.get("reference_jar_sha256") != JAR_SHA:
        raise SystemExit("original JAR hash no longer matches report")
    with __import__("zipfile").ZipFile(jar) as archive:
        if sha(archive.read("in.class")) != CLASS_SHA or report.get("reference_class_sha256") != CLASS_SHA:
            raise SystemExit("original MathHelper class hash no longer matches report")
    if report.get("source_hashes") != report.get("source_hashes_after"):
        raise SystemExit("report source hashes changed during original comparison")
    if source_snapshot(ROOT) != report["source_hashes"]:
        raise SystemExit("source files changed since the saved comparison")
    if report.get("reference_invocations") != report.get("operation_counts"):
        raise SystemExit("original method invocation counts do not match cases")

    actual_lines = raw["rust-run"].splitlines(keepends=True)
    first = actual_lines[0].decode("ascii").rstrip("\n").split("\t")
    op = cases[0]["op"]
    if op in {"sin", "cos", "sqrt_f32", "sqrt_f64_to_f32", "abs_f32"}:
        value = int(first[1], 16)
        first[1] = f"{(value ^ 1):08x}"
    else:
        first[1] = "1" if first[1] == "0" else "0"
    actual_lines[0] = ("\t".join(first) + "\n").encode("ascii")
    mutated = b"".join(actual_lines)
    mutation = compare_math_outputs(raw["java-run"], mutated, cases)
    if mutation["mismatch_count"] != 1 or mutation["first_mismatch"]["record"] != 0:
        raise SystemExit("math comparator missed a one-record mutation")
    directory = Path(report_path).resolve().parent
    mutant_path = directory / "synthetic-mutated-rust.tsv"
    mutant_path.write_bytes(mutated)
    (directory / "comparator-sensitivity.json").write_text(json.dumps({
        "kind": "synthetic one-bit output mutation; not a live parity result",
        "path": str(mutant_path), "comparison": mutation,
        "original_output_sha256": sha(raw["rust-run"]), "mutated_output_sha256": sha(mutated),
    }, indent=2) + "\n")

    rejection_cases = {}
    first_line, *remaining_lines = raw["rust-run"].splitlines(keepends=True)
    first_fields = first_line.decode("ascii").rstrip("\n").split("\t")
    first_fields[1] = "not-a-float"
    rejection_cases["wrong-type"] = ("\t".join(first_fields) + "\n").encode("ascii") + b"".join(remaining_lines)
    rejection_cases["row-count"] = b"".join(remaining_lines)
    rejection_cases["row-order"] = b"".join(remaining_lines[:1] + [first_line] + remaining_lines[1:])
    rejected = {}
    for name, candidate in rejection_cases.items():
        try:
            parse_math_output(candidate, cases, f"synthetic {name}")
        except MathRunError as error:
            rejected[name] = str(error)
        else:
            raise SystemExit(f"math report review accepted synthetic {name} mutation")
    changed_sources = dict(report["source_hashes"])
    first_source = next(iter(changed_sources))
    changed_sources[first_source] = "0" * 64
    if source_snapshot(ROOT) == changed_sources:
        raise SystemExit("math source verifier accepted synthetic source-hash mutation")
    rejected["source-hash"] = first_source
    (directory / "mutation-review.json").write_text(json.dumps({
        "kind": "synthetic protocol/output/source mutations; not live parity",
        "rejected": rejected,
    }, indent=2) + "\n")

    protocol_results = []
    if protocol:
        classes = Path(log["java_classes"])
        binary = Path(log["rust_binary"])
        java_command = ["java", "-cp", f"{classes}:{jar}", "MathOracle", str(jar)]
        invalid = {
            "unknown-operation": b"bad\tbogus\t00\n",
            "malformed-float": b"bad\tsin\txyz\n",
            "out-of-range-i32": b"bad\tfloor_div_i32\t2147483648\t1\n",
            "invalid-utf8": b"bad\tnull_or_empty\tff\n",
            "duplicate-id": b"same\tsin\t00000000\nsame\tcos\t00000000\n",
        }
        for name, data in invalid.items():
            for side, command in (("java", java_command), ("rust", [str(binary)])):
                try:
                    result = subprocess.run(command, input=data, capture_output=True, timeout=5,
                                            check=False, cwd=ROOT)
                except subprocess.TimeoutExpired as error:
                    raise SystemExit(f"{side} timeout during {name} protocol check") from error
                if result.returncode == 0:
                    raise SystemExit(f"{side} accepted invalid {name} input")
                stem = f"synthetic-{name}-{side}"
                (directory / f"{stem}.stdout").write_bytes(result.stdout)
                (directory / f"{stem}.stderr").write_bytes(result.stderr)
                protocol_results.append({"case": name, "side": side, "exit": result.returncode,
                                         "stdout_sha256": sha(result.stdout),
                                         "stderr_sha256": sha(result.stderr)})
        (directory / "protocol-review.json").write_text(json.dumps({
            "kind": "synthetic malformed protocol checks; not live parity",
            "cases": protocol_results,
        }, indent=2) + "\n")
    print(f"PASS: saved math report verified; synthetic mutations={len(rejected) + 1}; protocol checks={len(protocol_results)}")
    return 0


def verify_math(report_path, report, protocol=False):
    try:
        return _verify_math(report_path, report, protocol)
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise SystemExit(f"invalid saved math evidence: {error}") from error
