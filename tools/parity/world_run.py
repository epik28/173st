"""WORLD-LOAD-001 live oracle/probe runner. All world reads use two copies."""

from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import time
import tomllib
import tempfile

import run as parity

ROOT = parity.ROOT
FIXTURE = Path("tests/fixtures/world/live.tsv")
ORACLE = Path("tools/oracle/src/WorldOracle.java")
PROBE = Path("crates/beta-core/examples/world_probe.rs")
JAR_SHA256 = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASS_SHA256 = {
    "ei": "5d01d1dda7be346c57fd00846d2425615186c3ef1c65c06aa0bd0f45aa9f7fef",
    "nu": "a1a5701ec2180d01ddc604cc8c2e5aff1176b10faf83bdb64d2f1cc8c806ac4a",
    "ij": "9db862e2ee83e95a2db7a8495594d2c0e0bbe904d96c3f09f167f2b47b72b8d0",
    "qj": "6c75a123b63d555caa98e1a2619aa2483e405566de9e356585b1dc682a7da4a6",
}
MARK_WORLD = "WORLD-LOAD-001 copied inputs\n"
MARK_REGION = "REGION-001 copied inputs\n"
FIELDS_LEVEL = 21
FIELDS_CHUNK = 17


class WorldRunError(parity.RunError):
    pass


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def parse_world_fixture(data):
    """Validate five-field TSV, strict names/types, limits and -0 coordinates."""
    cases, ids = [], set()
    for number, line in enumerate(data.decode("utf-8").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 5:
            raise WorldRunError(f"world fixture line {number}: expected exactly five fields")
        case_id, kind, filename, x_text, z_text = fields
        if not re.fullmatch(r"[A-Za-z0-9_-]+", case_id) or case_id in ids:
            raise WorldRunError(f"world fixture line {number}: invalid or duplicate ID")
        if kind == "level":
            if (filename, x_text, z_text) != ("level.dat", "-", "-"):
                raise WorldRunError(f"world fixture line {number}: invalid level row")
            case = {"id": case_id, "kind": "level", "file": filename}
        elif kind == "chunk":
            if not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", x_text) or not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", z_text):
                raise WorldRunError(f"world fixture line {number}: invalid coordinate syntax")
            x, z = int(x_text), int(z_text)
            if not -(2**31) <= x < 2**31 or not -(2**31) <= z < 2**31:
                raise WorldRunError(f"world fixture line {number}: coordinate outside i32")
            expected = f"r.{x // 32}.{z // 32}.mcr"
            if filename != expected:
                raise WorldRunError(f"world fixture line {number}: region basename mismatch")
            case = {"id": case_id, "kind": "chunk", "file": filename, "x": x, "z": z}
        else:
            raise WorldRunError(f"world fixture line {number}: unknown row kind")
        ids.add(case_id)
        cases.append(case)
        if len(cases) > 100000:
            raise WorldRunError("world case limit 100000 exceeded")
    if not cases:
        raise WorldRunError("world fixture contains no cases")
    return cases


def world_keys(cases):
    return [(case["id"], case["kind"], case.get("x"), case.get("z")) for case in cases]


def _valid_int(field, signed=True, bits=64):
    pattern = rb"(?:0|-[1-9][0-9]*|[1-9][0-9]*)" if signed else rb"(?:0|[1-9][0-9]*)"
    if not re.fullmatch(pattern, field):
        return False
    number = int(field)
    return (-(2**(bits-1)) <= number < 2**(bits-1)) if signed else number < 2**bits


def _valid_name_set(field):
    if field == b"-":
        return True
    parts = field.split(b",")
    return (all(p == b"~" or re.fullmatch(rb"(?:[0-9a-f]{2})+", p) for p in parts)
            and parts == sorted(parts) and len(parts) == len(set(parts)))


