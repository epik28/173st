"""Verify saved JAVA-001 parity evidence and its failure-detection paths."""
import hashlib
import json
from pathlib import Path
import subprocess

from java_run import (ORACLE, ROOT, JavaRunError, compare, parse_input, parse_output,
                      source_snapshot)


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
            _require(logs.get(label, {}).get(stream) == saved,
                     f"saved {label} {stream} path disagrees with log_paths")
            path = Path(saved)
            _require(path.is_file() and not path.is_symlink(),
                     f"saved {label} {stream} log is missing or invalid: {path}")
            _require(sha(path.read_bytes()) == process.get(f"{stream}_sha256"),
                     f"saved {label} {stream} log hash mismatch")
    return processes


def _verify_artifacts(report, log, processes):
    classes = Path(log.get("java_classes", ""))
    adapter = classes / "JavaCompatOracle.class"
    _require(adapter.is_file() and not adapter.is_symlink(),
             f"saved Java adapter class is missing: {adapter}")
    _require(sha(adapter.read_bytes()) == report.get("oracle_class_sha256"),
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


def _verify_java(report_path, report, protocol=False):
    _require(report.get("suite") == "java" and report.get("status") == "pass",
             "requires a successful Java live report")
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
        raise SystemExit("Java oracle outputs are not repeatable")
    cases = parse_input(raw["input"])
    if len(cases) != report["case_count"] or raw["input"] != (ROOT / "tests/fixtures/java/input.tsv").read_bytes():
        raise SystemExit("saved cases differ from committed corpus")
    operation_counts = {}
    for case in cases:
        operation_counts[case["op"]] = operation_counts.get(case["op"], 0) + 1
    if operation_counts != report.get("operation_counts"):
        raise SystemExit("saved operation counts differ from cases")
    if raw["java-run"] != (ROOT / "tests/fixtures/java/reference.tsv").read_bytes():
        raise SystemExit("saved Java output differs from original oracle fixture")
    if sha((ROOT / ORACLE).read_bytes()) != report.get("oracle_sha256"):
        raise SystemExit("oracle source hash differs from report")
    audit = json.loads((ROOT / "tests/fixtures/java/audit.json").read_text())
    if audit.get("oracle_sha256") != report.get("oracle_sha256"):
        raise SystemExit("oracle source differs from the committed corpus audit")
    if audit.get("java_version") != report.get("runtime_versions", {}).get("java"):
        raise SystemExit("JVM runtime differs from the one used to produce the corpus")
    oracle_class = Path(log["java_classes"]) / "JavaCompatOracle.class"
    if sha(oracle_class.read_bytes()) != report.get("oracle_class_sha256"):
        raise SystemExit("compiled Java oracle class hash differs from report")
    if report.get("source_hashes") != report.get("source_hashes_after"):
        raise SystemExit("source hashes changed during comparison")
    if source_snapshot(ROOT) != report.get("source_hashes"):
        raise SystemExit("sources changed since comparison")
    comparison = compare(raw["java-run"], raw["rust-run"], cases)
    _verify_comparison_summary(report, comparison)
    if comparison["mismatch_count"]:
        raise SystemExit("saved Java/Rust outputs no longer match")

    directory = Path(report_path).resolve().parent
    lines = raw["rust-run"].splitlines(keepends=True)
    first = lines[0].decode("ascii").rstrip("\n").split("\t")
    first[1] = str(int(first[1]) + 1)
    mutated = ("\t".join(first) + "\n").encode("ascii") + b"".join(lines[1:])
    if compare(raw["java-run"], mutated, cases)["mismatch_count"] != 1:
        raise SystemExit("numeric output mutation escaped comparator")
    (directory / "synthetic-mutated-rust.tsv").write_bytes(mutated)

    first_fields = lines[0].decode("ascii").rstrip("\n").split("\t")
    type_mutation = ("\t".join((first_fields[0], "not-an-integer")) + "\n").encode() + b"".join(lines[1:])
    rejection = {
        "wrong-type": type_mutation,
        "row-count": b"".join(lines[:-1]),
        "row-order": b"".join(lines[1:2] + lines[:1] + lines[2:]),
    }
    rejected = {}
    for name, data in rejection.items():
        try:
            parse_output(data, cases, "synthetic " + name)
        except JavaRunError as error:
            rejected[name] = str(error)
        else:
            raise SystemExit(f"parser accepted synthetic {name} mutation")
    changed = dict(report["source_hashes"])
    key = next(iter(changed))
    changed[key] = "0" * 64
    if source_snapshot(ROOT) == changed:
        raise SystemExit("source verifier accepted a synthetic hash mutation")
    rejected["source-hash"] = key
    (directory / "mutation-review.json").write_text(json.dumps({
        "kind": "synthetic type/order/row/source mutations; not live parity",
        "rejected": rejected,
    }, ensure_ascii=False, indent=2) + "\n")

    protocol_results = []
    if protocol:
        classes = Path(log["java_classes"])
        binary = Path(log["rust_binary"])
        commands = {"java": ["java", "-cp", str(classes), "JavaCompatOracle"],
                    "rust": [str(binary)]}
        invalid = {
            "unknown-operation": b"bad\tunknown\t00\n",
            "bad-bits": b"bad\tf32_to_i32\txyz\n",
            "invalid-utf8": b"bad\tstring_hash_code\tff\n",
            "duplicate-id-after-valid-row": b"same\tf32_to_i32\t00000000\nsame\tf64_to_i32\t0000000000000000\n",
            "wrong-arity": b"bad\tstring_hash_code\n",
        }
        for name, data in invalid.items():
            for side, command in commands.items():
                try:
                    result = subprocess.run(command, input=data, capture_output=True,
                                            timeout=5, check=False, cwd=ROOT)
                except subprocess.TimeoutExpired as error:
                    raise SystemExit(f"{side} timed out during {name}") from error
                if result.returncode == 0:
                    raise SystemExit(f"{side} accepted {name}")
                if side == "rust" and result.stdout:
                    raise SystemExit("Rust probe leaked partial stdout after a protocol error")
                protocol_results.append({"case": name, "side": side,
                                         "returncode": result.returncode,
                                         "stdout_sha256": sha(result.stdout),
                                         "stderr_sha256": sha(result.stderr)})
        (directory / "protocol-review.json").write_text(json.dumps({
            "kind": "synthetic malformed Java/Rust protocol checks; not live parity",
            "cases": protocol_results,
        }, ensure_ascii=False, indent=2) + "\n")
    print(f"PASS: saved Java report verified; mutations={len(rejected) + 1}; protocol checks={len(protocol_results)}")
    return 0


def verify_java(report_path, report, protocol=False):
    try:
        return _verify_java(report_path, report, protocol)
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise SystemExit(f"invalid saved Java evidence: {error}") from error
