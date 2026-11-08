"""Verify retained LIGHT native/Rust evidence, then optionally exercise live TSV rejection."""
import json
from pathlib import Path
import subprocess
import re

from light_run import (ROOT, FIXTURES, EXAMPLE, REQUIRED, JAR_SHA, LightError, compare,
                       expected_commands, jar_path, native_hashes, parse_input, parse_output,
                       source_snapshot, strict_json, validate_audit, validate_pinned)
from run import sha256


def require(value, message):
    if not value:
        raise LightError(message)


def retained(path, directory):
    path = Path(path)
    require(not path.is_symlink() and path.is_file(), "missing or symlinked retained artifact: " + str(path))
    require(path.resolve().is_relative_to(directory.resolve()), "retained artifact escapes report directory: " + str(path))
    return path.read_bytes()


def verify_light(report_path, report, protocol=False):
    try:
        return _verify_light(Path(report_path), report, protocol)
    except (OSError, ValueError, KeyError, TypeError, LightError) as error:
        raise SystemExit("invalid LIGHT evidence: " + str(error)) from error


def _verify_light(report_path, report, protocol):
    directory = report_path.resolve().parent
    require(report.get("suite") == "light" and report.get("status") == "pass",
            "successful LIGHT report required")
    require(report.get("rust_dirty") is False, "clean source revision required")
    processes, logs = report["processes"], report["log_paths"]
    for label in REQUIRED:
        process = processes[label]
        require(type(process.get("returncode")) is int and process["returncode"] == 0
                and process.get("timeout") is False and process.get("error") is None,
                "failed process: " + label)
        for stream in ("stdout", "stderr"):
            path = process.get(stream)
            require(logs[label][stream] == path, "process/log path mismatch: " + label)
            require(sha256(retained(path, directory)) == process.get(stream + "_sha256"),
                    "process log hash differs: " + label)
    revision = retained(processes["git-revision"]["stdout"], directory).decode("ascii").strip()
    require(re.fullmatch(r"[0-9a-f]{40}", revision) and revision == report.get("rust_revision"),
            "revision metadata differs from retained git output")
    require(retained(processes["git-dirty"]["stdout"], directory) == b"", "captured worktree was dirty")
    current_revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, check=False)
    require(current_revision.returncode == 0 and current_revision.stdout.decode("ascii").strip() == revision,
            "report revision is not the current HEAD")
    current_dirty = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, check=False)
    require(current_dirty.returncode == 0 and not current_dirty.stdout, "current worktree is dirty")
    require(report.get("source_hashes") == report.get("source_hashes_after") == source_snapshot(ROOT),
            "captured source hashes differ from current sources")
    for name, digest in report["source_hashes"].items():
        if digest is not None:
            require(sha256(retained(directory / "sources" / name, directory)) == digest,
                    "retained source differs: " + name)
    for tool in ("java", "javac", "rustc", "cargo"):
        process = processes["version-" + tool]
        actual = (retained(process["stdout"], directory) + retained(process["stderr"], directory)).decode().strip()
        require(actual == report["runtime_versions"][tool], "runtime version metadata differs: " + tool)

    jar = jar_path(ROOT)
    require(report.get("jar_path") == str(jar) and report.get("jar_sha256") == JAR_SHA
            and sha256(jar.read_bytes()) == JAR_SHA, "configured client JAR identity differs")
    pinned = strict_json((ROOT / FIXTURES / "audit.json").read_bytes(), "pinned audit")
    validate_pinned(ROOT, pinned)
    require(native_hashes(jar, pinned["classes"]) == report.get("native_class_sha256"),
            "native class hashes differ from original JAR")

    commands = expected_commands(ROOT, directory, jar)
    for label, command in commands.items():
        require(processes[label].get("command") == command and processes[label].get("cwd") == str(ROOT),
                "command/cwd differs: " + label)
    for label in REQUIRED:
        for stream in ("stdout", "stderr"):
            expected_log = directory / f"{label}.{stream}"
            require(processes[label][stream] == str(expected_log), "process log filename differs: " + label)
    require(logs.get("input") == str(directory / "input.tsv"), "retained input path differs")
    require(logs.get("java_classes") == str(directory / "classes"), "Java class directory path differs")
    require(logs.get("rust_binary") == str(directory / EXAMPLE), "Rust binary path differs")
    for label in ("java-run", "java-repeat"):
        require(logs.get(label + "-audit") == str(directory / (label + "-audit.json")),
                "native audit path differs: " + label)
    classes = Path(logs["java_classes"])
    require(not classes.is_symlink() and classes.is_dir() and classes.resolve().is_relative_to(directory),
            "compiled adapter directory is invalid")
    class_hashes = {str(path.relative_to(classes)): sha256(retained(path, directory))
                    for path in classes.rglob("*.class")}
    require(class_hashes and class_hashes == report.get("adapter_class_sha256"), "compiled Java adapter hash differs")
    binary = Path(logs["rust_binary"])
    require(sha256(retained(binary, directory)) == report.get("rust_binary_sha256"), "retained Rust binary hash differs")
    require(processes["rust-run"].get("command") == [str(binary)], "Rust run command differs from retained binary")

    raw = retained(logs["input"], directory)
    require(sha256(raw) == report.get("input_sha256"), "retained input hash differs")
    cases = parse_input(raw)
    require(len(cases) == report.get("case_count"), "case count differs")
    from collections import Counter
    require(dict(Counter(case["op"] for case in cases)) == report.get("operation_counts"),
            "operation counts differ")
    require(report.get("scenario") in {"pinned", "custom"}, "invalid scenario")
    if report["scenario"] == "pinned":
        require(raw == (ROOT / FIXTURES / "input.tsv").read_bytes(), "pinned input differs from fixture")

    outputs, audits = [], []
    for label, digest_field in (("java-run", "reference_output_sha256"),
                                ("java-repeat", "reference_repeat_output_sha256"),
                                ("rust-run", "rust_output_sha256")):
        output = retained(processes[label]["stdout"], directory)
        require(sha256(output) == report.get(digest_field), "saved output hash differs: " + label)
        parse_output(output, cases, label)
        outputs.append(output)
        if label.startswith("java"):
            audit_path = logs[label + "-audit"]
            audit_bytes = retained(audit_path, directory)
            require(sha256(audit_bytes) == report.get(label + "-audit-sha256"), "native audit hash differs")
            audit = strict_json(audit_bytes, label + " audit")
            validate_audit(audit, cases, jar, pinned, raw, output)
            audits.append(audit)
    require(outputs[0] == outputs[1] and audits[0] == audits[1], "two original-JVM traces differ")
    if report["scenario"] == "pinned":
        require(outputs[0] == (ROOT / FIXTURES / "reference.tsv").read_bytes(), "native output differs from fixture")
    comparison = compare(outputs[0], outputs[2], cases)
    require(comparison["mismatch_count"] == 0 and all(report.get(key) == value for key, value in comparison.items()),
            "saved Rust/native comparison differs")

    first = outputs[2].splitlines(keepends=True)[0].decode("ascii").rstrip("\n").split("\t")
    if first[1] == "sample":
        first[3] = f"{int(first[3], 16) ^ 1:08x}"
    elif first[1] == "time":
        first[2] = f"{int(first[2], 16) ^ 1:08x}"
    else:
        first[2] = str((int(first[2]) + 1) % 16)
    mutated = ("\t".join(first) + "\n").encode("ascii") + b"".join(outputs[2].splitlines(keepends=True)[1:])
    mutation_result = compare(outputs[0], mutated, cases)
    require(mutation_result["mismatch_count"] == 1 and mutation_result["first_mismatch"]["record"] == 0,
            "comparison misses a one-row synthetic mutation")
    mutation_path = directory / "synthetic-mutated-output.tsv"
    mutation_path.write_bytes(mutated)
    (directory / "mutation-review.json").write_text(json.dumps({
        "kind": "synthetic copied-output mutation; not live native evidence",
        "source_sha256": sha256(outputs[2]), "mutated_sha256": sha256(mutated),
        "mutation_path": str(mutation_path), "comparison": mutation_result}, indent=2) + "\n")

    if protocol:
        checks = run_protocol_checks(directory, commands, jar, classes, binary, pinned)
        review = {"kind": "live LIGHT TSV protocol validation, separate from parity result",
                  "live_report": str(report_path.resolve()), "executions": checks}
        (directory / "protocol-review.json").write_text(json.dumps(review, indent=2) + "\n")
    print(f"PASS: retained LIGHT report, native repeat, exact comparison, mutation sensitivity"
          + (" and live protocol checks" if protocol else ""))
    return 0