def valid_world_record(line, key):
    if key is None or line is None or not line.endswith(b"\n") or b"\r" in line:
        return False
    fields = line[:-1].split(b"\t")
    case_id, kind = key[:2]
    if len(fields) != (FIELDS_LEVEL if kind == "level" else FIELDS_CHUNK) or fields[0] != case_id.encode("ascii") or fields[1] != kind.encode("ascii"):
        return False
    if kind == "level":
        return (all(_valid_int(fields[i], bits=64) for i in (2, 6, 7, 8))
                and all(_valid_int(fields[i], bits=32) for i in (3, 4, 5, 9, 11, 12, 14))
                and (fields[10] == b"-" or bool(re.fullmatch(rb"(?:[0-9a-f]{2})+", fields[10])))
                and fields[13] in (b"0", b"1") and fields[15] in (b"0", b"1")
                and fields[16] in (b"0", b"1") and _valid_int(fields[17], signed=False, bits=32)
                and _valid_name_set(fields[18]) and _valid_int(fields[19], signed=False, bits=32)
                and bool(re.fullmatch(rb"[0-9a-f]{64}", fields[20])))
    coords_ok = (all(_valid_int(fields[i], bits=32) for i in (3, 4))
                 and int(fields[3]) == key[2] and int(fields[4]) == key[3])
    if fields[2] == b"absent":
        return coords_ok and fields[5:] == [b"-"] * 12
    return (fields[2] == b"present" and coords_ok
            and _valid_int(fields[5], bits=64) and fields[6] in (b"0", b"1")
            and all(re.fullmatch(rb"[0-9a-f]{64}", fields[i]) for i in range(7, 12))
            and _valid_int(fields[12], signed=False, bits=32)
            and _valid_int(fields[13], signed=False, bits=32) and _valid_name_set(fields[14])
            and _valid_int(fields[15], signed=False, bits=32)
            and bool(re.fullmatch(rb"[0-9a-f]{64}", fields[16])))


def compare_world_outputs(reference, actual, cases):
    left, right = reference.splitlines(keepends=True), actual.splitlines(keepends=True)
    keys = world_keys(cases)
    mismatches, first = 0, None
    # Iterate to max length so unexpected/extra rows are counted as mismatches.
    for index in range(max(len(keys), len(left), len(right))):
        expected = left[index] if index < len(left) else None
        observed = right[index] if index < len(right) else None
        key = keys[index] if index < len(keys) else None
        if expected != observed or not valid_world_record(expected, key) or not valid_world_record(observed, key):
            mismatches += 1
            if first is None:
                first = {"record": index, "case": None if key is None else key[0],
                         "reference": None if expected is None else expected.decode("utf-8", "replace"),
                         "rust": None if observed is None else observed.decode("utf-8", "replace")}
    return {"mismatch_count": mismatches, "first_mismatch": first}


def source_snapshot(root):
    paths = {FIXTURE, ORACLE, Path("tools/oracle/world_corpus.py"), Path("tools/parity/run.py"),
             Path("tools/parity/world_run.py"), Path("tools/parity/world_review.py"),
             Path("tools/parity/verify_report.py"), Path("tests/fixtures/world/sources.json"),
             Path("config/local.toml"), Path("Cargo.toml"), Path("Cargo.lock"), Path("rust-toolchain.toml"),
             Path("crates/beta-core/examples/world_probe.rs")}
    for folder, pattern in (("tools/parity", "*.py"), ("tools/oracle", "*.java"),
                            ("tests/fixtures/world", "*.json"), ("tests/fixtures/world", "*.md")):
        paths.update(p.relative_to(root) for p in (root / folder).rglob(pattern))
    paths.update(p.relative_to(root) for p in (root / "crates").rglob("*.rs"))
    paths.update(p.relative_to(root) for p in (root / "crates").rglob("Cargo.toml"))
    return {str(p): (sha256((root / p).read_bytes()) if (root / p).is_file() else None) for p in sorted(paths)}


