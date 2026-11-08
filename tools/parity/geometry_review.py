"""Review retained native geometry artifacts before any optional process execution."""
from collections import Counter
import json
from pathlib import Path
import subprocess

from geometry_run import (ROOT, FIXTURES, JAR_SHA, REQUIRED_PROCESSES, SPECS,
                          GeometryError, compare, jar_path, native_hashes,
                          parse_input, parse_output, source_snapshot, validate_audit,
                          validate_pinned, expected_commands)
from run import sha256


def require(condition, message):
    if not condition:
        raise GeometryError(message)


def retained(path, directory):
    path = Path(path)
    require(not path.is_symlink() and path.is_file(), f"missing or symlinked retained artifact: {path}")
    require(path.resolve().is_relative_to(directory.resolve()), f"retained artifact escapes report directory: {path}")
    return path.read_bytes()


def verify_geometry(report_path, report, protocol=False):
    try:
        return _verify_geometry(Path(report_path), report, protocol)
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise SystemExit(f"invalid geometry evidence: {error}") from error


def _verify_geometry(report_path, report, protocol):
    directory = report_path.resolve().parent
    require(report.get("suite") == "geometry" and report.get("status") == "pass",
            "requires a successful geometry report")
    logs, processes = report["log_paths"], report["processes"]
    for label in REQUIRED_PROCESSES:
        p = processes[label]
        require(p.get("returncode") == 0 and p.get("timeout") is False and p.get("error") is None,
                f"process {label} did not complete successfully")
        for stream in ("stdout", "stderr"):
            require(logs[label][stream] == p[stream], f"{label} {stream} path disagreement")
            require(sha256(retained(p[stream], directory)) == p[stream + "_sha256"],
                    f"{label} {stream} hash mismatch")
    require(retained(processes["git-revision"]["stdout"], directory).decode().strip() == report["rust_revision"],
            "revision metadata disagrees with git output")
    require(bool(retained(processes["git-dirty"]["stdout"], directory)) == report["rust_dirty"],
            "working tree metadata disagrees with git output")
    require(report["source_hashes"] == report["source_hashes_after"] == source_snapshot(ROOT),
            "source hashes changed or disagree with current sources")
    binary = Path(logs["rust_binary"])
    require(sha256(retained(binary, directory)) == report["rust_binary_sha256"], "Rust executable hash mismatch")
    require(processes["rust-run"]["command"] == [str(binary)], "Rust run command differs from saved executable")
    classes = Path(logs["java_classes"])
    require(not classes.is_symlink() and classes.resolve().is_relative_to(directory), "unsafe Java class directory")
    actual_classes = {str(p.relative_to(classes)): sha256(retained(p, directory)) for p in classes.rglob("*.class")}
    require(actual_classes and actual_classes == report["adapter_class_hashes"], "Java adapter class hashes differ")
    for tool in ("java", "javac", "rustc", "cargo"):
        p = processes["version-" + tool]
        version = (retained(p["stdout"], directory) + retained(p["stderr"], directory)).decode().strip()
        require(version == report["runtime_versions"][tool], "runtime version differs from saved process")
    jar = jar_path(ROOT)
    commands = expected_commands(ROOT, directory, jar)
    for label, command in commands.items():
        require(processes[label].get("command") == command and processes[label].get("cwd") == str(ROOT),
                f"{label} command or working directory metadata differs")
    require(Path(report["jar_path"]).resolve() == jar and report["jar_sha256"] == JAR_SHA,
            "report references an unexpected original JAR")
    require(native_hashes(jar) == report["native_class_sha256"], "native class hashes differ")
    pinned = json.loads((ROOT / FIXTURES / "audit.json").read_text())
    validate_pinned(ROOT, pinned)
    input_bytes = retained(logs["input"], directory)
    require(sha256(input_bytes) == report["input_sha256"], "saved input hash mismatch")
    cases = parse_input(input_bytes)
    require(len(cases) == report["case_count"], "case count mismatch")
    require(dict(Counter(c["op"] for c in cases)) == report["operation_counts"], "operation counts mismatch")
    require(report.get("scenario") in {"pinned", "custom"}, "unknown scenario origin")
    if report["scenario"] == "pinned":
        require(input_bytes == (ROOT / FIXTURES / "input.tsv").read_bytes(), "pinned input differs from committed fixture")
    outputs, audits = [], []
    for label, field in (("java-run", "reference_output_sha256"),
                         ("java-repeat", "reference_repeat_output_sha256"),
                         ("rust-run", "rust_output_sha256")):
        raw = retained(processes[label]["stdout"], directory)
        require(sha256(raw) == report[field], f"saved {label} output hash mismatch")
        parse_output(raw, cases, label)
        outputs.append(raw)
        if label.startswith("java"):
            audit_bytes = retained(logs[label + "-audit"], directory)
            require(sha256(audit_bytes) == report[label + "-audit-sha256"], "native audit hash mismatch")
            audit = json.loads(audit_bytes)
            validate_audit(audit, cases, pinned, jar)
            audits.append(audit)
    require(audits[0] == audits[1] and outputs[0] == outputs[1], "native outputs/audits are not repeatable")
    if report["scenario"] == "pinned":
        require(outputs[0] == (ROOT / FIXTURES / "reference.tsv").read_bytes(), "native fixture output mismatch")
    comparison = compare(outputs[0], outputs[2], cases)
    require(comparison["mismatch_count"] == 0 and all(report[k] == v for k, v in comparison.items()),
            "saved comparison differs from actual outputs")

    lines = outputs[2].splitlines(keepends=True)
    fields = lines[0].decode().rstrip("\n").split("\t")
    kind = SPECS[cases[0]["op"]][1]
    value = fields[1]
    if kind == "boolean":
        changed = "1" if value == "0" else "0"
    elif value == "none":
        changed = ("0," if kind == "ray" else "") + ",".join(["0000000000000000"] * 3)
    else:
        parts = value.split(",")
        parts[-1] = f"{int(parts[-1], 16) ^ 1:016x}"
        changed = ",".join(parts)
    mutant = f"{fields[0]}\t{changed}\n".encode() + b"".join(lines[1:])
    require(compare(outputs[0], mutant, cases)["mismatch_count"] == 1, "comparator missed one-record mutation")
    rejection = {"type": f"{fields[0]}\tbad\n".encode() + b"".join(lines[1:]),
                 "count": b"".join(lines[:-1]),
                 "order": b"".join(lines[1:2] + lines[:1] + lines[2:]) if len(lines) > 1 else outputs[2] * 2}
    for name, data in rejection.items():
        try:
            parse_output(data, cases, "synthetic " + name)
        except GeometryError:
            continue
        raise GeometryError("parser accepted synthetic " + name)
    (directory / "mutation-review.json").write_text(json.dumps({
        "kind": "synthetic output mutations, not additional live parity cases",
        "detected": ["one-record-value", *rejection]}, indent=2) + "\n")
    checks = []
    if protocol:
        zeros = b"\t0000000000000000" * 3
        good = b"same\tvec_new" + zeros + b"\n"
        invalid = {"unknown": b"bad\tunknown" + zeros + b"\n",
                   "arity": b"bad\tvec_new\t0000000000000000\n",
                   "bits": b"bad\tvec_new\txyz" + zeros[:34] + b"\n",
                   "non-finite": b"bad\tvec_new\t7ff0000000000000" + zeros[:34] + b"\n",
                   "duplicate-after-valid": good * 2,
                   "missing-lf": good[:-1], "blank-row": good + b"\n"}
        for name, data in invalid.items():
            audit_path = directory / ("protocol-" + name + "-audit.json")
            commands = {"java": ["java", "-XX:-UsePerfData", "-cp", f"{classes}:{jar}",
                                  "GeometryOracle", str(jar), str(audit_path)], "rust": [str(binary)]}
            for side, command in commands.items():
                try:
                    result = subprocess.run(command, input=data, cwd=ROOT, capture_output=True, timeout=5, check=False)
                except subprocess.TimeoutExpired as error:
                    raise GeometryError(f"{side} timeout on {name}") from error
                require(result.returncode != 0 and not result.stdout and result.stderr,
                        f"{side} accepted, leaked stdout or omitted diagnostics for {name}")
                stem = directory / ("protocol-" + name + "-" + side)
                Path(str(stem) + ".stdout").write_bytes(result.stdout)
                Path(str(stem) + ".stderr").write_bytes(result.stderr)
                checks.append({"case": name, "side": side, "returncode": result.returncode,
                               "stdout_sha256": sha256(result.stdout), "stderr_sha256": sha256(result.stderr)})
        (directory / "protocol-review.json").write_text(json.dumps({"kind": "malformed synthetic protocol tests", "cases": checks}, indent=2) + "\n")
    print(f"PASS: saved geometry report, 4 output mutations, {len(checks)} malformed protocol executions")
    return 0
