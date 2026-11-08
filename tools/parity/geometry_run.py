"""Exact native Beta geometry comparisons, including custom agent-supplied cases."""
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
import tomllib
from urllib.parse import unquote, urlparse
import zipfile

from run import ROOT, artifacts_root, exit_code, run_process, sha256

JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
FIXTURES = Path("tests/fixtures/geometry")
ORACLE = Path("tools/oracle/src/GeometryOracle.java")
PROBE = Path("crates/beta-core/examples/geometry_probe.rs")
EXAMPLE = "geometry_probe"
SPECS = {
    "box_new": (6, "box"), "box_sweep": (9, "box"),
    "box_expand": (9, "box"), "box_offset": (9, "box"),
    "box_clip_x": (13, "scalar"), "box_clip_y": (13, "scalar"),
    "box_clip_z": (13, "scalar"), "box_intersects": (12, "boolean"),
    "box_contains": (9, "boolean"), "box_average_edge": (6, "scalar"),
    "box_ray": (12, "ray"), "vec_new": (3, "vector"),
    "vec_to": (6, "vector"), "vec_cross": (6, "vector"),
    "vec_add": (6, "vector"), "vec_normalize": (3, "vector"),
    "vec_length": (3, "scalar"), "vec_distance": (6, "scalar"),
    "vec_distance_sq": (6, "scalar"), "vec_intermediate_x": (7, "optional"),
    "vec_intermediate_y": (7, "optional"), "vec_intermediate_z": (7, "optional"),
}
REQUIRED_PROCESSES = ("git-revision", "git-dirty", "version-java", "version-javac",
                      "version-rustc", "version-cargo", "java-build", "java-run",
                      "java-repeat", "rust-build", "rust-run")


class GeometryError(ValueError):
    """Malformed cases, output, audit or execution evidence."""


def finite_hex(value):
    return (re.fullmatch(r"[0-9a-f]{16}", value) is not None
            and abs(struct.unpack(">d", bytes.fromhex(value))[0]) <= 30_000_000)


def parse_input(data):
    if not data or len(data) > 32 * 1024 * 1024:
        raise GeometryError("geometry input is empty or exceeds 32 MiB")
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise GeometryError("geometry input must be ASCII") from error
    if not text.endswith("\n"):
        raise GeometryError("geometry input requires final LF")
    lines = text[:-1].split("\n")
    if len(lines) > 100_000:
        raise GeometryError("geometry input exceeds 100000 rows")
    cases, seen = [], set()
    for number, line in enumerate(lines, 1):
        fields = line.split("\t")
        if len(fields) < 3 or not re.fullmatch(r"[A-Za-z0-9_.:-]{1,96}", fields[0]):
            raise GeometryError(f"invalid ID or framing at row {number}")
        case_id, op, *args = fields
        if case_id in seen or op not in SPECS:
            raise GeometryError(f"duplicate ID or unknown operation at row {number}")
        if len(args) != SPECS[op][0] or not all(finite_hex(arg) for arg in args):
            raise GeometryError(f"bad arity, bits or finite-domain argument at row {number}")
        seen.add(case_id)
        cases.append({"id": case_id, "op": op, "args": args})
    return cases


def valid_output(kind, value):
    if kind == "boolean":
        return value in {"0", "1"}
    if value == "none":
        return kind in {"optional", "ray"}
    fields = value.split(",")
    if kind == "ray":
        if len(fields) != 4 or fields.pop(0) not in {"0", "1", "2", "3", "4", "5"}:
            return False
    count = {"scalar": 1, "box": 6, "vector": 3, "optional": 3, "ray": 3}[kind]
    return len(fields) == count and all(re.fullmatch(r"[0-9a-f]{16}", x) for x in fields)


def parse_output(data, cases, label):
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise GeometryError(f"{label} must be ASCII") from error
    if not text.endswith("\n"):
        raise GeometryError(f"{label} requires final LF")
    lines = text[:-1].split("\n")
    if len(lines) != len(cases):
        raise GeometryError(f"{label} row count differs from input")
    values = []
    for index, (line, case) in enumerate(zip(lines, cases)):
        fields = line.split("\t")
        if (len(fields) != 2 or fields[0] != case["id"]
                or not valid_output(SPECS[case["op"]][1], fields[1])):
            raise GeometryError(f"{label} malformed or out of order at row {index}")
        values.append(fields[1])
    return values


def compare(reference, actual, cases):
    expected = parse_output(reference, cases, "native output")
    observed = parse_output(actual, cases, "Rust output")
    mismatches = [{"record": i, "case": c["id"], "operation": c["op"],
                   "reference": a, "rust": b}
                  for i, (c, a, b) in enumerate(zip(cases, expected, observed)) if a != b]
    return {"mismatch_count": len(mismatches),
            "first_mismatch": mismatches[0] if mismatches else None}


