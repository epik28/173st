"""Saved native motion traces, process provenance, and malformed input checks."""
import json
from pathlib import Path
import subprocess

from geometry_review import require, retained
from movement_run import (ROOT, FIXTURES, JAR_SHA, REQUIRED_PROCESSES, MovementError,
                          compare, expected_commands, jar_path, native_hashes,
                          parse_input, parse_output, source_snapshot, validate_audit,
                          validate_pinned)
from run import sha256


def verify_movement(report_path, report, protocol=False):
    try:
        return _verify_movement(Path(report_path), report, protocol)
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise SystemExit(f"invalid movement evidence: {error}") from error


def _verify_movement(path, report, protocol):
    directory = path.resolve().parent
    require(report.get("suite") == "movement" and report.get("status") == "pass", "requires a successful movement report")
    require(report.get("environment") == "synthetic-static-dry" and report.get("native_world_constructors") == 0,
            "report claims a different native environment")
    jar = jar_path(ROOT)
    commands = expected_commands(ROOT, directory, jar)
    logs, processes = report["log_paths"], report["processes"]
    for label in REQUIRED_PROCESSES:
        p = processes[label]
        require(p.get("returncode") == 0 and p.get("timeout") is False and p.get("error") is None,
                f"process {label} did not complete successfully")
        require(p.get("command") == commands[label] and p.get("cwd") == str(ROOT), f"{label} command metadata differs")
        for stream in ("stdout", "stderr"):
            require(logs[label][stream] == p[stream], f"{label} {stream} path mismatch")
            require(sha256(retained(p[stream], directory)) == p[stream + "_sha256"], f"{label} {stream} hash mismatch")
    require(retained(processes["git-revision"]["stdout"], directory).decode().strip() == report["rust_revision"],
            "revision differs from retained git output")
    require(bool(retained(processes["git-dirty"]["stdout"], directory)) == report["rust_dirty"], "working-tree metadata differs")
    require(report["source_hashes"] == report["source_hashes_after"] == source_snapshot(ROOT), "source snapshots differ")
    binary = Path(logs["rust_binary"])
    require(sha256(retained(binary, directory)) == report["rust_binary_sha256"], "Rust executable hash mismatch")
    classes = Path(logs["java_classes"])
    require(not classes.is_symlink() and classes.resolve().is_relative_to(directory), "unsafe Java class directory")
    class_hashes = {str(p.relative_to(classes)): sha256(retained(p, directory)) for p in classes.rglob("*.class")}
    require(class_hashes and class_hashes == report["adapter_class_hashes"], "Java adapter class hash mismatch")
    require(native_hashes(jar) == report["native_class_sha256"] and report["jar_sha256"] == JAR_SHA
            and Path(report["jar_path"]).resolve() == jar, "native JAR/classes differ")
    for tool in ("java", "javac", "rustc", "cargo"):
        p = processes["version-" + tool]
        require((retained(p["stdout"], directory) + retained(p["stderr"], directory)).decode().strip()
                == report["runtime_versions"][tool], "runtime version metadata differs")
    pinned = json.loads((ROOT / FIXTURES / "audit.json").read_text())
    validate_pinned(ROOT, pinned)
    raw_input = retained(logs["input"], directory)
    require(sha256(raw_input) == report["input_sha256"], "input hash mismatch")
    cases = parse_input(raw_input)
    require(len(cases) == report["case_count"] and sum(len(c["frames"]) for c in cases) == report["state_count"],
            "scenario/state count mismatch")
    require(report.get("scenario") in {"pinned", "custom"}, "unknown scenario source")
    if report["scenario"] == "pinned":
        require(raw_input == (ROOT / FIXTURES / "input.tsv").read_bytes(), "pinned input mismatch")
    outputs, audits = [], []
    for label, field in (("java-run", "reference_output_sha256"), ("java-repeat", "reference_repeat_output_sha256"),
                         ("rust-run", "rust_output_sha256")):
        output = retained(processes[label]["stdout"], directory)
        require(sha256(output) == report[field], f"{label} output hash mismatch")
        states = parse_output(output, cases, label)
        outputs.append(output)
        if label.startswith("java"):
            raw_audit = retained(logs[label + "-audit"], directory)
            require(sha256(raw_audit) == report[label + "-audit-sha256"], "native audit hash mismatch")
            audit = json.loads(raw_audit)
            validate_audit(audit, cases, states, pinned, jar)
            capture = retained(logs[label + "-registry"], directory)
            require(sha256(capture) == audit.get("registry_stdout_sha256")
                    and len(capture) == audit.get("registry_stdout_bytes"), "native registry stdout differs from audit")
            audits.append(audit)
    require(outputs[0] == outputs[1] and audits[0] == audits[1], "native results/audits are not repeatable")
    if report["scenario"] == "pinned":
        require(outputs[0] == (ROOT / FIXTURES / "reference.tsv").read_bytes(), "native fixture output mismatch")
    comparison = compare(outputs[0], outputs[2], cases)
    require(comparison["mismatch_count"] == 0 and all(report[k] == v for k, v in comparison.items()),
            "comparison summary disagrees with states")
    lines = outputs[2].splitlines(keepends=True)
    fields = lines[0].decode().rstrip("\n").split("\t")
    state = fields[2].split(",")
    state[0] = f"{int(state[0], 16) ^ 1:016x}"
    mutant = ("\t".join([*fields[:2], ",".join(state)]) + "\n").encode() + b"".join(lines[1:])
    sensitivity = compare(outputs[0], mutant, cases)
    require(sensitivity["mismatch_count"] == 1 and sensitivity["first_mismatch"]["record"] == 0,
            "comparator missed one-bit per-frame mutation")
    malformed = {"type": b"\t".join([fields[0].encode(), b"0", b"bad"]) + b"\n" + b"".join(lines[1:]),
                 "count": b"".join(lines[:-1]),
                 "order": b"".join(lines[1:2] + lines[:1] + lines[2:]) if len(lines) > 1 else outputs[2] * 2}
    for name, data in malformed.items():
        try:
            parse_output(data, cases, "synthetic " + name)
        except MovementError:
            continue
        raise MovementError("parser accepted synthetic " + name)
    (directory / "mutation-review.json").write_text(json.dumps({"kind": "synthetic trace mutations, not native cases",
                                                              "detected": ["raw-bit", *malformed]}, indent=2) + "\n")
    results = []
    if protocol:
        first_row = raw_input.splitlines(keepends=True)[0]
        original = first_row.decode().rstrip("\n").split("\t")
        bad_mode = original.copy()
        bad_mode[1] = "unknown"
        bad_state = original.copy()
        parts = bad_state[2].split(",")
        parts[0] = "7ff0000000000000"
        bad_state[2] = ",".join(parts)
        invalid = {"mode": ("\t".join(bad_mode) + "\n").encode(),
                   "non-finite": ("\t".join(bad_state) + "\n").encode(),
                   "duplicate-after-valid": first_row * 2, "missing-lf": first_row[:-1],
                   "blank-after-valid": first_row + b"\n", "wrong-columns": b"invalid\tmove\n"}
        for name, data in invalid.items():
            audit_path = directory / ("protocol-" + name + "-audit.json")
            for side, command in (("java", ["java", "-XX:-UsePerfData", "-cp", f"{classes}:{jar}",
                                            "MovementOracle", str(jar), str(audit_path)]), ("rust", [str(binary)])):
                try:
                    result = subprocess.run(command, input=data, cwd=ROOT, capture_output=True, timeout=5, check=False)
                except subprocess.TimeoutExpired as error:
                    raise MovementError(f"{side} timeout on {name}") from error
                require(result.returncode != 0 and not result.stdout and result.stderr,
                        f"{side} accepted, leaked stdout or omitted diagnostic for {name}")
                stem = directory / ("protocol-" + name + "-" + side)
                Path(str(stem) + ".stdout").write_bytes(result.stdout)
                Path(str(stem) + ".stderr").write_bytes(result.stderr)
                results.append({"case": name, "side": side, "returncode": result.returncode,
                                "stdout_sha256": sha256(result.stdout), "stderr_sha256": sha256(result.stderr)})
        (directory / "protocol-review.json").write_text(json.dumps({"kind": "synthetic malformed protocol tests",
                                                                  "cases": results}, indent=2) + "\n")
    print(f"PASS: saved movement trace, 4 output mutations, {len(results)} malformed protocol executions")
    return 0
