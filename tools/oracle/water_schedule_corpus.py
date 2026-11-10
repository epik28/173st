#!/usr/bin/env python3
"""Capture/audit native water scheduling caused by bounded world edits."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/water_tick_schedule"
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
SETUP = "#setup\tfd Unsafe seed; custom wt/bf with nine byte-array lm chunks (-1..1); all synthetic chunks loaded; time starts at 0; no save/provider reads"
METHODS = "#methods\tfd.a(int,int,int,int,int); fd.i(int,int,int,int); fd.c(int,int,int,int,int); om.b(fd,int,int,int,int); nx.b(fd,int,int,int,int)"
BOUNDS = "#bounds\tstate snapshot x,z=-1..1 y=62..65; only explicit fd.i neighbor notifications are exercised"
CASES = ["initial-flow-placement", "place-adjacent-solid-neighbor", "remove-still-water-support"]
VERSION = "water-tick-seeding-1"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fields(line: str, prefix: str) -> dict[str, str]:
    if not line.startswith(prefix):
        raise ValueError(f"missing {prefix.strip()} header")
    return dict(field.split("=", 1) for field in line.split("\t")[1:] if "=" in field)


def queue(value: str) -> list[tuple[int, int, int, int, int]]:
    if not value:
        return []
    result = []
    for item in value.split(","):
        row = tuple(int(value) for value in item.split(":"))
        if len(row) != 5:
            raise ValueError("malformed native scheduled-tick queue entry")
        result.append(row)
    return result


def parse_native(raw: bytes) -> dict:
    lines = raw.decode("ascii", errors="strict").splitlines()
    if len(lines) < 7 or not lines[0].startswith("#WATER_SCHEDULE_NATIVE\t1\t"):
        raise ValueError("native water schedule header missing")
    if fields(lines[0], "#WATER_SCHEDULE_NATIVE\t1\t") != {"jar": JAR_SHA, **CLASS_HASHES}:
        raise ValueError("pinned Beta client or liquid/scheduler class hash mismatch")
    if lines[1:4] != [SETUP, METHODS, BOUNDS]:
        raise ValueError("synthetic setup, methods, or observation bounds changed")

    cases = []
    input_rows = []
    cursor = 4
    while cursor < len(lines):
        head = lines[cursor].split("\t")
        if len(head) != 4 or head[0] != "case":
            raise ValueError(f"expected schedule case at line {cursor + 1}")
        name = head[1]
        if name not in CASES or any(case["name"] == name for case in cases):
            raise ValueError("unknown or duplicate schedule case")
        attrs = dict(item.split("=", 1) for item in head[2:])
        if set(attrs) != {"rng_seed", "operation"}:
            raise ValueError("schedule case metadata changed")
        input_rows.append(lines[cursor])
        cursor += 1
        inputs = []
        while cursor < len(lines) and lines[cursor].startswith("input\t"):
            parts = lines[cursor].split("\t")
            if len(parts) != 6:
                raise ValueError("malformed schedule setup input")
            inputs.append(tuple(int(value) for value in parts[1:]))
            input_rows.append(lines[cursor])
            cursor += 1
        if cursor >= len(lines) or not lines[cursor].startswith("setup_queue\t"):
            raise ValueError("schedule setup queue missing")
        setup_queue = queue(lines[cursor].split("\t", 1)[1])
        cursor += 1
        setup_events = []
        while cursor < len(lines) and lines[cursor].startswith("setup_"):
            setup_events.append(tuple(lines[cursor][6:].split("\t")))
            cursor += 1
        if cursor >= len(lines) or not lines[cursor].startswith("queue_before\t"):
            raise ValueError("pre-operation queue missing")
        queue_before = queue(lines[cursor].split("\t", 1)[1])
        cursor += 1
        if cursor >= len(lines) or not lines[cursor].startswith("action\t"):
            raise ValueError("native world edit action missing")
        action = tuple(lines[cursor].split("\t")[1:])
        if len(action) != 6:
            raise ValueError("malformed native world edit action")
        input_rows.append(lines[cursor])
        cursor += 1
        events = []
        while cursor < len(lines) and lines[cursor].startswith(("edit\t", "schedule\t", "neighbor\t")):
            event = tuple(lines[cursor].split("\t"))
            expected = {"edit": 8, "schedule": 6, "neighbor": 5}[event[0]]
            if len(event) != expected:
                raise ValueError("malformed schedule action event")
            events.append(event)
            cursor += 1
        if cursor >= len(lines) or not lines[cursor].startswith("queue_after\t"):
            raise ValueError("post-operation queue missing")
        queue_after = queue(lines[cursor].split("\t", 1)[1])
        cursor += 1
        if cursor >= len(lines) or not lines[cursor].startswith("state\t"):
            raise ValueError("bounded post-operation state missing")
        state = {}
        for cell in lines[cursor].split("\t", 1)[1].rstrip(";").split(";"):
            pos, sep, value = cell.partition("=")
            if not sep or pos in state:
                raise ValueError("malformed or duplicate schedule state cell")
            block_id, meta = (int(part) for part in value.split(":"))
            state[pos] = (block_id, meta)
        if len(state) != 36:
            raise ValueError("schedule snapshot must contain all 36 bounded cells")
        cursor += 1
        if cursor >= len(lines) or lines[cursor] != f"#case_end\t{name}":
            raise ValueError("schedule case terminator missing or mismatched")
        cursor += 1
        cases.append({"name": name, "attrs": attrs, "inputs": inputs,
                      "setup_queue": setup_queue, "setup_events": setup_events,
                      "queue_before": queue_before, "action": action,
                      "events": events, "queue_after": queue_after, "state": state})
    if [case["name"] for case in cases] != CASES:
        raise ValueError("native schedule case matrix incomplete or out of order")
    return {"cases": cases, "case_count": len(cases),
            "input_sha256": digest(("\n".join(input_rows) + "\n").encode("ascii")),
            "native_sha256": digest(raw), "events": sum(len(c["setup_events"]) + len(c["events"]) for c in cases)}


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


def capture_one(jar: Path, run_dir: Path) -> bytes:
    classes = run_dir / "classes"
    classes.mkdir(parents=True)
    compiled = subprocess.run(["javac", "-Xlint:none", "-cp", str(jar), "-d", str(classes), str(ORACLE)],
                              capture_output=True, timeout=45)
    if compiled.returncode:
        raise RuntimeError(compiled.stderr.decode(errors="replace")[-3000:])
    result = subprocess.run(["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
                             os.pathsep.join((str(classes), str(jar))), "WaterTickOracle", str(jar), "--schedule"],
                            capture_output=True, timeout=90)
    if result.returncode or result.stderr:
        raise RuntimeError("native schedule oracle failed/noisy: " + result.stderr.decode(errors="replace")[-3000:])
    parse_native(result.stdout)
    return result.stdout


def fixture_check(raw_override: bytes | None = None) -> dict:
    raw = (FIXTURES / "native.tsv").read_bytes() if raw_override is None else raw_override
    result = parse_native(raw)
    audit = json.loads((FIXTURES / "audit.json").read_text(encoding="utf-8"))
    if (audit.get("extension_version") != VERSION or audit.get("native_sha256") != digest(raw) or
            audit.get("input_sha256") != result["input_sha256"] or audit.get("case_count") != len(CASES) or
            audit.get("jar_sha256") != JAR_SHA or audit.get("oracle_source_sha256") != digest(ORACLE.read_bytes()) or
            audit.get("captures") != 2 or audit.get("repeat_identical") is not True or
            audit.get("save_access") is not False):
        raise ValueError("water schedule audit does not bind the fixture, adapter, and two memory-only captures")
    if (FIXTURES / "input.tsv").read_bytes() != (
            "\n".join(line for line in raw.decode("ascii").splitlines()
                      if line.startswith(("case\t", "input\t", "action\t"))) + "\n").encode("ascii"):
        raise ValueError("water schedule input.tsv differs from the frozen native scenes")
    result["extension_version"] = VERSION
    return result


def verify_live(write: bool = False) -> dict:
    if not all(shutil.which(name) for name in ("javac", "java")):
        raise RuntimeError("live water schedule capture requires javac and java")
    jar = configured_jar()
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured original client JAR hash mismatch")
    with tempfile.TemporaryDirectory(prefix="water-schedule-native-") as temporary:
        temp = Path(temporary)
        first, second = capture_one(jar, temp / "first"), capture_one(jar, temp / "second")
    if first != second:
        raise AssertionError("independent native schedule JVM captures differ")
    result = parse_native(first)
    audit = {key: result[key] for key in ("case_count", "input_sha256", "native_sha256", "events")}
    audit.update({"extension_version": VERSION, "jar_sha256": JAR_SHA,
                  "oracle_source_sha256": digest(ORACLE.read_bytes()), "captures": 2,
                  "repeat_identical": True, "save_access": False})
    if write:
        FIXTURES.mkdir(parents=True, exist_ok=True)
        rows = [line for line in first.decode("ascii").splitlines()
                if line.startswith(("case\t", "input\t", "action\t"))]
        (FIXTURES / "input.tsv").write_text("\n".join(rows) + "\n", encoding="ascii")
        (FIXTURES / "native.tsv").write_bytes(first)
        (FIXTURES / "audit.json").write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    elif first != (FIXTURES / "native.tsv").read_bytes():
        raise AssertionError("live native schedule output differs from committed fixture")
    return audit


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", action="store_true", help="run two independent native schedule captures")
    parser.add_argument("--write", action="store_true", help="freeze the fixture after identical captures")
    args = parser.parse_args()
    if args.write and not args.live:
        parser.error("--write requires --live")
    result = verify_live(write=args.write) if args.live else fixture_check()
    print(json.dumps({key: value for key, value in result.items() if key != "cases"}, indent=2))