def config(root):
    local = root / "config/local.toml"
    if not local.is_file():
        local = root / "config/local.example.toml"
    return tomllib.loads(local.read_text())["paths"]


def jar_path(root):
    return (root / config(root)["client_jar"]).resolve(strict=True)


def native_hashes(jar):
    if sha256(jar.read_bytes()) != JAR_SHA:
        raise GeometryError("unexpected original client JAR hash")
    with zipfile.ZipFile(jar) as archive:
        return {name: sha256(archive.read(name + ".class")) for name in ("eq", "bt", "vf", "in")}


def validate_audit(audit, cases, pinned, jar):
    if audit.get("schema_version") != 1 or audit.get("suite") != "geometry":
        raise GeometryError("native audit schema mismatch")
    if audit.get("jar_sha256") != JAR_SHA or audit.get("class_sha256") != native_hashes(jar):
        raise GeometryError("native audit JAR/class hash mismatch")
    if audit["class_sha256"] != pinned.get("class_sha256"):
        raise GeometryError("native classes differ from committed audit")
    if set(pinned.get("method_mapping", {})) != set(SPECS):
        raise GeometryError("committed native method mapping is incomplete")
    if audit.get("method_mapping") != pinned["method_mapping"]:
        raise GeometryError("resolved native method mapping differs from committed audit")
    if audit.get("case_count") != len(cases) or audit.get("invocations") != dict(Counter(c["op"] for c in cases)):
        raise GeometryError("native invocation counts differ from actual cases")
    code_sources = audit.get("code_sources", {})
    if set(code_sources) != set(audit["class_sha256"]):
        raise GeometryError("native code sources missing")
    for uri in code_sources.values():
        source = urlparse(uri)
        if source.scheme != "file" or source.netloc or Path(unquote(source.path)).resolve() != jar:
            raise GeometryError("native method was not loaded from configured JAR")


def validate_pinned(root, pinned):
    require_hashes = {"jar_sha256": JAR_SHA,
                      "oracle_sha256": sha256((root / ORACLE).read_bytes()),
                      "input_sha256": sha256((root / FIXTURES / "input.tsv").read_bytes()),
                      "output_sha256": sha256((root / FIXTURES / "reference.tsv").read_bytes())}
    if any(pinned.get(k) != v for k, v in require_hashes.items()):
        raise GeometryError("committed corpus provenance hash mismatch")
    if pinned.get("repeat_output_sha256") != pinned["output_sha256"]:
        raise GeometryError("committed corpus repeat hash mismatch")


def source_snapshot(root):
    fixed = {ORACLE, PROBE, Path("Cargo.toml"), Path("Cargo.lock"),
             Path("rust-toolchain.toml"), Path("tools/parity/run.py"),
             Path("tools/parity/geometry_run.py"), Path("tools/parity/geometry_review.py"),
             Path("tools/parity/verify_report.py"), Path("tools/oracle/geometry_corpus.py"),
             Path(".cargo/config"), Path(".cargo/config.toml")}
    for folder, pattern in (("crates/beta-core", "*.rs"), (str(FIXTURES), "*")):
        fixed.update(p.relative_to(root) for p in (root / folder).rglob(pattern) if p.is_file())
    for pattern in ("Cargo.toml", "build.rs"):
        fixed.update(p.relative_to(root) for p in (root / "crates").rglob(pattern))
    return {str(p): sha256((root / p).read_bytes()) if (root / p).is_file() else None for p in sorted(fixed)}


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
        commands[label] = ["java", "-XX:-UsePerfData", "-cp", f"{classes}:{jar}", "GeometryOracle", str(jar),
                           str(directory / (label + "-audit.json"))]
    return commands


