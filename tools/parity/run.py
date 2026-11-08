#!/usr/bin/env python3
"""Live Java/Rust parity. No saved or synthetic reference outputs are used."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path("tests/fixtures/rng/next-int.tsv")
ORACLE = Path("tools/oracle/src/RandomOracle.java")
PROBE = Path("crates/beta-core/examples/rng_probe.rs")
REGION_FIXTURE = Path("tests/fixtures/region/live.tsv")
REGION_ORACLE = Path("tools/oracle/src/RegionOracle.java")
REGION_PROBE = Path("crates/beta-core/examples/region_probe.rs")
REGION_MARKER = "REGION-001 copied inputs\n"
REGION_JAR_SHA256 = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
REGION_CLASS_SHA256 = "6c75a123b63d555caa98e1a2619aa2483e405566de9e356585b1dc682a7da4a6"


class RunError(Exception):
    pass


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def parse_fixture(data):
    cases, ids = [], set()
    for number, line in enumerate(data.decode("utf-8").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 3 or not re.fullmatch(r"[A-Za-z0-9_-]+", fields[0]):
            raise RunError(f"fixture line {number}: invalid fields/ID")
        case_id, seed, count = fields
        if (case_id in ids or not re.fullmatch(r"[+-]?[0-9]+", seed)
                or not re.fullmatch(r"\+?[0-9]+", count)):
            raise RunError(f"fixture line {number}: duplicate ID or invalid decimal")
        seed, count = int(seed), int(count)
        if not -(2**63) <= seed < 2**63 or not 1 <= count <= 100000:
            raise RunError(f"fixture line {number}: seed/count outside contract")
        ids.add(case_id)
        cases.append({"id": case_id, "seed": seed, "count": count})
    if not cases:
        raise RunError("fixture contains no cases")
    return cases


def parse_extended_fixture(data):
    cases, ids, total = [], set(), 0
    for number, line in enumerate(data.decode("utf-8").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 3 or not re.fullmatch(r"[A-Za-z0-9_-]+", fields[0]) or fields[0] in ids:
            raise RunError(f"fixture line {number}: invalid fields/ID")
        case_id, seed, tokens = fields
        if not re.fullmatch(r"[+-]?[0-9]+", seed) or not -(2**63) <= int(seed) < 2**63:
            raise RunError("invalid seed")
        operations = []
        for token in tokens.split(";"):
            parts = token.split(":")
            if len(parts) == 1 and token in {"int", "long", "boolean", "float", "double"}:
                operations.append((token, None))
            elif len(parts) == 2 and parts[0] in {"bound", "seed"}:
                if not re.fullmatch(r"[+-]?[0-9]+", parts[1]):
                    raise RunError("invalid operation argument")
                value = int(parts[1])
                bits = 32 if parts[0] == "bound" else 64
                if not -(2**(bits-1)) <= value < 2**(bits-1):
                    raise RunError("operation argument out of range")
                operations.append((parts[0], value))
            else:
                raise RunError(f"invalid operation: {token}")
        total += len(operations)
        if total > 100000:
            raise RunError("operation limit 100000 exceeded")
        ids.add(case_id)
        cases.append({"id": case_id, "seed": int(seed), "count": len(operations), "operations": operations})
    if not cases:
        raise RunError("fixture contains no cases")
    return cases


def extended_keys(cases):
    return [(case["id"], index, op, arg) for case in cases
            for index, (op, arg) in enumerate(case["operations"])]


def valid_extended_record(line, key):
    case_id, index, operation, argument = key
    prefix = f"{case_id}\t{index}\t{operation}\t".encode("ascii")
    if not line.startswith(prefix) or not line.endswith(b"\n"):
        return False
    value = line[len(prefix):-1]
    if operation == "seed":
        return value == b"ok"
    if operation == "boolean":
        return value in {b"0", b"1"}
    if operation in {"float", "double"}:
        width, maximum = (8, 0x3f7fffff) if operation == "float" else (16, 0x3fefffffffffffff)
        return bool(re.fullmatch(rb"[0-9a-f]{" + str(width).encode() + rb"}", value)) and int(value, 16) <= maximum
    if operation == "bound" and argument <= 0:
        return value == b"invalid_bound"
    if not re.fullmatch(rb"(?:0|-[1-9][0-9]*|[1-9][0-9]*)", value):
        return False
    number = int(value)
    if operation == "bound":
        return 0 <= number < argument
    bits = 64 if operation == "long" else 32
    return -(2**(bits-1)) <= number < 2**(bits-1)


def parse_chunk_fixture(data):
    cases, ids, total = [], set(), 0
    for line in data.decode("utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 2 or not re.fullmatch(r"[A-Za-z0-9_-]+", fields[0]) or fields[0] in ids:
            raise RunError("invalid chunk fields/ID")
        operations = []
        for token in fields[1].split(";"):
            parts = token.split(":")
            op, arguments = parts[0], parts[1:]
            if op == "coord" and len(arguments) == 1:
                if not re.fullmatch(r"[+-]?[0-9]+", arguments[0]) or not -(2**31) <= int(arguments[0]) < 2**31:
                    raise RunError("coord argument outside signed i32")
                args = [int(arguments[0])]
            elif op == "byte" and len(arguments) == 2:
                array, index = arguments
                if array not in {"blocks", "data", "block_light", "sky_light"}:
                    raise RunError("unknown chunk array")
                limit = 32768 if array == "blocks" else 16384
                if not re.fullmatch(r"\+?[0-9]+", index) or not 0 <= int(index) < limit:
                    raise RunError("raw byte index outside array")
                args = [array, int(index)]
            elif (op in {"get", "index"} and len(arguments) == 3) or (op == "set" and len(arguments) == 7):
                if any(not re.fullmatch(r"\+?[0-9]+", arg) or not 0 <= int(arg) <= 255 for arg in arguments):
                    raise RunError("chunk argument outside u8")
                args = [int(arg) for arg in arguments]
            else:
                raise RunError(f"invalid chunk operation: {token}")
            operations.append((op, args))
        total += len(operations)
        if total > 100000:
            raise RunError("operation limit 100000 exceeded")
        ids.add(fields[0])
        cases.append({"id": fields[0], "count": len(operations), "operations": operations})
    if not cases:
        raise RunError("fixture contains no cases")
    return cases

def parse_nbt_fixture(data):
    cases, ids = [], set()
    for number, line in enumerate(data.decode("ascii").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 2 or not re.fullmatch(r"[A-Za-z0-9_-]+", fields[0]) or fields[0] in ids:
            raise RunError(f"NBT fixture line {number}: invalid fields/ID")
        if not re.fullmatch(r"[0-9a-fA-F]+", fields[1]) or len(fields[1]) % 2:
            raise RunError(f"NBT fixture line {number}: invalid hex")
        ids.add(fields[0]); cases.append({"id": fields[0], "data": bytes.fromhex(fields[1])})
    if not cases: raise RunError("NBT fixture contains no cases")
    return cases


def parse_region_fixture(data):
    """Parse the coordinate-only REGION-001 corpus and bind names to floor coords."""
    cases, ids = [], set()
    for number, line in enumerate(data.decode("ascii").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 4:
            raise RunError(f"region fixture line {number}: expected four fields")
        case_id, filename, x_text, z_text = fields
        if (not re.fullmatch(r"[A-Za-z0-9_-]+", case_id) or case_id in ids
                or not re.fullmatch(r"r\.-?(?:0|[1-9][0-9]*)\.-?(?:0|[1-9][0-9]*)\.mcr", filename)):
            raise RunError(f"region fixture line {number}: invalid ID or basename")
        if not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", x_text) or not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", z_text):
            raise RunError(f"region fixture line {number}: invalid coordinate syntax")
        x, z = int(x_text), int(z_text)
        if not -(2**31) <= x < 2**31 or not -(2**31) <= z < 2**31:
            raise RunError(f"region fixture line {number}: coordinate outside i32")
        expected = f"r.{x // 32}.{z // 32}.mcr"
        if filename != expected:
            raise RunError(f"region fixture line {number}: region basename does not match coordinates")
        ids.add(case_id)
        cases.append({"id": case_id, "file": filename, "x": x, "z": z})
        if len(cases) > 100000:
            raise RunError("region case limit 100000 exceeded")
    if not cases:
        raise RunError("region fixture contains no cases")
    return cases


def region_keys(cases):
    return [(case["id"], case["file"], case["x"], case["z"]) for case in cases]


def valid_region_record(line, key):
    if key is None or line is None or not line.endswith(b"\n") or b"\r" in line:
        return False
    fields = line[:-1].split(b"\t")
    if len(fields) != 6 or fields[0] != key[0].encode("ascii"):
        return False
    if fields[1] == b"absent":
        return fields[2:] == [b"-", b"-", b"-", b"-"]
    if fields[1] != b"present":
        return False
    return (re.fullmatch(rb"(?:0|[1-9][0-9]*)", fields[2]) is not None
            and int(fields[2]) <= 0xffffffff
            and fields[3] in {b"1", b"2"}
            and re.fullmatch(rb"(?:0|[1-9][0-9]*)", fields[4]) is not None
            and int(fields[4]) <= 64 * 1024 * 1024
            and re.fullmatch(rb"[0-9a-f]{64}", fields[5]) is not None)


def safe_region_file(input_dir, name):
    """Resolve only an ordinary region basename inside a marked per-run copy."""
    input_dir = Path(input_dir).resolve(strict=True)
    marker = input_dir / ".region-input-copy"
    if marker.is_symlink() or not marker.is_file() or marker.read_text() != REGION_MARKER:
        raise RunError("region input copy marker is missing or invalid")
    if Path(name).name != name or not re.fullmatch(r"r\.-?(?:0|[1-9][0-9]*)\.-?(?:0|[1-9][0-9]*)\.mcr", name):
        raise RunError("region path must be a valid basename")
    path = input_dir / name
    if path.is_symlink() or not path.is_file() or path.resolve(strict=True).parent != input_dir:
        raise RunError("region path escapes input copy or is absent")
    return path


def compare_region_outputs(reference, actual, cases):
    left, right = reference.splitlines(keepends=True), actual.splitlines(keepends=True)
    keys = region_keys(cases)
    count, first = 0, None
    for index in range(max(len(keys), len(left), len(right))):
        expected = left[index] if index < len(left) else None
        observed = right[index] if index < len(right) else None
        key = keys[index] if index < len(keys) else None
        if expected != observed or not valid_region_record(expected, key) or not valid_region_record(observed, key):
            count += 1
            if first is None:
                first = {"record": index, "case": None if key is None else key[0],
                         "reference": None if expected is None else expected.decode("utf-8", "replace"),
                         "rust": None if observed is None else observed.decode("utf-8", "replace")}
    return {"mismatch_count": count, "first_mismatch": first}

def valid_nbt_record(line, key):
    prefix = f"{key[0]}\t{key[1]}\t".encode("ascii")
    if not line.startswith(prefix) or not line.endswith(b"\n"): return False
    fields = line[len(prefix):-1].split(b"\t")
    return (len(fields) == 2 and len(fields[0]) % 2 == 0 and re.fullmatch(rb"[0-9a-f]+", fields[0])
            and re.fullmatch(rb"[1-9][0-9]*", fields[1]) is not None)


def canonical_nbt(data):
    """Independent wire inspection, not an original-game oracle.

    Compound order is ignored and duplicate names use last-value-wins. IEEE bits
    and empty-list element types are retained; strings use UTF-16 code units.
    """
    offset = 0

    def take(length):
        nonlocal offset
        if length < 0 or offset + length > len(data):
            raise ValueError("truncated NBT")
        result = data[offset:offset + length]
        offset += length
        return result

    def integer(width, signed=True):
        return int.from_bytes(take(width), "big", signed=signed)

    def string():
        encoded = take(integer(2, False))
        units, index = [], 0
        while index < len(encoded):
            first = encoded[index]
            width = 1 if first < 128 else (2 if first & 0xe0 == 0xc0 else (3 if first & 0xf0 == 0xe0 else 0))
            if not width or index + width > len(encoded):
                raise ValueError("invalid modified UTF-8")
            unit = first if width == 1 else first & (0x1f if width == 2 else 0x0f)
            for byte in encoded[index + 1:index + width]:
                if byte & 0xc0 != 0x80:
                    raise ValueError("invalid modified UTF-8 continuation")
                unit = (unit << 6) | (byte & 63)
            units.append(unit)
            index += width
        return tuple(units)

    def payload(kind, depth):
        if depth > 512:
            raise ValueError("NBT depth limit")
        if kind in (1, 2, 3, 4):
            return [kind, integer({1: 1, 2: 2, 3: 4, 4: 8}[kind])]
        if kind in (5, 6):
            return [kind, take(4 if kind == 5 else 8).hex()]
        if kind == 8:
            return [kind, list(string())]
        if kind in (7, 9, 11):
            element = integer(1, False) if kind == 9 else None
            length = integer(4)
            if length < 0 or length > 2_000_000:
                raise ValueError("NBT collection length")
            if kind == 7:
                return [kind, take(length).hex()]
            if kind == 11:
                return [kind, [integer(4) for _ in range(length)]]
            if element not in range(12) or (element == 0 and length):
                raise ValueError("invalid list element type")
            return [kind, element, [payload(element, depth + 1) for _ in range(length)]]
        if kind == 10:
            fields = {}
            while True:
                child = integer(1, False)
                if child == 0:
                    break
                name = string()
                fields[name] = payload(child, depth + 1)
            return [kind, [[list(name), fields[name]] for name in sorted(fields)]]
        raise ValueError("invalid NBT tag type")

    kind = integer(1, False)
    name = string()
    tree = {"name_utf16": list(name), "value": payload(kind, 0)}
    if offset != len(data):
        raise ValueError("trailing NBT bytes")
    return tree


def java_written_nbt(tree):
    """Explicit DataOutput NaN rule; no other payload/type normalization."""
    tree = json.loads(json.dumps(tree))

    def visit(tag):
        kind = tag[0]
        if kind in (5, 6):
            bits = int(tag[1], 16)
            mantissa = 23 if kind == 5 else 52
            exponent = 8 if kind == 5 else 11
            if (bits >> mantissa) & ((1 << exponent) - 1) == (1 << exponent) - 1 and bits & ((1 << mantissa) - 1):
                tag[1] = "7fc00000" if kind == 5 else "7ff8000000000000"
        elif kind == 9:
            for child in tag[2]:
                visit(child)
        elif kind == 10:
            for _, child in tag[1]:
                visit(child)
    visit(tree["value"])
    return tree


def inspect_nbt_outputs(data, cases):
    records = data.splitlines(keepends=True)
    if len(records) != len(cases):
        raise ValueError("NBT output count")
    result = []
    for line, case in zip(records, cases):
        if not valid_nbt_record(line, (case["id"], 0)):
            raise ValueError("NBT output protocol")
        _, _, wire, consumed = line.decode("ascii").rstrip("\n").split("\t")
        if int(consumed) != len(case["data"]):
            raise ValueError("incorrect NBT consumed length")
        result.append({"id": case["id"], "consumed": int(consumed),
                       "tree": canonical_nbt(bytes.fromhex(wire))})
        if result[-1]["tree"] != java_written_nbt(canonical_nbt(case["data"])):
            raise ValueError(f"NBT output tree changed input semantics: {case['id']}")
    return result


def valid_chunk_record(line, key):
    case_id, index, op, args = key
    prefix = f"{case_id}\t{index}\t{op}\t".encode("ascii")
    if not line.startswith(prefix) or not line.endswith(b"\n"):
        return False
    value = line[len(prefix):-1]
    if op == "coord":
        return value == f"{args[0] >> 4},{args[0] & 15}".encode()
    if op in {"index", "get", "set"}:
        x, y, z = args[:3]
        if x >= 16 or y >= 128 or z >= 16:
            return value == b"invalid_coordinate"
        if op == "index":
            return value == str(y+z*128+x*2048).encode()
        if op == "set":
            return value == (b"invalid_nibble" if any(arg > 15 for arg in args[4:]) else b"ok")
        parts = value.split(b",")
        return (len(parts) == 4 and all(re.fullmatch(rb"(?:0|[1-9][0-9]*)", part) for part in parts)
                and 0 <= int(parts[0]) <= 255 and all(0 <= int(part) <= 15 for part in parts[1:]))
    return bool(re.fullmatch(rb"(?:0|[1-9][0-9]*)", value)) and 0 <= int(value) <= 255


def client_jar(root):
    config = root / "config/local.toml"
    if not config.is_file():
        raise RunError("client JAR not configured (config/local.toml)")
    value = tomllib.loads(config.read_text()).get("paths", {}).get("client_jar")
    if not value or not (root / value).is_file():
        raise RunError("configured client JAR is absent")
    return (root / value).resolve()


def expected_keys(cases):
    return [(case["id"], index) for case in cases for index in range(case["count"])]


def valid_record(line, key):
    if key is None:
        return False
    if len(key) == 4:
        if key[2] in {"coord", "index", "get", "set", "byte"}:
            return valid_chunk_record(line, key)
        return valid_extended_record(line, key)
    if len(key) == 3 and key[2] == "nbt":
        return valid_nbt_record(line, (key[0], key[1]))
    prefix = f"{key[0]}\t{key[1]}\t".encode("ascii")
    if not line.startswith(prefix) or not line.endswith(b"\n"):
        return False
    value = line[len(prefix):-1]
    if not re.fullmatch(rb"(?:0|-[1-9][0-9]*|[1-9][0-9]*)", value):
        return False
    return -(2**31) <= int(value) < 2**31


def compare_outputs(reference, actual, keys):
    """Require valid, complete ordered records even if both outputs are empty."""
    left, right = reference.splitlines(keepends=True), actual.splitlines(keepends=True)
    count, first = 0, None
    for index in range(max(len(keys), len(left), len(right))):
        expected = left[index] if index < len(left) else None
        observed = right[index] if index < len(right) else None
        key = keys[index] if index < len(keys) else None
        if (expected != observed or not valid_record(expected or b"", key)
                or not valid_record(observed or b"", key)):
            count += 1
            if first is None:
                first = {"record": index, "expected_key": list(key) if key else None,
                         "reference": None if expected is None else expected.decode("utf-8", "replace"),
                         "rust": None if observed is None else observed.decode("utf-8", "replace")}
    return {"mismatch_count": count, "first_mismatch": first}


def run_process(command, *, cwd, directory, label, timeout, input_data=None):
    """Keep raw bytes, diagnostics and execution status even on failure/timeout."""
    stdout_path, stderr_path = directory / f"{label}.stdout", directory / f"{label}.stderr"
    started = time.monotonic()
    result = {"command": [str(part) for part in command], "cwd": str(cwd),
              "stdout": str(stdout_path), "stderr": str(stderr_path),
              "returncode": None, "timeout": False, "error": None}
    try:
        completed = subprocess.run(command, cwd=cwd, input=input_data,
                                   capture_output=True, timeout=timeout, check=False)
        stdout, stderr = completed.stdout, completed.stderr
        result["returncode"] = completed.returncode
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b"", error.stderr or b""
        result["timeout"] = True
        result["error"] = f"timeout after {timeout}s"
    except OSError as error:
        stdout, stderr = b"", str(error).encode()
        result["error"] = str(error)
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    result["stdout_sha256"] = sha256(stdout)
    result["stderr_sha256"] = sha256(stderr)
    result["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
    return result


def source_snapshot(root):
    paths = {FIXTURE, ORACLE, Path("tools/parity/run.py"), Path("config/local.toml"), Path("Cargo.toml"),
             Path("Cargo.lock"), Path("rust-toolchain.toml")}
    for folder, pattern in (("tools/oracle", "*.java"), ("tools/parity", "*.py"), ("tests/fixtures/rng", "*.tsv"), ("tests/fixtures/chunk", "*.tsv"), ("tests/fixtures/nbt", "*.tsv"), ("tests/fixtures/region", "*.tsv"), ("tests/fixtures/region", "*.json"), ("tests/fixtures/region", "*.md")):
        paths.update(path.relative_to(root) for path in (root / folder).rglob(pattern))
    paths.add(Path("tools/oracle/region_corpus.py"))
    paths.update(path.relative_to(root) for path in (root / "crates").rglob("*.rs"))
    paths.update(path.relative_to(root) for path in (root / "crates").rglob("Cargo.toml"))
    return {str(path): sha256((root / path).read_bytes()) if (root / path).is_file() else None
            for path in sorted(paths)}


def artifacts_root(root):
    local = root / "config/local.toml"
    if local.exists():
        value = tomllib.loads(local.read_text()).get("paths", {}).get("artifacts", "artifacts")
        return root / value
    return root / "artifacts"


def exit_code(status, strict):
    return int(status in {"fail", "error"} or (strict and status != "pass"))


def run_rng(root=ROOT, timeout=60, suite="rng"):
    chunk = suite == "chunk"
    nbt = suite == "nbt"
    extended = suite == "rng-extended"
    fixture = Path("tests/fixtures/rng/extended.tsv") if extended else (Path("tests/fixtures/nbt/corpus.tsv") if nbt else FIXTURE)
    class_name = "NbtOracle" if nbt else ("RandomExtendedOracle" if extended else "RandomOracle")
    oracle = Path(f"tools/oracle/src/{class_name}.java") if (extended or nbt) else ORACLE
    example = "nbt_probe" if nbt else ("rng_extended_probe" if extended else "rng_probe")
    probe = Path(f"crates/beta-core/examples/{example}.rs") if (extended or nbt) else PROBE
    if chunk:
        fixture = Path("tests/fixtures/chunk/layout.tsv")
        class_name, example = "ChunkOracle", "chunk_probe"
        oracle = Path("tools/oracle/src/ChunkOracle.java")
        probe = Path("crates/beta-core/examples/chunk_probe.rs")
    started = time.monotonic()
    parent = artifacts_root(root) / f"parity/{suite}"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {
        "schema_version": 1, "suite": suite, "scenario_id": "RNG-002-mixed" if extended else "RNG-001-nextInt",
        "started_at": datetime.now(timezone.utc).isoformat(),
        "status": "error", "reason": "comparison not completed",
        "scope": "Java RNG API: mixed methods and shared state; no Beta game integration" if extended else "Java RNG API: fresh java.util.Random(seed).nextInt(); no Beta game integration",
        "reference_side": "Java API", "reference_version": None,
        "reference_jar_sha256": "not_applicable: java.base API; no Minecraft JAR",
        "runtime_versions": {}, "platform": platform.platform(), "toolchain": {},
        "adapter_sha256": None, "adapter_class_sha256": None,
        "rust_revision": None, "rust_dirty": None, "rust_binary_sha256": None,
        "source_hashes": {}, "source_hashes_after": {}, "input_sha256": None,
        "fixture_origin": f"repository {fixture}; fixed inputs, no expected outputs",
        "seed": [], "event_schedule": "case order in saved input; fresh RNG per case; sequential operations and explicit reseeds",
        "comparison_policy": "exact ordered LF TSV; typed integers/raw IEEE bits; no tolerance or normalization",
        "case_count": 0, "value_count": 0, "mismatch_count": None, "first_mismatch": None,
        "duration_ms": None, "reference_output_sha256": None, "rust_output_sha256": None,
        "log_paths": {"input": str(directory / "input.tsv"), "report": str(report_path)},
        "processes": {},
    }

    def execute(label, command, input_data=None):
        result = run_process(command, cwd=root, directory=directory, label=label,
                             timeout=timeout, input_data=input_data)
        report["processes"][label] = result
        report["log_paths"][label] = {key: result[key] for key in ("stdout", "stderr")}
        if result["error"] or result["returncode"] != 0:
            raise RunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
        return Path(result["stdout"]).read_bytes()

    if nbt:
        report.update(scenario_id="NBT-001-roundtrip", reference_side="Original Beta 1.7.3 ij reader/writer + TSV wrapper",
                      scope="Uncompressed NBT types 0..10; IntArray is separate wire evidence; no world loading",
                      comparison_policy="exact ordered round-trip hex and consumed bytes; independent canonical output trees retain IEEE bits",
                      event_schedule="one fresh named root per input row")
    if chunk:
        report.update(scenario_id="CHUNK-001-layout", reference_side="Beta 1.7.3 client nibble class + API wrapper",
                      scope="Original nibble constructor/get/set/raw; wrapper coordinate/index/ID/validation; no world loading/rendering",
                      event_schedule="fresh zeroed container per case; operations in saved input order")
    jar, jar_hash = None, None
    before = None
    try:
        before = source_snapshot(root)
        report["source_hashes"] = before
        report["adapter_sha256"] = before.get(str(oracle))
        data = (root / fixture).read_bytes()
        (directory / "input.tsv").write_bytes(data)
        report["input_sha256"] = sha256(data)
        parse = parse_nbt_fixture if nbt else (parse_chunk_fixture if chunk else (parse_extended_fixture if extended else parse_fixture))
        cases = parse(data)
        keys = ([(case["id"], index, "nbt") for case in cases for index in [0]] if nbt
                else (extended_keys(cases) if (chunk or extended) else expected_keys(cases)))
        report.update(case_count=len(cases), value_count=len(keys), seed="not_applicable: no RNG" if (chunk or nbt) else cases)
        if chunk or nbt:
            jar = client_jar(root)
            jar_hash = sha256(jar.read_bytes())
            report.update(reference_jar_sha256=jar_hash, reference_jar_path=str(jar))
        report["rust_revision"] = execute("git-revision", ["git", "rev-parse", "HEAD"]).decode().strip()
        report["rust_dirty"] = bool(execute("git-dirty", ["git", "status", "--porcelain"]))
        for tool in ("java", "javac", "rustc", "cargo"):
            execute(f"version-{tool}", [tool, "-version" if tool in {"java", "javac"} else "--version"])
            process = report["processes"][f"version-{tool}"]
            version = (Path(process["stdout"]).read_bytes() + Path(process["stderr"]).read_bytes()).decode().strip()
            report["runtime_versions"][tool] = version
        report["reference_version"] = report["runtime_versions"]["java"]
        report["toolchain"] = {tool: report["runtime_versions"][tool] for tool in ("rustc", "cargo")}
        classes = directory / "classes"
        classes.mkdir()
        build_command = ["javac", "-d", str(classes)]
        if nbt:
            build_command += ["-cp", str(client_jar(root))]
        build_command += [str(root / oracle)]
        execute("java-build", build_command)
        report["adapter_class_sha256"] = sha256((classes / f"{class_name}.class").read_bytes())
        java_command = ["java", "-cp", str(classes) + (":" + str(client_jar(root)) if nbt else ""), class_name]
        if chunk:
            metadata = json.loads(execute("java-describe", java_command + ["--describe", str(jar)]))
            if metadata.get("jar_sha256") != jar_hash or metadata.get("class") != "wi":
                raise RunError("original nibble metadata does not match JAR")
            report.update(reference_version="Minecraft Beta 1.7.3 client; " + report["reference_version"],
                          reference_metadata=metadata)
            audit_path = directory / "reference-invocations.json"
            report["log_paths"]["reference_invocations"] = str(audit_path)
            java_command += [str(jar), str(audit_path)]
        if nbt:
            java_command += [str(client_jar(root))]
        reference = execute("java-run", java_command, data)
        if nbt:
            stderr = Path(report["log_paths"]["java-run"]["stderr"]).read_text()
            if f"jar_sha256={jar_hash}" not in stderr or f"cases={len(cases)} classes=ij,nu" not in stderr:
                raise RunError("original NBT invocation audit missing")
            report.update(reference_version="Minecraft Beta 1.7.3 client; " + report["reference_version"],
                          reference_invocations={"named_reads": len(cases), "named_writes": len(cases),
                                                 "classes": ["ij", "nu"], "jar_sha256": jar_hash})
        if chunk:
            invocations = json.loads(audit_path.read_text())
            if (invocations.get("jar_sha256") != jar_hash
                    or any(invocations.get(key, 0) <= 0 for key in ("native_constructors", "native_gets", "native_sets", "raw_reads"))):
                raise RunError("original nibble calls not demonstrated")
            report["reference_invocations"] = invocations
        report["reference_output_sha256"] = sha256(reference)
        if compare_outputs(reference, reference, keys)["mismatch_count"]:
            raise RunError("Java oracle output violates scenario protocol")
        if not (root / probe).is_file():
            report.update(status="not_implemented", reason=f"Rust {example} source is absent; Java oracle executed")
        else:
            build = execute("rust-build", ["cargo", "build", "--locked", "--offline", "-p", "beta-core",
                                            "--example", example, "--message-format=json-render-diagnostics"])
            binaries = []
            for line in build.splitlines():
                message = json.loads(line)
                if (message.get("reason") == "compiler-artifact"
                        and message.get("target", {}).get("name") == example
                        and message.get("executable")):
                    binaries.append(Path(message["executable"]))
            if len(binaries) != 1:
                raise RunError(f"cargo did not identify exactly one {example} executable")
            binary = directory / example
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            actual = execute("rust-run", [str(binary)], data)
            report["rust_output_sha256"] = sha256(actual)
            comparison = compare_outputs(reference, actual, keys)
            report.update(comparison)
            if nbt:
                trees = {"input": [{"id": case["id"], "tree": canonical_nbt(case["data"])} for case in cases],
                         "java": inspect_nbt_outputs(reference, cases), "rust": inspect_nbt_outputs(actual, cases)}
                tree_path = directory / "canonical-trees.json"
                tree_path.write_text(json.dumps(trees, ensure_ascii=True) + "\n")
                report["log_paths"]["canonical_trees"] = str(tree_path)
                report["canonical_trees_sha256"] = sha256(tree_path.read_bytes())
                report["canonical_match"] = trees["java"] == trees["rust"]
                if not report["canonical_match"] and not comparison["mismatch_count"]:
                    raise RunError("canonical NBT trees differ")
            report.update(status="fail" if comparison["mismatch_count"] else "pass",
                          reason="output mismatch" if comparison["mismatch_count"] else "all records match exactly")
    except (RunError, OSError, ValueError) as error:
        report.update(status="error", reason=str(error))
    finally:
        try:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if jar is not None and sha256(jar.read_bytes()) != jar_hash:
                report.update(status="error", reason="original JAR changed during comparison")
            if before is not None and before != after:
                report.update(status="error", reason="inputs/sources changed during comparison")
            saved = directory / "input.tsv"
            if saved.exists() and sha256(saved.read_bytes()) != report["input_sha256"]:
                report.update(status="error", reason="saved input changed during comparison")
        except OSError as error:
            report.update(status="error", reason=f"cannot verify sources after execution: {error}")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def region_source_snapshot(root):
    paths = {REGION_FIXTURE, REGION_ORACLE, REGION_PROBE, Path("tools/oracle/region_corpus.py"),
             Path("tools/parity/run.py"), Path("config/local.toml"), Path("Cargo.toml"),
             Path("Cargo.lock"), Path("rust-toolchain.toml"), Path("tests/fixtures/region/sources.json"),
             Path("tests/fixtures/region/audit.json"), Path("tests/fixtures/region/coordinates.tsv"),
             Path("tests/fixtures/region/provenance.md"), Path("tools/parity/region_review.py"),
             Path("tools/parity/verify_report.py")}
    paths.update(path.relative_to(root) for path in (root / "tests/fixtures/region").iterdir() if path.is_file())
    paths.update(path.relative_to(root) for path in (root / "tools/oracle").rglob("*.java"))
    paths.update(path.relative_to(root) for path in (root / "tools/parity").rglob("*.py"))
    paths.update(path.relative_to(root) for path in (root / "crates").rglob("*.rs"))
    paths.update(path.relative_to(root) for path in (root / "crates").rglob("Cargo.toml"))
    result = {}
    for path in sorted(paths):
        full = root / path
        result[str(path)] = sha256(full.read_bytes()) if full.is_file() else None
    return result


def region_config(root):
    config_path = root / "config/local.toml"
    if not config_path.is_file():
        raise RunError("region paths not configured (config/local.toml)")
    paths = tomllib.loads(config_path.read_text()).get("paths", {})
    if not paths.get("prism_instance"):
        raise RunError("paths.prism_instance is not configured")
    if not paths.get("client_jar"):
        raise RunError("paths.client_jar is not configured")
    prism = (root / paths["prism_instance"]).resolve(strict=True)
    jar = (root / paths["client_jar"]).resolve(strict=True)
    return prism, jar


def run_region(root=ROOT, timeout=60):
    """Run original and Rust readers only against a new, hash-verified copy."""
    from importlib.util import module_from_spec, spec_from_file_location

    module_path = root / "tools/oracle/region_corpus.py"
    spec = spec_from_file_location("region_corpus_runner", module_path)
    corpus = module_from_spec(spec)
    spec.loader.exec_module(corpus)

    started = time.monotonic()
    parent = artifacts_root(root) / "parity/region"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {"schema_version": 1, "suite": "region", "scenario_id": "REGION-001-read-copies",
              "started_at": datetime.now(timezone.utc).isoformat(), "status": "error",
              "reason": "comparison not completed", "reference_side": "Original Beta 1.7.3 qj(File).a(int,int)",
              "reference_version": None, "reference_jar_path": None, "reference_jar_sha256": None,
              "reference_class_sha256": REGION_CLASS_SHA256,
              "scope": "read-only McRegion record extraction from run-specific copies; no world loading or writes",
              "comparison_policy": "exact ordered presence, unsigned timestamp, compression id, NBT length and SHA-256",
              "platform": platform.platform(), "runtime_versions": {}, "toolchain": {}, "rust_revision": None,
              "rust_dirty": None, "rust_binary_sha256": None, "adapter_class_sha256": None,
              "source_hashes": {}, "source_hashes_after": {}, "source_world_hashes": {},
              "source_world_hashes_after": {}, "input_copy_hashes": {}, "input_copy_hashes_after": {},
              "input_sha256": None, "case_count": 0, "present_count": None, "absent_count": None,
              "native_cases": {"live": 0, "synthetic": 0}, "mismatch_count": None, "first_mismatch": None,
              "duration_ms": None, "reference_output_sha256": None, "rust_output_sha256": None,
              "log_paths": {"input": str(directory / "input.tsv"), "report": str(report_path)}, "processes": {}}

    def execute(label, command, input_data=None, cwd=root):
        result = run_process(command, cwd=cwd, directory=directory, label=label,
                             timeout=timeout, input_data=input_data)
        report["processes"][label] = result
        report["log_paths"][label] = {key: result[key] for key in ("stdout", "stderr")}
        if result["error"] or result["returncode"] != 0:
            raise RunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
        return Path(result["stdout"]).read_bytes()

    input_dir, evidence = None, None
    source_world, jar = None, None
    source_world_hashes = {}
    jar_hash = None
    initial = None
    failure = None
    try:
        prism, jar = region_config(root)
        manifest = json.loads((root / "tests/fixtures/region/sources.json").read_text())
        source_world = prism / manifest["world_relative_path"]
        _, source_world_hashes = corpus.source_hashes(root)
        jar_hash = sha256(jar.read_bytes())
        if jar_hash != REGION_JAR_SHA256:
            raise RunError("configured original client JAR has wrong SHA-256")
        report.update(reference_jar_path=str(jar), reference_jar_sha256=jar_hash,
                      source_world_path=str(source_world), source_world_hashes=source_world_hashes)
        initial = region_source_snapshot(root)
        report["source_hashes"] = initial

        input_dir, evidence = corpus.snapshot(directory, root)
        (directory / "snapshot.json").replace(directory / "snapshot-evidence.json")
        copied = {name: sha256((input_dir / name).read_bytes()) for name in source_world_hashes}
        report["input_copy_hashes"] = copied
        fixture = (root / REGION_FIXTURE).read_bytes()
        cases = parse_region_fixture(fixture)
        fixture_hash = sha256(fixture)
        (directory / "input.tsv").write_bytes(fixture)
        report.update(input_sha256=fixture_hash, case_count=len(cases),
                      present_count=704, absent_count=len(cases)-704,
                      native_cases={"live": len(cases), "synthetic": 0})

        for tool, command in (("java", ["java", "-version"]), ("javac", ["javac", "-version"]),
                              ("rustc", ["rustc", "--version"]), ("cargo", ["cargo", "--version"])):
            execute(f"version-{tool}", command)
            proc = report["processes"][f"version-{tool}"]
            report["runtime_versions"][tool] = (Path(proc["stdout"]).read_bytes() + Path(proc["stderr"]).read_bytes()).decode().strip()
        report["reference_version"] = report["runtime_versions"]["java"]
        report["toolchain"] = {key: report["runtime_versions"][key] for key in ("rustc", "cargo")}

        classes = directory / "classes"
        classes.mkdir()
        execute("java-build", ["javac", "-cp", str(jar), "-d", str(classes), str(root / REGION_ORACLE)])
        class_file = classes / "RegionOracle.class"
        report["adapter_class_sha256"] = sha256(class_file.read_bytes())
        java_command = ["java", "-XX:-UsePerfData", "-Xmx256m", "-cp", f"{classes}:{jar}", "RegionOracle", str(jar)]
        payload_dir = directory / "java-payloads"
        audit_path = directory / "native-audit.json"
        java_result = run_process(java_command + [str(input_dir), str(payload_dir), str(audit_path)],
                                  cwd=root, directory=directory, label="java-run", timeout=timeout,
                                  input_data=fixture)
        report["processes"]["java-run"] = java_result
        report["log_paths"]["java-run"] = {key: java_result[key] for key in ("stdout", "stderr")}
        if java_result["error"] or java_result["returncode"] != 0:
            raise RunError(f"java-run: {java_result['error'] or 'exit ' + str(java_result['returncode'])}")
        reference = Path(java_result["stdout"]).read_bytes()
        report["reference_output_sha256"] = sha256(reference)
        audit = json.loads(audit_path.read_text())
        if (audit.get("jar_sha256") != jar_hash or audit.get("class") != "qj"
                or audit.get("method") != "a(int,int)" or audit.get("class_sha256") != REGION_CLASS_SHA256
                or audit.get("native_reads") != len(cases) or audit.get("present") != 704
                or audit.get("absent") != len(cases)-704):
            raise RunError("original qj invocation audit does not match live fixture")
        report["reference_invocations"] = audit
        report["log_paths"]["reference_invocations"] = str(audit_path)
        if compare_region_outputs(reference, reference, cases)["mismatch_count"]:
            raise RunError("native adapter output violates region TSV contract")
        expected_payloads = set()
        for line, case in zip(reference.splitlines(keepends=True), cases):
            fields = line[:-1].decode("ascii").split("\t")
            if fields[1] == "present":
                name = case["id"] + ".nbt"
                path = payload_dir / name
                if path.is_symlink() or not path.is_file():
                    raise RunError(f"native payload missing or unsafe: {name}")
                payload = path.read_bytes()
                if len(payload) != int(fields[4]) or sha256(payload) != fields[5]:
                    raise RunError(f"native payload hash/length mismatch: {name}")
                expected_payloads.add(name)
        if {p.name for p in payload_dir.iterdir()} != expected_payloads:
            raise RunError("native payload set differs from present coordinate cases")
        report["payload_files"] = {name: {"sha256": sha256((payload_dir/name).read_bytes()),
                                          "length": (payload_dir/name).stat().st_size}
                                    for name in sorted(expected_payloads)}
        report["log_paths"]["payload_dir"] = str(payload_dir)
        report["log_paths"]["input_copy_dir"] = str(input_dir)

        if not (root / REGION_PROBE).is_file():
            report.update(status="not_implemented", reason="Rust region_probe source is absent; Java oracle executed")
        else:
            report["rust_revision"] = execute("git-revision", ["git", "rev-parse", "HEAD"]).decode().strip()
            report["rust_dirty"] = bool(execute("git-dirty", ["git", "status", "--porcelain"]))
            build_bytes = execute("rust-build", ["cargo", "build", "--locked", "--offline", "-p", "beta-core",
                                                       "--example", "region_probe", "--message-format=json-render-diagnostics"])
            # run_process keeps compiler diagnostics; inspect the saved stdout for Cargo's JSON artifact record.
            binaries = []
            for line in build_bytes.splitlines():
                message = json.loads(line)
                if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "region_probe" and message.get("executable"):
                    binaries.append(Path(message["executable"]))
            if len(binaries) != 1:
                raise RunError("cargo did not identify exactly one region_probe executable")
            binary = directory / "region_probe"
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            rust_payloads = directory / "rust-payloads"
            rust_payloads.mkdir()
            # A fresh, exclusive snapshot protects each side from a native repair/write.
            rust_input = directory / "rust-input"
            rust_input.mkdir()
            (rust_input / ".region-input-copy").write_text(REGION_MARKER)
            for name, digest in copied.items():
                with (rust_input/name).open("xb") as target:
                    target.write((input_dir/name).read_bytes())
                if sha256((rust_input/name).read_bytes()) != digest:
                    raise RunError(f"Rust input snapshot hash mismatch: {name}")
            report["log_paths"]["rust_input_copy_dir"] = str(rust_input)
            rust = execute("rust-run", [str(binary), str(rust_input), str(rust_payloads)], fixture)
            report["rust_output_sha256"] = sha256(rust)
            comparison = compare_region_outputs(reference, rust, cases)
            report.update(comparison)
            report.update(status="fail" if comparison["mismatch_count"] else "pass",
                          reason="output mismatch" if comparison["mismatch_count"] else "all records match exactly")
            report["log_paths"]["rust_payload_dir"] = str(rust_payloads)
            if {p.name for p in rust_payloads.iterdir()} != expected_payloads:
                raise RunError("Rust payload set differs from present coordinate cases")
            for name, info in report["payload_files"].items():
                payload = (rust_payloads/name).read_bytes()
                info["rust_sha256"] = sha256(payload)
                info["rust_length"] = len(payload)
                if sha256(payload) != info["sha256"] or len(payload) != info["length"]:
                    raise RunError(f"Rust payload bytes differ: {name}")
            report["input_copy_hashes_after"] = {name: sha256((input_dir/name).read_bytes()) for name in copied}
            report["rust_input_copy_hashes"] = {name: sha256((rust_input/name).read_bytes()) for name in copied}
            if report["input_copy_hashes_after"] != copied or report["rust_input_copy_hashes"] != copied:
                raise RunError("region input copy changed during reader execution")

        # Real files and JAR are verified once more even on not_implemented.
        _, after_world = corpus.source_hashes(root)
        report["source_world_hashes_after"] = after_world
        if after_world != source_world_hashes or sha256(jar.read_bytes()) != jar_hash:
            raise RunError("original world source or client JAR changed during comparison")
    except (RunError, OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        failure = str(error)
        report.update(status="error", reason=str(error))
    finally:
        try:
            if initial is not None:
                after = region_source_snapshot(root)
                report["source_hashes_after"] = after
                if after != initial:
                    report.update(status="error", reason="inputs/sources changed during comparison")
            if input_dir is not None and evidence is not None:
                corpus.verify_snapshot(directory, evidence, root)
                report["input_copy_hashes_after"] = {name: sha256((input_dir/name).read_bytes())
                                                     for name in evidence["copied"]}
            if jar is not None and jar_hash is not None and sha256(jar.read_bytes()) != jar_hash:
                raise ValueError("original JAR changed during comparison")
        except (OSError, ValueError, KeyError) as error:
            verification_error = f"source/copy verification failed: {error}"
            report.update(status="error", reason=f"{failure}; {verification_error}" if failure else verification_error)
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("group", choices=("list", "rng", "rng-extended", "chunk", "nbt", "region", "world", "view", "math", "java", "geometry", "movement", "color", "light", "fluid"))
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--timeout", type=float, default=60, help="per-process timeout in seconds")
    args = parser.parse_args(argv)
    if not 0 < args.timeout < float("inf"):
        parser.error("--timeout must be finite and positive")
    if args.group == "list":
        print("rng: RNG-001 nextInt; requires java, javac, rustc, cargo and rng_probe")
        print("rng-extended: RNG-002 mixed Random methods; requires Java and rng_extended_probe")
        print("chunk: CHUNK-001 original Beta nibble class; requires configured client JAR and chunk_probe")
        print("nbt: NBT-001 original Beta ij/nu reader/writer; requires configured client JAR and nbt_probe")
        print("region: REGION-001 original qj reads hash-verified copies; requires configured Prism instance/client JAR and region_probe")
        print("world: WORLD-LOAD-001 original ij/ei/qj reads two hash-verified copies; requires configured Prism instance/client JAR and world_probe")
        print("view: VIEW-001 original uu/cv mappings on a hash-verified 5x5 scene; requires configured Prism instance/client JAR/resource and view_probe")
        print("math: MATH-001 original client MathHelper; requires configured client JAR and math_probe")
        print("java: JAVA-001 direct JVM casts/String.hashCode; requires java_compat_probe")
        print("geometry: GAME-ORACLE-001 original eq/bt/vf geometry; custom input via tools/parity/geometry_run.py --input FILE")
        print("color: COLOR-001 original seeded climate and grass/foliage colors")
        print("light: LIGHT-001 original saved-light tables, clear-weather sun, and emission registry")
        print("movement: MOVE-001 original sn/ls collision and dry travel on synthetic static scenes; custom input via movement_run.py")
        print("fluid: FLUID-001 original cv water geometry from native Tessellator submissions; strict raw-vertex comparison")
        return 0
    try:
        if args.group == "region":
            report, path = run_region(timeout=args.timeout)
        elif args.group == "world":
            from world_run import run_world
            report, path = run_world(timeout=args.timeout)
        elif args.group == "view":
            from view_run import run_view
            report, path = run_view(timeout=args.timeout)
        elif args.group == "math":
            from math_run import run_math
            report, path = run_math(timeout=args.timeout)
        elif args.group == "java":
            from java_run import run_java_suite
            report, path = run_java_suite(timeout=args.timeout)
        elif args.group == "geometry":
            from geometry_run import run_geometry_suite
            report, path = run_geometry_suite(timeout=args.timeout)
        elif args.group == "color":
            from color_run import run_color_suite
            report, path = run_color_suite(timeout=args.timeout)
        elif args.group == "light":
            from light_run import run_light_suite
            report, path = run_light_suite(timeout=args.timeout)
        elif args.group == "movement":
            from movement_run import run_movement_suite
            report, path = run_movement_suite(timeout=args.timeout)
        elif args.group == "fluid":
            from fluid_run import run_fluid
            report, path = run_fluid(timeout=args.timeout)
        else:
            report, path = run_rng(timeout=args.timeout, suite=args.group)
    except (OSError, ValueError) as error:
        print(f"error: {error}")
        return 1
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    return exit_code(report["status"], args.strict)


if __name__ == "__main__":
    raise SystemExit(main())