def run_protocol_checks(directory, commands, jar, classes, binary, pinned):
    malformed = {
        "empty": b"", "missing-final-lf": b"x\temission\t89", "blank-row": b"x\temission\t89\n\n",
        "duplicate-id": b"x\temission\t89\nx\temission\t91\n",
        "late-invalid-is-atomic": b"ok\temission\t89\nbad\tunknown\n",
        "unknown-op": b"x\tunknown\n", "bad-id": b"bad id\temission\t89\n",
        "non-ascii": b"\xff\temission\t89\n", "sample-arity": b"x\tsample\t0\t0\t0\t0\n",
        "dimension": b"x\tsample\t1\t0\t0\t0\t0\n", "noncanonical-nibble": b"x\tsample\t0\t01\t0\t0\t0\n",
        "nibble-overflow": b"x\tsample\t0\t0\t0\t16\t0\n",
        "i64-overflow": b"x\ttime\t0\t9223372036854775808\t00000000\n",
        "partial-uppercase": b"x\ttime\t0\t-1\t3F800000\n",
        "partial-nonfinite": b"x\ttime\t0\t-1\t7f800000\n",
        "partial-outside": b"x\ttime\t-1\t-1\t3f800001\n",
        "emission-overflow": b"x\temission\t256\n", "extra-fields": b"x\temission\t89\textra\n",
        "row-limit": b"".join(f"e{i}\temission\t89\n".encode() for i in range(100_001)),
        "byte-limit": b"x" * (16 * 1024 * 1024 + 1),
    }
    valid = {
        "negative-time-and-negative-zero": b"t0\ttime\t0\t-1\t80000000\nt1\ttime\t-1\t-9223372036854775808\t3f800000\n",
        "all-operation-shapes": b"s\tsample\t0\t15\t0\t15\t7\nt\ttime\t0\t-24001\t00000000\ne\temission\t89\n",
    }
    entries = []

    def run_case(name, data, should_pass):
        outputs = {}
        for side in ("java", "rust"):
            audit_path = directory / f"protocol-{name}-{side}-audit.json"
            if side == "java":
                command = list(commands["java-run"])
                command[-1] = str(audit_path)
            else:
                command = list(commands["rust-run"])
            try:
                proc = subprocess.run(command, cwd=ROOT, input=data, capture_output=True, timeout=30, check=False)
            except subprocess.TimeoutExpired as error:
                raise LightError(f"protocol check timeout: {name}/{side}") from error
            out_path, err_path = directory / f"protocol-{name}-{side}.stdout", directory / f"protocol-{name}-{side}.stderr"
            out_path.write_bytes(proc.stdout); err_path.write_bytes(proc.stderr)
            accepted = proc.returncode == 0
            audit_meta = {}
            if should_pass:
                cases = parse_input(data)
                parse_output(proc.stdout, cases, f"protocol {side}")
                accepted = accepted and bool(proc.stdout) and bool(proc.stderr) is False
                outputs[side] = proc.stdout
                if side == "java":
                    audit_bytes = audit_path.read_bytes()
                    audit = strict_json(audit_bytes, "protocol audit")
                    validate_audit(audit, cases, jar, pinned, data, proc.stdout)
                    audit_meta = {"audit": str(audit_path), "audit_sha256": sha256(audit_bytes)}
            else:
                accepted = not accepted and proc.stdout == b"" and bool(proc.stderr) and not audit_path.exists()
            entries.append({"case": name, "side": side, "valid": should_pass, "accepted": accepted,
                            "command": command, "returncode": proc.returncode,
                            "stdout": str(out_path), "stdout_sha256": sha256(proc.stdout),
                            "stderr": str(err_path), "stderr_sha256": sha256(proc.stderr), **audit_meta})
            require(accepted, f"protocol {side} unexpected result for {name}: exit {proc.returncode}")
        if should_pass:
            require(outputs["java"] == outputs["rust"], "valid protocol outputs differ: " + name)

    for name, data in malformed.items():
        run_case(name, data, False)
    for name, data in valid.items():
        run_case(name, data, True)

    # Wrong configured JAR must fail in the native process before producing output/audit.
    missing_jar = directory / "absent-client-bad.jar"
    wrong_audit = directory / "protocol-wrong-jar-audit.json"
    command = list(commands["java-run"]); command[-2] = str(missing_jar); command[-1] = str(wrong_audit)
    result = subprocess.run(command, cwd=ROOT, input=b"x\temission\t89\n", capture_output=True, timeout=15)
    require(result.returncode != 0 and result.stdout == b"" and result.stderr and not wrong_audit.exists(),
            "native oracle accepted missing/wrong JAR")
    (directory / "protocol-wrong-jar.stdout").write_bytes(result.stdout)
    (directory / "protocol-wrong-jar.stderr").write_bytes(result.stderr)
    entries.append({"case": "wrong-jar", "side": "java", "valid": False, "accepted": True,
                    "command": command, "returncode": result.returncode,
                    "stdout": str(directory / "protocol-wrong-jar.stdout"),
                    "stdout_sha256": sha256(result.stdout),
                    "stderr": str(directory / "protocol-wrong-jar.stderr"),
                    "stderr_sha256": sha256(result.stderr)})
    return entries


if __name__ == "__main__":
    raise SystemExit("use verify_report.py <report.json> --protocol")