def run_geometry_suite(root=ROOT, timeout=60, input_path=None):
    root = Path(root).resolve()
    started = time.monotonic()
    parent = artifacts_root(root) / "parity/geometry"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {"schema_version": 1, "suite": "geometry", "status": "error",
              "reason": "comparison not completed", "processes": {}, "log_paths": {},
              "scenario": "custom" if input_path else "pinned",
              "started_at": datetime.now(timezone.utc).isoformat(),
              "comparison_policy": "exact ordered TSV, raw f64 bits", "mismatch_count": None,
              "first_mismatch": None}
    before = source_snapshot(root)
    report["source_hashes"] = before

    def execute(label, command, data=None):
        proc = run_process(command, cwd=root, directory=directory, label=label,
                           timeout=timeout, input_data=data)
        report["processes"][label] = proc
        report["log_paths"][label] = {k: proc[k] for k in ("stdout", "stderr")}
        if proc["error"] or proc["returncode"] != 0:
            raise GeometryError(f"{label}: {proc['error'] or 'exit ' + str(proc['returncode'])}")
        return Path(proc["stdout"]).read_bytes()

    try:
        jar = jar_path(root)
        report.update(jar_path=str(jar), jar_sha256=sha256(jar.read_bytes()))
        report["native_class_sha256"] = native_hashes(jar)
        pinned = json.loads((root / FIXTURES / "audit.json").read_text())
        validate_pinned(root, pinned)
        raw = Path(input_path).read_bytes() if input_path else (root / FIXTURES / "input.tsv").read_bytes()
        cases = parse_input(raw)
        (directory / "input.tsv").write_bytes(raw)
        report["log_paths"]["input"] = str(directory / "input.tsv")
        report.update(input_sha256=sha256(raw), case_count=len(cases),
                      operation_counts=dict(Counter(c["op"] for c in cases)))
        report["rust_revision"] = execute("git-revision", ["git", "rev-parse", "HEAD"]).decode().strip()
        report["rust_dirty"] = bool(execute("git-dirty", ["git", "status", "--porcelain"]))
        report["runtime_versions"] = {}
        for tool in ("java", "javac", "rustc", "cargo"):
            execute("version-" + tool, [tool, "-version" if tool in {"java", "javac"} else "--version"])
            p = report["processes"]["version-" + tool]
            report["runtime_versions"][tool] = (Path(p["stdout"]).read_bytes() + Path(p["stderr"]).read_bytes()).decode().strip()
        classes = directory / "classes"
        classes.mkdir()
        execute("java-build", ["javac", "-encoding", "UTF-8", "-cp", str(jar), "-d", str(classes), str(root / ORACLE)])
        report["adapter_class_hashes"] = {str(p.relative_to(classes)): sha256(p.read_bytes()) for p in classes.rglob("*.class")}
        report["log_paths"]["java_classes"] = str(classes)
        outputs, audits = [], []
        for label in ("java-run", "java-repeat"):
            audit_path = directory / (label + "-audit.json")
            report["log_paths"][label + "-audit"] = str(audit_path)
            output = execute(label, ["java", "-XX:-UsePerfData", "-cp", f"{classes}:{jar}",
                                     "GeometryOracle", str(jar), str(audit_path)], raw)
            parse_output(output, cases, label)
            audit = json.loads(audit_path.read_text())
            validate_audit(audit, cases, pinned, jar)
            report[label + "-audit-sha256"] = sha256(audit_path.read_bytes())
            outputs.append(output)
            audits.append(audit)
        reference, repeated = outputs
        if reference != repeated or audits[0] != audits[1]:
            raise GeometryError("native output or audit is not repeatable")
        if not input_path and reference != (root / FIXTURES / "reference.tsv").read_bytes():
            raise GeometryError("native output differs from committed fixture")
        report.update(reference_output_sha256=sha256(reference), reference_repeat_output_sha256=sha256(repeated))
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust geometry_probe is absent; native oracle executed twice")
        else:
            build = execute("rust-build", ["cargo", "build", "--locked", "--offline", "-p", "beta-core", "--example", EXAMPLE,
                                           "--message-format=json-render-diagnostics"])
            binaries = []
            for line in build.splitlines():
                record = json.loads(line)
                if record.get("reason") == "compiler-artifact" and record.get("target", {}).get("name") == EXAMPLE and record.get("executable"):
                    binaries.append(Path(record["executable"]))
            if len(binaries) != 1:
                raise GeometryError("cargo did not identify one geometry probe")
            binary = directory / EXAMPLE
            shutil.copy2(binaries[0], binary)
            report["log_paths"]["rust_binary"] = str(binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            actual = execute("rust-run", [str(binary)], raw)
            report["rust_output_sha256"] = sha256(actual)
            report.update(compare(reference, actual, cases))
            report.update(status="fail" if report["mismatch_count"] else "pass",
                          reason="output mismatch" if report["mismatch_count"] else "all native geometry records match exactly")
        if native_hashes(jar) != report["native_class_sha256"]:
            raise GeometryError("original JAR changed during comparison")
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        report.update(status="error", reason=str(error))
    finally:
        report["source_hashes_after"] = source_snapshot(root)
        if before != report["source_hashes_after"]:
            report.update(status="error", reason="sources changed during comparison")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, help="custom synthetic TSV, does not rewrite pinned fixtures")
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--timeout", type=float, default=60)
    args = parser.parse_args()
    if not 0 < args.timeout < float("inf"):
        parser.error("--timeout must be positive and finite")
    report, path = run_geometry_suite(timeout=args.timeout, input_path=args.input)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return exit_code(report["status"], args.strict)


if __name__ == "__main__":
    raise SystemExit(main())
