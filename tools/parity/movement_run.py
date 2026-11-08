"""Native entity collision/dry-travel traces; exact raw-bit per-frame comparisons."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import shutil
import struct
import tempfile
import time
import zipfile

from geometry_run import JAR_SHA, jar_path, REQUIRED_PROCESSES
from run import ROOT, artifacts_root, exit_code, run_process, sha256

FIXTURES = Path("tests/fixtures/movement")
ORACLE = Path("tools/oracle/src/MovementOracle.java")
PROBE = Path("crates/beta-core/examples/movement_probe.rs")
EXAMPLE = "movement_probe"
NATIVE_CLASSES = ("sn", "ls", "eq", "bt", "in", "uu")


class MovementError(ValueError):
    pass


def raw_number(raw, width):
    if not re.fullmatch(r"[0-9a-f]{" + str(width // 4) + "}", raw):
        raise MovementError(f"expected lowercase raw f{width} hex")
    value = struct.unpack(">d" if width == 64 else ">f", bytes.fromhex(raw))[0]
    if not -float("inf") < value < float("inf"):
        raise MovementError("non-finite number")
    return value


def bounded_number(raw, width, lo, hi):
    value = raw_number(raw, width)
    if not lo <= value <= hi:
        raise MovementError(f"f{width} argument outside [{lo}, {hi}]")
    return value


def parse_state(raw, initial=False):
    fields = raw.split(",")
    if len(fields) != 18:
        raise MovementError("state requires 18 components")
    numbers = [raw_number(f, 64) for f in fields[:12]]
    numbers += [raw_number(f, 32) for f in fields[12:17]]
    if not re.fullmatch(r"(?:0|[1-9][0-9]{0,2})", fields[17]) or int(fields[17]) > 255:
        raise MovementError("invalid state flags")
    if initial:
        if any(abs(n) > 30_000_000 for n in numbers[:3] + numbers[6:12]):
            raise MovementError("initial coordinates outside world domain")
        if any(abs(n) > 32 for n in numbers[3:6]):
            raise MovementError("initial velocity exceeds 32")
        if any(not 0 < numbers[9 + i] - numbers[6 + i] <= 4 for i in range(3)):
            raise MovementError("initial body extents must be positive and <= 4")
        for index, lo, hi in ((12, -2, 2), (13, 0, 2), (14, 0, 1),
                              (15, 0, 256), (16, -36000, 36000)):
            if not lo <= numbers[index] <= hi:
                raise MovementError("initial f32 state outside field domain")
    return fields


def parse_input(data):
    if not data or len(data) > 16 * 1024 * 1024:
        raise MovementError("input must be nonempty and <= 16 MiB")
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise MovementError("input must be ASCII") from error
    if not text.endswith("\n"):
        raise MovementError("input requires final LF")
    lines = text[:-1].split("\n")
    if len(lines) > 4096:
        raise MovementError("scenario count exceeds 4096")
    cases, seen, total = [], set(), 0
    for number, line in enumerate(lines, 1):
        fields = line.split("\t")
        try:
            if len(fields) != 6:
                raise MovementError("expected six TSV columns")
            case_id, mode, initial, boxes, surface, frames_raw = fields
            if not re.fullmatch(r"[A-Za-z0-9_.:-]{1,96}", case_id) or case_id in seen:
                raise MovementError("invalid or duplicate ID")
            if mode not in {"move", "travel"} or surface not in {"0", "1", "79"}:
                raise MovementError("invalid mode or surface")
            state = parse_state(initial, initial=True)
            colliders = [] if boxes == "-" else boxes.split(":")
            if len(colliders) > 128:
                raise MovementError("collider count exceeds 128")
            for box in colliders:
                parts = box.split(",")
                if len(parts) != 6:
                    raise MovementError("box requires six components")
                coordinates = [bounded_number(p, 64, -30_000_000, 30_000_000) for p in parts]
                if any(coordinates[i] >= coordinates[i + 3] for i in range(3)):
                    raise MovementError("box extents must be positive")
            frames = [f.split(",") for f in frames_raw.split(";")]
            if not 1 <= len(frames) <= 256:
                raise MovementError("frame count outside 1..256")
            for f in frames:
                if mode == "move":
                    if len(f) != 3:
                        raise MovementError("move frame requires three components")
                    for p in f:
                        bounded_number(p, 64, -32, 32)
                else:
                    if len(f) != 5 or any(x not in {"0", "1"} for x in f[3:]):
                        raise MovementError("travel frame requires headings/yaw and two boolean flags")
                    bounded_number(f[0], 32, -1, 1)
                    bounded_number(f[1], 32, -1, 1)
                    bounded_number(f[2], 32, -36000, 36000)
            seen.add(case_id)
            total += len(frames)
            if total > 100_000:
                raise MovementError("total state count exceeds 100000")
            cases.append({"id": case_id, "mode": mode, "initial": state,
                          "boxes": colliders, "surface": int(surface), "frames": frames})
        except MovementError as error:
            raise MovementError(f"row {number}: {error}") from error
    return cases


def output_keys(cases):
    return [(case["id"], index, case["mode"]) for case in cases for index in range(len(case["frames"]))]


def parse_output(data, cases, label):
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise MovementError(f"{label} is not ASCII") from error
    if not text.endswith("\n"):
        raise MovementError(f"{label} requires final LF")
    lines, keys = text[:-1].split("\n"), output_keys(cases)
    if len(lines) != len(keys):
        raise MovementError(f"{label} state count mismatch")
    states = []
    for row, key in zip(lines, keys):
        fields = row.split("\t")
        if len(fields) != 3 or fields[:2] != [key[0], str(key[1])]:
            raise MovementError(f"{label} malformed or out of order at {key[:2]}")
        parse_state(fields[2])
        states.append(fields[2])
    return states


def compare(reference, actual, cases):
    a, b = parse_output(reference, cases, "native trace"), parse_output(actual, cases, "Rust trace")
    mismatches = []
    for row, (key, left, right) in enumerate(zip(output_keys(cases), a, b)):
        if left != right:
            component = next(i for i, (x, y) in enumerate(zip(left.split(","), right.split(","))) if x != y)
            mismatches.append({"record": row, "case": key[0], "frame": key[1], "mode": key[2],
                               "component": component, "reference": left, "rust": right})
    return {"mismatch_count": len(mismatches), "first_mismatch": mismatches[0] if mismatches else None}


def primary_invocations(cases, states):
    counts, offset = Counter(), 0
    for case in cases:
        counts[case["mode"]] += len(case["frames"])
        grounded = bool(int(case["initial"][-1]) & 1)
        for f, state in zip(case["frames"], states[offset:offset + len(case["frames"])]):
            if case["mode"] == "travel" and f[3] == "1" and grounded:
                counts["jump"] += 1
            grounded = bool(int(state.split(",")[-1]) & 1)
        offset += len(case["frames"])
    return dict(counts)


def native_hashes(jar):
    if sha256(jar.read_bytes()) != JAR_SHA:
        raise MovementError("unexpected original client JAR hash")
    with zipfile.ZipFile(jar) as archive:
        return {name: sha256(archive.read(name + ".class")) for name in NATIVE_CLASSES}


def validate_pinned(root, pinned):
    for key, value in (("jar_sha256", JAR_SHA), ("oracle_sha256", sha256((root / ORACLE).read_bytes())),
                       ("input_sha256", sha256((root / FIXTURES / "input.tsv").read_bytes())),
                       ("output_sha256", sha256((root / FIXTURES / "reference.tsv").read_bytes()))):
        if pinned.get(key) != value:
            raise MovementError("committed movement provenance mismatch: " + key)
    if pinned.get("repeat_output_sha256") != pinned["output_sha256"]:
        raise MovementError("committed repeat output hash mismatch")


def validate_audit(audit, cases, states, pinned, jar):
    from urllib.parse import unquote, urlparse
    if audit.get("schema_version") != 1 or audit.get("suite") != "movement":
        raise MovementError("movement audit schema mismatch")
    if audit.get("jar_sha256") != JAR_SHA or audit.get("class_sha256") != native_hashes(jar):
        raise MovementError("native JAR/class audit mismatch")
    if audit["class_sha256"] != pinned.get("class_sha256"):
        raise MovementError("native classes differ from pinned manifest")
    for key in ("method_mapping", "field_mapping", "surfaces"):
        if not pinned.get(key) or audit.get(key) != pinned[key]:
            raise MovementError("native " + key + " differs from pinned manifest")
    if set(audit["method_mapping"]) != {"move", "travel", "jump"}:
        raise MovementError("native method map incomplete")
    if audit.get("invocations") != primary_invocations(cases, states):
        raise MovementError("native primary invocation counts disagree with scenarios/states")
    if audit.get("case_count") != len(cases) or audit.get("state_count") != len(states):
        raise MovementError("native case/state count mismatch")
    if (type(audit.get("case_count")) is not int or type(audit.get("state_count")) is not int
            or any(type(v) is not int or v <= 0 for v in audit.get("invocations", {}).values())):
        raise MovementError("native counters require integer values")
    if (audit.get("environment") != "synthetic-static-dry" or audit.get("native_world_constructors") != 0
            or type(audit.get("collision_queries")) is not int or audit["collision_queries"] < 0):
        raise MovementError("native synthetic environment audit mismatch")
    sources = audit.get("code_sources", {})
    if set(sources) != set(NATIVE_CLASSES):
        raise MovementError("native code source audit missing classes")
    for value in sources.values():
        uri = urlparse(value)
        if uri.scheme != "file" or uri.netloc or Path(unquote(uri.path)).resolve() != jar:
            raise MovementError("native class code source differs from original JAR")


def source_snapshot(root=ROOT):
    fixed = {Path("Cargo.toml"), Path("Cargo.lock"), Path("rust-toolchain.toml"),
             Path(".cargo/config"), Path(".cargo/config.toml"), Path("config/local.toml"),
             Path("tools/oracle/movement_corpus.py"), ORACLE, PROBE}
    for folder, patterns in (("crates/beta-core", ("*.rs", "Cargo.toml")),
                             ("tools/parity", ("*.py",)), (str(FIXTURES), ("*",))):
        for pattern in patterns:
            fixed.update(p.relative_to(root) for p in (root / folder).rglob(pattern) if p.is_file())
    return {str(p): sha256((root / p).read_bytes()) if (root / p).is_file() else None for p in sorted(fixed)}


def expected_commands(root, directory, jar):
    classes = directory / "classes"
    commands = {"git-revision": ["git", "rev-parse", "HEAD"], "git-dirty": ["git", "status", "--porcelain"],
                "java-build": ["javac", "-encoding", "UTF-8", "-cp", str(jar), "-d", str(classes), str(root / ORACLE)],
                "rust-build": ["cargo", "build", "--locked", "--offline", "-p", "beta-core", "--example", EXAMPLE,
                               "--message-format=json-render-diagnostics"], "rust-run": [str(directory / EXAMPLE)]}
    for tool in ("java", "javac", "rustc", "cargo"):
        commands["version-" + tool] = [tool, "-version" if tool in {"java", "javac"} else "--version"]
    for label in ("java-run", "java-repeat"):
        commands[label] = ["java", "-XX:-UsePerfData", "-cp", f"{classes}:{jar}", "MovementOracle", str(jar),
                           str(directory / (label + "-audit.json"))]
    return commands


def run_movement_suite(root=ROOT, timeout=60, input_path=None):
    root, started = Path(root).resolve(), time.monotonic()
    parent = artifacts_root(root) / "parity/movement"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    path = directory / "report.json"
    report = {"schema_version": 1, "suite": "movement", "status": "error", "reason": "comparison not completed",
              "scenario": "custom" if input_path else "pinned", "environment": "synthetic-static-dry",
              "comparison_policy": "exact ordered per-frame raw-bit states", "native_world_constructors": 0,
              "started_at": datetime.now(timezone.utc).isoformat(), "processes": {}, "log_paths": {},
              "mismatch_count": None, "first_mismatch": None, "source_hashes": source_snapshot(root)}

    def execute(label, data=None):
        p = run_process(commands[label], cwd=root, directory=directory, label=label, timeout=timeout, input_data=data)
        report["processes"][label] = p
        report["log_paths"][label] = {k: p[k] for k in ("stdout", "stderr")}
        if p["error"] or p["returncode"] != 0:
            raise MovementError(f"{label}: {p['error'] or 'exit ' + str(p['returncode'])}")
        return Path(p["stdout"]).read_bytes()

    try:
        jar = jar_path(root)
        commands = expected_commands(root, directory, jar)
        report.update(jar_path=str(jar), jar_sha256=JAR_SHA, native_class_sha256=native_hashes(jar))
        pinned = json.loads((root / FIXTURES / "audit.json").read_text())
        validate_pinned(root, pinned)
        raw = Path(input_path).read_bytes() if input_path else (root / FIXTURES / "input.tsv").read_bytes()
        cases = parse_input(raw)
        (directory / "input.tsv").write_bytes(raw)
        report["log_paths"]["input"] = str(directory / "input.tsv")
        report.update(case_count=len(cases), state_count=len(output_keys(cases)), input_sha256=sha256(raw))
        report["rust_revision"] = execute("git-revision").decode().strip()
        report["rust_dirty"] = bool(execute("git-dirty"))
        report["runtime_versions"] = {}
        for tool in ("java", "javac", "rustc", "cargo"):
            execute("version-" + tool)
            p = report["processes"]["version-" + tool]
            report["runtime_versions"][tool] = (Path(p["stdout"]).read_bytes() + Path(p["stderr"]).read_bytes()).decode().strip()
        classes = directory / "classes"
        classes.mkdir()
        execute("java-build")
        report["log_paths"]["java_classes"] = str(classes)
        report["adapter_class_hashes"] = {str(p.relative_to(classes)): sha256(p.read_bytes()) for p in classes.rglob("*.class")}
        outputs, audits = [], []
        for label in ("java-run", "java-repeat"):
            result = execute(label, raw)
            states = parse_output(result, cases, label)
            audit_path = directory / (label + "-audit.json")
            report["log_paths"][label + "-audit"] = str(audit_path)
            audit_bytes = audit_path.read_bytes()
            report[label + "-audit-sha256"] = sha256(audit_bytes)
            audit = json.loads(audit_bytes)
            validate_audit(audit, cases, states, pinned, jar)
            registry = Path(str(audit_path) + ".registry.stdout")
            capture = registry.read_bytes()
            if sha256(capture) != audit.get("registry_stdout_sha256") or len(capture) != audit.get("registry_stdout_bytes"):
                raise MovementError("retained native bootstrap stdout differs from audit")
            report["log_paths"][label + "-registry"] = str(registry)
            outputs.append(result)
            audits.append(audit)
        if outputs[0] != outputs[1] or audits[0] != audits[1]:
            raise MovementError("native movement trace/audit not repeatable")
        if not input_path and outputs[0] != (root / FIXTURES / "reference.tsv").read_bytes():
            raise MovementError("native trace differs from pinned fixture")
        report.update(reference_output_sha256=sha256(outputs[0]), reference_repeat_output_sha256=sha256(outputs[1]))
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust movement_probe absent; two native traces executed")
        else:
            records = [json.loads(line) for line in execute("rust-build").splitlines()]
            binaries = [Path(r["executable"]) for r in records if r.get("reason") == "compiler-artifact"
                        and r.get("target", {}).get("name") == EXAMPLE and r.get("executable")]
            if len(binaries) != 1:
                raise MovementError("cargo did not identify exactly one movement probe")
            binary = directory / EXAMPLE
            shutil.copy2(binaries[0], binary)
            report["log_paths"]["rust_binary"] = str(binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            actual = execute("rust-run", raw)
            report["rust_output_sha256"] = sha256(actual)
            report.update(compare(outputs[0], actual, cases))
            report.update(status="fail" if report["mismatch_count"] else "pass",
                          reason="movement state mismatch" if report["mismatch_count"] else "all native per-frame movement states match exactly")
        if native_hashes(jar) != report["native_class_sha256"]:
            raise MovementError("original JAR changed during comparison")
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        report.update(status="error", reason=str(error))
    finally:
        report["source_hashes_after"] = source_snapshot(root)
        if report["source_hashes"] != report["source_hashes_after"]:
            report.update(status="error", reason="sources changed during comparison")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, path


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--input", type=Path)
    p.add_argument("--timeout", type=float, default=60)
    p.add_argument("--strict", action="store_true")
    args = p.parse_args()
    if not 0 < args.timeout < float("inf"):
        p.error("--timeout must be positive and finite")
    report, path = run_movement_suite(timeout=args.timeout, input_path=args.input)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return exit_code(report["status"], args.strict)


if __name__ == "__main__":
    raise SystemExit(main())
