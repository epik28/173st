#!/usr/bin/env python3
"""Capture/audit bounded water update ticks from the pinned Beta 1.7.3 client."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/water_tick"
SOURCE_FIXTURES = ROOT / "tests/fixtures/water_tick_sources"
ORACLE = ROOT / "tools/oracle/src/WaterTickOracle.java"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASS_HASHES = {
    "uu": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "om": "189fe9f8e3a363be62eefbbdd60052b811fb1d25a011f42df265f9a436e49fb2",
    "nx": "0a2191eee933fa9ae5207d9f6d085f5f93e006ec35459b0fa073ef859104d213",
    "rp": "659aba3f004ae066c05560529480f3f03db59a3dc1412325847ac01d2f1bbec6",
    "fd": "b91d4726c43ed2dad1a72fd8d925eaded8a7c365af5d9f37c3ad101b957a4100",
    "qy": "500a13a82592d4aa09a9b36de409138962f7a43e31f5bd6906976d4d01630c83",
}
METHODS = "#methods\tom.a(fd,int,int,int,Random); nx.a(fd,int,int,int,Random); fd.c(int,int,int,int,int); fd.a(boolean); fd.i(int,int,int,int)"
BOUNDS = "#bounds\tstate snapshot x,z=-2..2 y=62..65; loaded chunks are synthetic and all loaded; no unloaded-neighbor result is claimed"
LOADED = "#loaded_chunks\t-1,-1;-1,0;-1,1;0,-1;0,0;0,1;1,-1;1,0;1,1"
CASES = ["still-source", "flow-open", "flow-decay", "downward-open", "horizontal-basin",
         "blocked-down-and-sides", "source-neighbor-water", "two-adjacent-sources", "two-tick-spread"]
SOURCE_CASES = ["two-neighbors-supported-meta1", "one-neighbor-supported-meta1",
                "two-neighbors-unsupported-meta1", "two-neighbors-source-below-meta1"]
SETUP = "#setup\tfd copy-constructor over Unsafe seed; custom wt/bf with byte-array lm chunks only; fixed origin (0,64,0); nine in-memory chunks (-1..1); no save/provider reads"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _fields(line: str, prefix: str) -> dict[str, str]:
    if not line.startswith(prefix):
        raise ValueError(f"missing {prefix.strip()} header")
    return dict(field.split("=", 1) for field in line.split("\t")[1:] if "=" in field)


def _queue(value: str) -> list[tuple[int, int, int, int, int]]:
    if not value:
        return []
    out = []
    for entry in value.split(","):
        fields = tuple(int(n) for n in entry.split(":"))
        if len(fields) != 5:
            raise ValueError("malformed native scheduled-tick entry")
        out.append(fields)
    return out


def parse_native(raw: bytes, expected_cases: list[str] = CASES) -> dict:
    lines = raw.decode("ascii", errors="strict").splitlines()
    if len(lines) < 7 or not lines[0].startswith("#WATER_TICK_NATIVE\t1\t"):
        raise ValueError("native water tick header missing")
    header = _fields(lines[0], "#WATER_TICK_NATIVE\t1\t")
    if header != {"jar": JAR_SHA, **CLASS_HASHES}:
        raise ValueError("pinned Beta client or liquid/scheduler classes changed")
    if lines[1:5] != [SETUP, METHODS, BOUNDS, LOADED]:
        raise ValueError("synthetic setup, method provenance, bounds or loaded chunks changed")
    cursor = 5
    cases = []
    all_input_rows = []
    while cursor < len(lines):
        if not lines[cursor].startswith("case\t"):
            raise ValueError(f"expected case at line {cursor + 1}")
        head = lines[cursor].split("\t")
        if len(head) != 6:
            raise ValueError("malformed scene header")
        name = head[1]
        all_input_rows.append(lines[cursor])
        attrs = dict(item.split("=", 1) for item in head[2:])
        if name not in expected_cases or name in {case["name"] for case in cases}:
            raise ValueError("unknown or duplicate water scene")
        required_attrs = {"seed_id", "input_meta", "ticks", "rng_seed"}
        if set(attrs) != required_attrs:
            raise ValueError("water scene metadata fields changed")
        ticks = int(attrs["ticks"])
        cursor += 1
        inputs = []
        while cursor < len(lines) and lines[cursor].startswith("input\t"):
            parts = lines[cursor].split("\t")
            if len(parts) != 6:
                raise ValueError("malformed water scene input row")
            inputs.append(tuple(int(v) for v in parts[1:]))
            all_input_rows.append(lines[cursor])
            cursor += 1
        if not inputs or cursor >= len(lines) or not lines[cursor].startswith("setup_queue\t"):
            raise ValueError("water scene input or setup queue missing")
        initial_queue = _queue(lines[cursor].split("\t", 1)[1])
        cursor += 1
        setup_events = []
        while cursor < len(lines) and lines[cursor].startswith("setup_"):
            setup_events.append(lines[cursor][6:])
            cursor += 1
        steps = []
        for _ in range(ticks):
            if cursor >= len(lines) or not lines[cursor].startswith("queue_before\t"):
                raise ValueError("scheduled queue snapshot missing")
            qparts = lines[cursor].split("\t")
            if len(qparts) != 3 or not qparts[2].startswith("world_time="):
                raise ValueError("malformed queue-before row")
            qbefore = _queue(qparts[1])
            world_time = int(qparts[2].split("=", 1)[1])
            if not qbefore or world_time <= min(row[4] for row in qbefore):
                raise ValueError("tick was not dispatched after its native scheduled time")
            cursor += 1
            if cursor >= len(lines) or not lines[cursor].startswith("step\t"):
                raise ValueError("scheduler step result missing")
            result_fields = dict(item.split("=", 1) for item in lines[cursor].split("\t")[2:])
            if set(result_fields) != {"pending", "queue_after"}:
                raise ValueError("malformed scheduler result")
            qafter = _queue(result_fields["queue_after"])
            cursor += 1
            if cursor >= len(lines) or not lines[cursor].startswith("state\t"):
                raise ValueError("bounded block state missing")
            state = {}
            for cell in lines[cursor].split("\t", 1)[1].rstrip(";").split(";"):
                position, sep, value = cell.partition("=")
                if not sep or position in state:
                    raise ValueError("malformed or duplicate state cell")
                block_id, meta = (int(v) for v in value.split(":"))
                state[position] = (block_id, meta)
            if len(state) != 100:
                raise ValueError("bounded state must contain all 100 cells")
            cursor += 1
            events = []
            while cursor < len(lines) and not lines[cursor].startswith(("#case_end\t", "queue_before\t")):
                event = lines[cursor].split("\t")
                if event[0] not in {"edit", "schedule", "neighbor"}:
                    raise ValueError("unknown native tick event")
                expected_length = {"edit": 8, "schedule": 6, "neighbor": 5}[event[0]]
                if len(event) != expected_length:
                    raise ValueError("malformed native tick event row")
                events.append(tuple(event))
                cursor += 1
            steps.append({"queue_before": qbefore, "queue_after": qafter,
                          "world_time": world_time, "pending": result_fields["pending"] == "1",
                          "state": state, "events": events})
        if cursor >= len(lines) or lines[cursor] != f"#case_end\t{name}":
            raise ValueError(f"case terminator missing or mismatched for {name}: {lines[cursor] if cursor < len(lines) else 'EOF'}")
        cursor += 1
        cases.append({"name": name, "attrs": attrs, "inputs": inputs,
                      "setup_queue": initial_queue, "setup_events": setup_events, "steps": steps})
    if [case["name"] for case in cases] != expected_cases:
        raise ValueError("native water scene matrix is incomplete or out of order")
    return {"cases": cases, "case_count": len(cases),
            "input_sha256": digest(("\n".join(all_input_rows) + "\n").encode("ascii")),
            "native_sha256": digest(raw), "events": sum(len(case["setup_events"]) + sum(len(step["events"]) for step in case["steps"]) for case in cases),
            "repeatable": True}


def configured_jar() -> Path:
    configured = os.environ.get("MINECRAFT_BETA_173_JAR")
    if configured:
        return Path(configured).expanduser().resolve(strict=True)
    config = ROOT / "config/local.toml"
    for line in config.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("client_jar") and "=" in line:
            return (ROOT / line.split("=", 1)[1].strip().strip('"')).resolve(strict=True)
    raise ValueError("config/local.toml has no client_jar")


def one_live(jar: Path, run_dir: Path, sources: bool = False) -> bytes:
    classes = run_dir / "classes"
    classes.mkdir(parents=True)
    compiled = subprocess.run(["javac", "-Xlint:none", "-cp", str(jar), "-d", str(classes), str(ORACLE)],
                              capture_output=True, timeout=45)
    if compiled.returncode:
        raise RuntimeError(compiled.stderr.decode(errors="replace")[-3000:])
    command = ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
               os.pathsep.join((str(classes), str(jar))), "WaterTickOracle", str(jar)]
    if sources:
        command.append("--sources")
    result = subprocess.run(command,
                            capture_output=True, timeout=90)
    if result.returncode or result.stderr:
        raise RuntimeError("native water oracle failed/noisy: " + result.stderr.decode(errors="replace")[-3000:])
    parse_native(result.stdout, SOURCE_CASES if sources else CASES)
    return result.stdout


def fixture_check(raw_override: bytes | None = None) -> dict:
    raw = (FIXTURES / "native.tsv").read_bytes() if raw_override is None else raw_override
    result = parse_native(raw)
    audit = json.loads((FIXTURES / "audit.json").read_text(encoding="utf-8"))
    if audit.get("native_sha256") != digest(raw) or audit.get("jar_sha256") != JAR_SHA:
        raise ValueError("water tick audit does not bind frozen native bytes and client JAR")
    if audit.get("oracle_source_sha256") != digest(ORACLE.read_bytes()):
        raise ValueError("water tick audit does not bind the Java adapter source")
    if digest((FIXTURES / "input.tsv").read_bytes()) != result["input_sha256"]:
        raise ValueError("water tick input file differs from native scene matrix")
    if audit.get("input_sha256") != result["input_sha256"] or audit.get("case_count") != result["case_count"]:
        raise ValueError("water tick input matrix or case count does not match audit")
    return result


def verify_live(write: bool = False) -> dict:
    if not all(shutil.which(name) for name in ("javac", "java")):
        raise RuntimeError("live water tick capture requires javac and java")
    jar = configured_jar()
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured original client JAR hash mismatch")
    with tempfile.TemporaryDirectory(prefix="water-tick-native-") as temporary:
        temp = Path(temporary)
        first, second = one_live(jar, temp / "first"), one_live(jar, temp / "second")
    if first != second:
        raise AssertionError("independent native water tick captures differ")
    result = parse_native(first)
    audit = {key: result[key] for key in ("case_count", "input_sha256", "native_sha256", "events", "repeatable")}
    audit.update({"jar_sha256": JAR_SHA, "oracle_source_sha256": digest(ORACLE.read_bytes()),
                  "captures": 2, "repeat_identical": True, "save_access": False})
    if write:
        FIXTURES.mkdir(parents=True, exist_ok=True)
        input_rows = [line for line in first.decode("ascii").splitlines() if line.startswith(("case\t", "input\t"))]
        (FIXTURES / "input.tsv").write_text("\n".join(input_rows) + "\n", encoding="ascii")
        (FIXTURES / "native.tsv").write_bytes(first)
        (FIXTURES / "audit.json").write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    elif first != (FIXTURES / "native.tsv").read_bytes():
        raise AssertionError("live water tick output differs from committed fixture")
    return audit


def source_fixture_check(raw_override: bytes | None = None) -> dict:
    raw = (SOURCE_FIXTURES / "native.tsv").read_bytes() if raw_override is None else raw_override
    result = parse_native(raw, SOURCE_CASES)
    audit = json.loads((SOURCE_FIXTURES / "audit.json").read_text(encoding="utf-8"))
    if (audit.get("extension_version") != "water-source-formation-1" or
            audit.get("native_sha256") != digest(raw) or audit.get("jar_sha256") != JAR_SHA):
        raise ValueError("source-formation audit does not bind frozen native bytes and client JAR")
    if audit.get("oracle_source_sha256") != digest(ORACLE.read_bytes()):
        raise ValueError("source-formation audit does not bind the Java adapter source")
    if (audit.get("captures") != 2 or audit.get("repeat_identical") is not True or
            audit.get("repeatable") is not True or audit.get("save_access") is not False):
        raise ValueError("source-formation audit does not prove two identical memory-only captures")
    if digest((SOURCE_FIXTURES / "input.tsv").read_bytes()) != result["input_sha256"]:
        raise ValueError("source-formation input file differs from native scene matrix")
    if audit.get("input_sha256") != result["input_sha256"] or audit.get("case_count") != len(SOURCE_CASES):
        raise ValueError("source-formation input matrix or case count does not match audit")
    result["extension_version"] = audit["extension_version"]
    return result


def verify_sources_live(write: bool = False) -> dict:
    if not all(shutil.which(name) for name in ("javac", "java")):
        raise RuntimeError("live source-formation capture requires javac and java")
    jar = configured_jar()
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured original client JAR hash mismatch")
    with tempfile.TemporaryDirectory(prefix="water-source-native-") as temporary:
        temp = Path(temporary)
        first, second = one_live(jar, temp / "first", sources=True), one_live(jar, temp / "second", sources=True)
    if first != second:
        raise AssertionError("independent native source-formation captures differ")
    result = parse_native(first, SOURCE_CASES)
    audit = {key: result[key] for key in ("case_count", "input_sha256", "native_sha256", "events", "repeatable")}
    audit.update({"jar_sha256": JAR_SHA, "oracle_source_sha256": digest(ORACLE.read_bytes()),
                  "captures": 2, "repeat_identical": True, "save_access": False,
                  "extension_version": "water-source-formation-1"})
    if write:
        SOURCE_FIXTURES.mkdir(parents=True, exist_ok=True)
        input_rows = [line for line in first.decode("ascii").splitlines() if line.startswith(("case\t", "input\t"))]
        (SOURCE_FIXTURES / "input.tsv").write_text("\n".join(input_rows) + "\n", encoding="ascii")
        (SOURCE_FIXTURES / "native.tsv").write_bytes(first)
        (SOURCE_FIXTURES / "audit.json").write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    elif first != (SOURCE_FIXTURES / "native.tsv").read_bytes():
        raise AssertionError("live source-formation output differs from committed fixture")
    return audit


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", action="store_true", help="run the pinned original client in isolated memory worlds")
    parser.add_argument("--source-live", action="store_true", help="capture source-formation scenes from the pinned original client")
    parser.add_argument("--write", action="store_true", help="freeze fixture after two independent identical captures")
    parser.add_argument("--sources", action="store_true", help="verify source-formation extension fixture")
    args = parser.parse_args()
    if sum((args.live, args.source_live, args.sources)) > 1:
        parser.error("choose only one of --live, --source-live or --sources")
    if args.write and not (args.live or args.source_live):
        parser.error("--write requires --live or --source-live")
    if args.source_live:
        result = verify_sources_live(write=args.write)
    elif args.live:
        result = verify_live(write=args.write)
    elif args.sources:
        result = source_fixture_check()
    else:
        result = fixture_check()
    print(json.dumps({key: value for key, value in result.items() if key != "cases"}, indent=2))