def _load_corpus(root):
    spec = importlib.util.spec_from_file_location("world_corpus_for_parity", root / "tools/oracle/world_corpus.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _hash_sources(corpus, root):
    _, sources = corpus.source_paths(root)
    return {name: sha256(path.read_bytes()) for name, path in sources.items()}


def _check_copy(path, expected):
    path = Path(path)
    if path.is_symlink():
        raise WorldRunError("input copy directory must not be a symlink")
    path = path.resolve(strict=True)
    expected_names = set(expected) | {".world-input-copy", ".region-input-copy"}
    entries = list(path.iterdir())
    if ({item.name for item in entries} != expected_names
            or any(item.is_symlink() or not item.is_file() for item in entries)):
        raise WorldRunError("input copy contains unexpected entries or symlinks")
    for marker, contents in ((".world-input-copy", MARK_WORLD), (".region-input-copy", MARK_REGION)):
        item = path / marker
        if item.is_symlink() or not item.is_file() or item.read_text() != contents:
            raise WorldRunError(f"missing/invalid marker in input copy: {marker}")
    actual = {}
    for name, digest in expected.items():
        item = path / name
        if Path(name).name != name or item.is_symlink() or not item.is_file() or item.resolve(strict=True).parent != path:
            raise WorldRunError(f"unsafe/missing copied input: {name}")
        actual[name] = sha256(item.read_bytes())
    if actual != expected:
        raise WorldRunError("input copy hashes differ")
    return actual


def _process(directory, processes, label, command, *, cwd, timeout, input_data=None):
    result = parity.run_process(command, cwd=cwd, directory=directory, label=label,
                                timeout=timeout, input_data=input_data)
    processes[label] = result
    if result["error"] or result["returncode"] != 0:
        raise WorldRunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
    return Path(result["stdout"]).read_bytes()


def _report_process_logs(report, directory):
    report["log_paths"].update({label: {"stdout": item["stdout"], "stderr": item["stderr"]}
                                for label, item in report["processes"].items()})


def run_world(root=ROOT, timeout=60):
    corpus = _load_corpus(root)
    parent = parity.artifacts_root(root) / "parity/world"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {"schema_version": 1, "suite": "world", "scenario_id": "WORLD-LOAD-001-read-copies",
              "started_at": datetime.now(timezone.utc).isoformat(), "status": "error",
              "reason": "comparison not completed", "reference_side": "Original Beta 1.7.3 ij/nu/ei/qj",
              "reference_version": None, "reference_jar_path": None, "reference_jar_sha256": None,
              "reference_class_sha256": dict(CLASS_SHA256),
              "scope": "native NBT data extraction + ei metadata constructor + qj region reads; native World/chunk loads=0",
              "native_semantics": {"metadata_constructor": "ei(nu)", "nbt_extraction": "ij.b(DataInput) + nu fields",
                                   "region_read": "qj.a(int,int)", "full_world_load": False, "native_chunk_loads": 0},
              "comparison_policy": "exact ordered typed TSV, native array/raw payload SHA-256 and byte lengths",
              "platform": platform.platform(), "runtime_versions": {}, "toolchain": {},
              "rust_revision": None, "rust_dirty": None, "rust_binary_sha256": None,
              "adapter_class_sha256": None, "source_hashes": {}, "source_hashes_after": {},
              "source_world_hashes": {}, "source_world_hashes_after": {},
              "input_copy_hashes": {}, "rust_input_copy_hashes": {}, "input_copy_hashes_after": {},
              "input_sha256": None, "case_count": 0, "present_count": None, "absent_count": None,
              "native_cases": {"live": 0, "synthetic": 0}, "mismatch_count": None, "first_mismatch": None,
              "duration_ms": None, "reference_output_sha256": None, "rust_output_sha256": None,
              "retained_files": {}, "processes": {}, "log_paths": {"input": str(directory / "input.tsv"),
                                                                        "report": str(report_path)}}
    initial_sources = None
    evidence = None
    input_dir = None
    jar = None
    jar_hash = None
    failure = None
    started = time.monotonic()
    try:
        cfg = tomllib.loads((root / "config/local.toml").read_text())["paths"]
        jar = (root / cfg["client_jar"]).resolve(strict=True)
        jar_hash = sha256(jar.read_bytes())
        if jar_hash != JAR_SHA256:
            raise WorldRunError("configured original client JAR has wrong SHA-256")
        world_source, source_map = corpus.source_paths(root)
        source_hashes = {name: sha256(p.read_bytes()) for name, p in source_map.items()}
        report.update(reference_jar_path=str(jar), reference_jar_sha256=jar_hash,
                      source_world_path=str(world_source), source_world_hashes=source_hashes)
        initial_sources = source_snapshot(root)
        report["source_hashes"] = initial_sources
        input_dir, evidence = corpus.snapshot(directory, root)
        (directory / "snapshot.json").replace(directory / "snapshot-evidence.json")
        report["input_copy_hashes"] = _check_copy(input_dir, source_hashes)
        fixture = (root / FIXTURE).read_bytes()
        cases = parse_world_fixture(fixture)
        if len(cases) != 749 or sum(c["kind"] == "level" for c in cases) != 1:
            raise WorldRunError("pinned world fixture must contain exactly one level and 748 chunk rows")
        (directory / "input.tsv").write_bytes(fixture)
        report.update(input_sha256=sha256(fixture), case_count=len(cases), native_cases={"live": len(cases), "synthetic": 0})

        for tool, command in (("java", ["java", "-version"]), ("javac", ["javac", "-version"]),
                              ("rustc", ["rustc", "--version"]), ("cargo", ["cargo", "--version"])):
            _process(directory, report["processes"], f"version-{tool}", command,
                     cwd=root, timeout=timeout)
            proc = report["processes"][f"version-{tool}"]
            report["runtime_versions"][tool] = (Path(proc["stdout"]).read_bytes() + Path(proc["stderr"]).read_bytes()).decode("utf-8", "replace").strip()
        report["reference_version"] = "Minecraft Beta 1.7.3 client; " + report["runtime_versions"]["java"]
        report["toolchain"] = {k: report["runtime_versions"][k] for k in ("rustc", "cargo")}

        classes = directory / "classes"
        classes.mkdir()
        _process(directory, report["processes"], "java-build",
                 ["javac", "-cp", str(jar), "-d", str(classes), str(root / ORACLE)], cwd=root, timeout=timeout)
        class_file = classes / "WorldOracle.class"
        report["adapter_class_sha256"] = sha256(class_file.read_bytes())
        classpath = f"{classes}:{jar}"
        java_command = ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", classpath, "WorldOracle", str(jar)]
        java_payloads = directory / "java-payloads"
        audit_path = directory / "native-audit.json"
        java = _process(directory, report["processes"], "java-run",
                        java_command + [str(input_dir), str(java_payloads), str(audit_path)],
                        cwd=root, timeout=timeout, input_data=fixture)
        report["reference_output_sha256"] = sha256(java)
        report["log_paths"].update({"payload_dir": str(java_payloads), "input_copy_dir": str(input_dir),
                                   "reference_invocations": str(audit_path)})
        if compare_world_outputs(java, java, cases)["mismatch_count"]:
            raise WorldRunError("original Java output violates typed TSV contract")
        audit = json.loads(audit_path.read_text())
        _validate_audit(audit, source_hashes, len(cases))
        report["reference_invocations"] = audit
        report["present_count"], report["absent_count"] = audit["chunk_present"], audit["chunk_absent"]
        report["native_calls"] = audit["native_calls"]
        report["reference_code_sources"] = audit["code_sources"]
        report["retained_files"] = _retained_manifest(java_payloads, audit)
        _bind_retained_to_rows(java_payloads, report["retained_files"], java, cases)

        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust world_probe source is absent; Java 749-row oracle executed")
        else:
            report["rust_revision"] = _process(directory, report["processes"], "git-revision",
                                                ["git", "rev-parse", "HEAD"], cwd=root, timeout=timeout).decode().strip()
            dirty = _process(directory, report["processes"], "git-dirty", ["git", "status", "--porcelain"], cwd=root, timeout=timeout)
            report["rust_dirty"] = bool(dirty)
            build_bytes = _process(directory, report["processes"], "rust-build",
                                   ["cargo", "build", "--locked", "--offline", "-p", "beta-core", "--example", "world_probe", "--message-format=json-render-diagnostics"],
                                   cwd=root, timeout=max(timeout, 120))
            binaries = []
            for line in build_bytes.splitlines():
                message = json.loads(line)
                if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "world_probe" and message.get("executable"):
                    binaries.append(Path(message["executable"]))
            if len(binaries) != 1:
                raise WorldRunError("cargo build did not yield exactly one world_probe executable")
            binary = directory / "world_probe"
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            rust_input = directory / "rust-input"
            rust_input.mkdir()
            (rust_input / ".world-input-copy").write_text(MARK_WORLD)
            (rust_input / ".region-input-copy").write_text(MARK_REGION)
            for name in source_hashes:
                with (rust_input / name).open("xb") as target:
                    target.write((input_dir / name).read_bytes())
            report["rust_input_copy_hashes"] = _check_copy(rust_input, source_hashes)
            report["log_paths"]["rust_input_copy_dir"] = str(rust_input)
            rust_payloads = directory / "rust-payloads"
            rust = _process(directory, report["processes"], "rust-run",
                            [str(binary), str(rust_input), str(rust_payloads)], cwd=root,
                            timeout=timeout, input_data=fixture)
            report["rust_output_sha256"] = sha256(rust)
            comparison = compare_world_outputs(java, rust, cases)
            report.update(comparison, status="fail" if comparison["mismatch_count"] else "pass",
                          reason="output mismatch" if comparison["mismatch_count"] else "all records match exactly")
            report["log_paths"]["rust_payload_dir"] = str(rust_payloads)
            rust_manifest = _retained_manifest(rust_payloads, audit, prefix="Rust")
            if rust_manifest != report["retained_files"]:
                raise WorldRunError("Rust retained payload filenames/bytes differ from Java")
            report["rust_retained_files"] = rust_manifest
            report["input_copy_hashes_after"] = _check_copy(input_dir, source_hashes)
            if _check_copy(rust_input, source_hashes) != source_hashes:
                raise WorldRunError("Rust input copy changed during probe")
            _check_payload_rows(java_payloads, java, cases)
            _check_payload_rows(rust_payloads, rust, cases)

        after_world = _hash_sources(corpus, root)
        report["source_world_hashes_after"] = after_world
        if after_world != source_hashes or sha256(jar.read_bytes()) != jar_hash:
            raise WorldRunError("original world source or JAR changed during run")
    except (WorldRunError, OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        failure = str(error)
        report.update(status="error", reason=failure)
    finally:
        try:
            if initial_sources is not None:
                after = source_snapshot(root)
                report["source_hashes_after"] = after
                if after != initial_sources:
                    report.update(status="error", reason="runner source files changed during execution")
            if input_dir is not None and evidence is not None:
                corpus.verify_snapshot(directory, evidence, root)
                report["input_copy_hashes_after"] = {name: sha256((input_dir/name).read_bytes())
                                                     for name in evidence["copied"]}
            if jar is not None and jar_hash is not None and sha256(jar.read_bytes()) != jar_hash:
                report.update(status="error", reason="original JAR changed during run")
        except (OSError, ValueError, KeyError) as error:
            report.update(status="error", reason=f"{failure + '; ' if failure else ''}final source/copy verification failed: {error}")
        _report_process_logs(report, directory)
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def _validate_audit(audit, source_hashes, row_count):
    calls = audit.get("native_calls", {})
    classes = audit.get("classes", {})
    if (audit.get("jar_sha256") != JAR_SHA256 or classes != CLASS_SHA256
            or audit.get("class_names") != ["ij", "nu", "ei", "qj"]
            or audit.get("native_methods") != {"nbt_parse": "ij.b(DataInput)", "metadata_constructor": "ei(nu)",
                                                  "region_read": "qj.a(int,int)", "native_chunk_loads": 0}
            or audit.get("native_calls") != {"ij_b_DataInput": 705, "ei_constructor": 1,
                                              "qj_a_int_int": 748, "native_chunk_loads": 0}
            or audit.get("live_rows") != row_count or audit.get("level_rows") != 1
            or audit.get("chunk_present") != 704 or audit.get("chunk_absent") != 44
            or audit.get("input_sha256") != source_hashes):
        raise WorldRunError("native call/class/source audit does not match pinned C01 evidence")
    if any(Path(path).name != "client-b1.7.3.jar" or ".local/minecraft/" not in path
           for path in audit.get("code_sources", {}).values()) or len(audit.get("code_sources", {})) != 4:
        raise WorldRunError("native classes did not load from the pinned local JAR")


def _retained_manifest(path, audit, prefix="native"):
    path = Path(path)
    expected = audit.get("retained_files", {}) if prefix == "native" else None
    files = sorted(p for p in path.iterdir()) if path.is_dir() else []
    names = {p.name for p in files}
    if any(p.is_symlink() or not p.is_file() or not re.fullmatch(r"[A-Za-z0-9_-]+\.nbt", p.name) for p in files):
        raise WorldRunError(f"{prefix} retained directory contains unsafe/unexpected entries")
    actual = {p.name: {"sha256": sha256(p.read_bytes()), "length": p.stat().st_size} for p in files}
    if expected is not None and ({name: value for name, value in expected.items()} !=
                                 {name: value["sha256"] for name, value in actual.items()}):
        raise WorldRunError("native retained hash manifest differs from actual retained bytes")
    return actual


def _bind_retained_to_rows(path, retained, output, cases):
    rows = output.splitlines(keepends=True)
    if len(rows) != len(cases):
        raise WorldRunError("native output row count mismatch")
    expected = {}
    for line, case in zip(rows, cases):
        fields = line[:-1].split(b"\t")
        if case["kind"] == "level" or (case["kind"] == "chunk" and fields[2] == b"present"):
            expected[case["id"] + ".nbt"] = (int(fields[-2]), fields[-1].decode("ascii"))
    if set(retained) != set(expected):
        raise WorldRunError("retained files do not match level + present rows exactly")
    for name, (length, digest) in expected.items():
        raw = (Path(path) / name).read_bytes()
        if len(raw) != length or sha256(raw) != digest or retained[name] != {"length": length, "sha256": digest}:
            raise WorldRunError(f"retained raw bytes do not match corresponding TSV row: {name}")


def _check_payload_rows(path, output, cases):
    _bind_retained_to_rows(path, _retained_manifest(path, {}), output, cases)
