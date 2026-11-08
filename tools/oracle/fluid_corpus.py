#!/usr/bin/env python3
"""Verify/replay the pinned Beta client water-geometry corpus."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/fluid"
ORACLE = ROOT / "tools/oracle/src/FluidOracle.java"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
SUITE = ("FLUID-001-C01\tids=8,9\tmetadata=0..15\tneighbors="
         "east-source,west-flow,north-source,south-flow,diagonal-water,four-sources,"
         "water-above,flow-column,solid-east,solid-west,solid-north,solid-south")
SAMPLES = ([f"level-{block_id}-{meta}" for block_id in (8, 9) for meta in range(16)] + [
    "east-source", "west-flow", "north-source", "south-flow", "diagonal-water",
    "four-sources", "water-above", "flow-column", "solid-east", "solid-west",
    "solid-north", "solid-south",
])
EXPECTED_CLASSES = {
    "uu": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "cv": "ce8f363e1f7566530ac31680eabb8c715d9ddc7c97f31cf0a62f953e069a6863",
    "nw": "2facc45feb91d0fea0c5888ac6793a87ffa6e4cdf5b0cc4e418c6c162ca8bf33",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def parse_native(raw):
    text = raw.decode("ascii", errors="strict")
    lines = text.splitlines()
    if not lines or not lines[0].startswith("#FLUID_NATIVE\t1\t"):
        raise ValueError("native fluid header missing")
    fields = dict(field.split("=", 1) for field in lines[0].split("\t")[2:])
    if fields != {"jar": JAR_SHA, **EXPECTED_CLASSES}:
        raise ValueError("pinned fluid source or original classes changed")
    if len(lines) < 6 or not lines[1].startswith("#bootstrap\t"):
        raise ValueError("native class-bootstrap audit missing")
    bootstrap_fields = dict(field.split("=", 1) for field in lines[1].split("\t")[1:])
    bootstrap = bytes.fromhex("") if not bootstrap_fields["base64"] else __import__("base64").b64decode(bootstrap_fields["base64"], validate=True)
    if int(bootstrap_fields["bytes"]) != len(bootstrap) or bootstrap_fields["sha256"] != digest(bootstrap):
        raise ValueError("native registry bootstrap audit mismatch")
    suite_hash = digest(SUITE.encode("ascii"))
    if lines[2] != f"#input\t{SUITE}\tsha256={suite_hash}":
        raise ValueError("native suite input differs from the frozen C01 matrix")
    if lines[3] != "#setup\tsynthetic xp proxy; direct cv.b(uu,x,y,z); raw nw.g words; no native World/Chunk; no GL draw/flush":
        raise ValueError("native fluid invocation path is not documented")
    if lines[4] != f"#cases\t{len(SAMPLES)}":
        raise ValueError("native case count mismatch")
    cursor = 5
    cases = []
    vertices = {name: 0 for name in SAMPLES}
    while cursor < len(lines) and lines[cursor].startswith("case\t"):
        row = lines[cursor].split("\t")
        if len(row) != 6 or row[1] not in SAMPLES:
            raise ValueError("malformed native water case")
        cases.append(row)
        cursor += 1
        for _ in range(int(row[5])):
            if cursor >= len(lines):
                raise ValueError("truncated native vertex rows")
            vertex = lines[cursor].split("\t")
            if len(vertex) != 10 or vertex[0] != "vertex" or vertex[1] != row[1]:
                raise ValueError("native vertex row is not bound to its case")
            if any(len(word) != 8 or any(ch not in "0123456789abcdef" for ch in word) for word in vertex[2:]):
                raise ValueError("native vertex words must be lowercase raw 32-bit hex")
            vertices[row[1]] += 1
            cursor += 1
    if [row[1] for row in cases] != SAMPLES:
        raise ValueError("native water cases are incomplete or out of order")
    if any(row[4] != "1" or int(row[5]) == 0 for row in cases):
        raise ValueError("original fluid renderer rejected a required case")
    native_calls = f"#native_calls\tcv.b(uu,int,int,int)={len(SAMPLES)}\tgl_draw=0\tgl_flush=0\tworld_native_loads=0"
    if cursor >= len(lines) or lines[cursor] != native_calls:
        raise ValueError("native renderer invocation/no-GL audit mismatch")
    cursor += 1
    calls = []
    while cursor < len(lines) and lines[cursor].startswith("#world_call\t"):
        row = lines[cursor].split("\t")
        if len(row) != 3 or int(row[2]) <= 0:
            raise ValueError("invalid native world-call audit")
        calls.append(row)
        cursor += 1
    if not calls or cursor != len(lines):
        raise ValueError("native world-call audit missing or has extra output")
    return {"cases": len(cases), "vertices": sum(vertices.values()),
            "per_case_vertices": vertices, "native_calls": calls,
            "native_sha256": digest(raw), "repeatable": True}


def configured_jar():
    configured = os.environ.get("MINECRAFT_BETA_173_JAR")
    if configured:
        return Path(configured).expanduser().resolve(strict=True)
    config = ROOT / "config/local.toml"
    for line in config.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("client_jar") and "=" in line:
            return (ROOT / line.split("=", 1)[1].strip().strip('"')).resolve(strict=True)
    raise ValueError("config/local.toml has no client_jar")


def one_live(jar, run_dir):
    classes = run_dir / "classes"
    classes.mkdir(parents=True)
    compiled = subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(ORACLE)],
                              capture_output=True, timeout=40)
    if compiled.returncode:
        raise RuntimeError(compiled.stderr.decode(errors="replace")[-3000:])
    result = subprocess.run(["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
                             os.pathsep.join((str(classes), str(jar))), "FluidOracle", str(jar)],
                            capture_output=True, timeout=90)
    if result.returncode or result.stderr:
        raise RuntimeError("native fluid oracle failed/noisy: " + result.stderr.decode(errors="replace")[-3000:])
    parse_native(result.stdout)
    return result.stdout


def fixture_check(raw_override=None):
    raw = ((FIXTURES / "native.tsv").read_bytes() if raw_override is None else raw_override)
    result = parse_native(raw)
    audit = json.loads((FIXTURES / "audit.json").read_text(encoding="utf-8"))
    if audit.get("native_sha256") != digest(raw) or audit.get("jar_sha256") != JAR_SHA:
        raise ValueError("saved fluid audit does not bind native fixture and JAR")
    if audit.get("cases") != result["cases"] or audit.get("vertices") != result["vertices"]:
        raise ValueError("saved fluid audit counts mismatch")
    return result


def verify_live(write=False):
    if not all(shutil.which(name) for name in ("javac", "java")):
        raise RuntimeError("live fluid capture requires javac and java")
    jar = configured_jar()
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured original client JAR hash mismatch")
    with tempfile.TemporaryDirectory(prefix="fluid-native-") as temp_name:
        temp = Path(temp_name)
        first_dir, second_dir = temp / "first", temp / "second"
        first_dir.mkdir(); second_dir.mkdir()
        first, second = one_live(jar, first_dir), one_live(jar, second_dir)
        if first != second:
            raise AssertionError("independent original-client fluid captures differ")
    result = parse_native(first)
    audit = {**result, "jar_sha256": JAR_SHA, "suite_sha256": digest(SUITE.encode("ascii")),
             "oracle_source_sha256": digest(ORACLE.read_bytes()), "repeatable": True}
    if write:
        FIXTURES.mkdir(parents=True, exist_ok=True)
        (FIXTURES / "input.tsv").write_text(SUITE + "\n", encoding="ascii")
        (FIXTURES / "native.tsv").write_bytes(first)
        (FIXTURES / "audit.json").write_text(json.dumps(audit, indent=2) + "\n", encoding="utf-8")
    elif first != (FIXTURES / "native.tsv").read_bytes():
        raise AssertionError("live fluid output differs from committed fixture")
    return audit


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", action="store_true", help="opt into running the pinned original client")
    parser.add_argument("--write", action="store_true", help="write fixture after two identical live captures")
    args = parser.parse_args()
    if args.write and not args.live:
        parser.error("--write requires --live")
    print(json.dumps(verify_live(write=args.write) if args.live else fixture_check(), indent=2))
