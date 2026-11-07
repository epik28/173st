#!/usr/bin/env python3
"""Generated wire inputs compared with native world-data extraction, separately."""

import argparse
import gzip
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import zlib

from run import sha256


JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
REGIONS = ("r.-1.-1.mcr", "r.-1.0.mcr", "r.0.-1.mcr", "r.0.0.mcr")


def mutf(text):
    """Input generation only; expected results always come from the native JAR."""
    output = bytearray()
    encoded = text.encode("utf-16-be")
    for offset in range(0, len(encoded), 2):
        unit = int.from_bytes(encoded[offset:offset + 2], "big")
        if 1 <= unit <= 127:
            output.append(unit)
        elif unit <= 2047:
            output.extend((0xc0 | unit >> 6, 0x80 | unit & 63))
        else:
            output.extend((0xe0 | unit >> 12, 0x80 | unit >> 6 & 63, 0x80 | unit & 63))
    return struct.pack(">H", len(output)) + output


def payload(kind, value):
    if kind in (1, 2, 3, 4, 5, 6):
        return struct.pack({1: ">b", 2: ">h", 3: ">i", 4: ">q", 5: ">f", 6: ">d"}[kind], value)
    if kind == 7:
        return struct.pack(">i", len(value)) + value
    if kind == 8:
        return mutf(value)
    if kind == 9:
        element, values = value
        return bytes([element]) + struct.pack(">i", len(values)) + b"".join(payload(element, item) for item in values)
    if kind == 10:
        return b"".join(bytes([ty]) + mutf(name) + payload(ty, item)
                        for name, (ty, item) in value.items()) + b"\0"
    raise ValueError(f"unsupported generated wire type {kind}")


def root(fields):
    return b"\x0a\0\0" + payload(10, fields)


def generated_cases():
    cases = []

    def level(name, data, valid=True, *, packed=None):
        raw = root({"Data": (10, data)})
        cases.append(dict(name=name, kind="level", raw=raw,
                          packed=gzip.compress(raw, mtime=0) if packed is None else packed,
                          valid=valid, x=0, z=0))

    base = {"RandomSeed": (4, -(2**63)), "SpawnX": (3, -(2**31)),
            "SpawnY": (3, 2**31 - 1), "SpawnZ": (3, -1), "Time": (4, 2**63 - 1),
            "LastPlayed": (4, -(2**63)), "SizeOnDisk": (4, 2**63 - 1),
            "LevelName": (8, "world\t☃😀\0"), "version": (3, 19132),
            "rainTime": (3, -(2**31)), "raining": (1, -128),
            "thunderTime": (3, 2**31 - 1), "thundering": (1, -1)}
    player = {"Dimension": (3, -1), "Pos": (9, (6, [1.25, 64.0, -3.5])),
              "Rotation": (9, (5, [90.0, -0.0])), "Inventory": (9, (1, [])),
              "Health": (2, 19), "UnknownPlayer": (5, float("nan"))}
    level("missing-defaults", {})
    level("numeric-bounds-and-unicode", base)
    level("empty-name-and-player", dict(base, LevelName=(8, ""), Player=(10, {})))
    level("player-and-unknown-fields", dict(base, Player=(10, player), **{"": (3, 1), "extra😀": (8, "retained")}))
    for field, wrong in (("RandomSeed", (3, 42)), ("raining", (3, 1)),
                         ("Player", (3, 0)), ("LevelName", (4, 0))):
        level("wrong-" + field, dict(base, **{field: wrong}), False)
    for name, raw in (("missing-Data", root({})), ("wrong-Data", root({"Data": (3, 0)})),
                      ("trailing-NBT", root({"Data": (10, base)}) + b"\0")):
        cases.append(dict(name=name, kind="level", raw=raw, packed=gzip.compress(raw, mtime=0), valid=False, x=0, z=0))
    raw = root({"Data": (10, base)})
    packed = gzip.compress(raw, mtime=0)
    for name, encoded, valid in (
        ("truncated-gzip", packed[:-2], False),
        ("corrupt-gzip", packed[:-1] + bytes([packed[-1] ^ 1]), False),
        ("gzip-trailing", packed + b"trailing", True),
        ("gzip-partial-next-header", packed + b"\x1f\x8b", True),
        ("gzip-empty-next-member", packed + gzip.compress(b"", mtime=0), True),
        ("gzip-extra-NBT-member", packed + gzip.compress(root({}), mtime=0), False),
        ("gzip-output-limit", gzip.compress(b"\0" * (64 * 1024 * 1024 + 1), mtime=0), False),
    ):
        cases.append(dict(name=name, kind="level", raw=raw, packed=encoded, valid=valid, x=0, z=0))

    arrays = {"Blocks": (7, bytes((i * 37 + 3) & 255 for i in range(32768))),
              **{name: (7, bytes((i * 19 + 128) & 255 for i in range(16384)))
                 for name in ("Data", "BlockLight", "SkyLight")},
              "HeightMap": (7, bytes(i % 129 for i in range(256)))}
    chunk_base = dict(arrays, xPos=(3, 0), zPos=(3, 0), LastUpdate=(4, 2**63 - 1),
                      TerrainPopulated=(1, -128),
                      Entities=(9, (10, [{"id": (8, "future_entity"), "extra": (7, b"\x80\xff")}])) ,
                      TileEntities=(9, (10, [{"id": (8, "future_tile"), "future": (8, "saved")}])) )

    def chunk(name, fields, valid=True, *, x=0, z=0, raw_override=None):
        raw = root({"Level": (10, fields), "RootExtra": (8, "unchanged")}) if raw_override is None else raw_override
        cases.append(dict(name=name, kind="chunk", raw=raw, valid=valid, x=x, z=z))

    chunk("unsigned-arrays-and-unknown-entities", chunk_base)
    chunk("negative-coordinates-and-extra-fields", dict(chunk_base, xPos=(3, -1), zPos=(3, 2),
          **{"": (3, 1), "TileTicks": (9, (10, [{"id": (3, 1)}])), "extra☃": (8, "data")}), x=-1, z=2)
    chunk("empty-byte-entity-lists", dict(chunk_base, Entities=(9, (1, [])), TileEntities=(9, (1, []))))
    chunk("missing-optional-chunk-fields", {k: v for k, v in chunk_base.items()
          if k not in {"LastUpdate", "TerrainPopulated", "Entities", "TileEntities"}})
    for field in ("xPos", "zPos", "Blocks", "Data", "BlockLight", "SkyLight", "HeightMap"):
        chunk("missing-" + field, {k: v for k, v in chunk_base.items() if k != field}, False)
    for field in arrays:
        chunk("short-" + field, dict(chunk_base, **{field: (7, b"\0")}), False)
    for field, wrong in (("Blocks", (3, 0)), ("TerrainPopulated", (3, 1)),
                         ("LastUpdate", (3, 1)), ("Entities", (3, 0)),
                         ("Entities", (9, (3, [1])))):
        chunk("wrong-" + field + "-" + str(wrong[0]) + ("-list" if wrong[0] == 9 else ""),
              dict(chunk_base, **{field: wrong}), False)
    chunk("coordinate-mismatch", dict(chunk_base, xPos=(3, 1)), False)
    chunk("missing-Level", {}, False, raw_override=root({}))
    cases.append(dict(name="absent-slot", kind="absent", raw=None, valid=True, x=0, z=0))
    for case in cases:
        case["name"] = case["kind"] + "-" + case["name"]
    return cases


def region_bytes(raw, x, z):
    record = bytes([2]) + zlib.compress(raw)
    count = (4 + len(record) + 4095) // 4096
    result = bytearray((2 + count) * 4096)
    slot = x % 32 + (z % 32) * 32
    result[slot * 4:slot * 4 + 4] = ((2 << 8) | count).to_bytes(4, "big")
    result[8192:8196] = len(record).to_bytes(4, "big")
    result[8196:8196 + len(record)] = record
    return bytes(result)


def review(report_path):
    report_path = Path(report_path).resolve(strict=True)
    report = json.loads(report_path.read_text())
    if report.get("suite") != "world" or report.get("status") != "pass":
        raise ValueError("requires successful world live report")
    directory = report_path.parent
    jar, binary = Path(report["reference_jar_path"]), directory / "world_probe"
    if sha256(jar.read_bytes()) != JAR_SHA or sha256(binary.read_bytes()) != report["rust_binary_sha256"]:
        raise ValueError("saved binary/JAR hash mismatch")
    commands = {"java": ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", f"{directory / 'classes'}:{jar}", "WorldOracle", str(jar)],
                "rust": [str(binary)]}
    output = Path(tempfile.mkdtemp(prefix="world-edges-", dir=directory))
    results = []
    default_level = gzip.compress(root({"Data": (10, {})}), mtime=0)
    for case in generated_cases():
        name, kind, x, z = case["name"], case["kind"], case["x"], case["z"]
        filename = "level.dat" if kind == "level" else f"r.{x // 32}.{z // 32}.mcr"
        row = (f"{name}\tlevel\tlevel.dat\t-\t-\n" if kind == "level"
               else f"{name}\tchunk\t{filename}\t{x}\t{z}\n").encode("ascii")
        item = {"name": name, "kind": "generated; not live world parity", "expected_valid": case["valid"], "sides": {}}
        stdouts = {}
        for side, base_command in commands.items():
            parent = output / name / side
            inputs = parent / "input"
            inputs.mkdir(parents=True)
            (inputs / ".world-input-copy").write_text("WORLD-LOAD-001 copied inputs\n")
            (inputs / ".region-input-copy").write_text("REGION-001 copied inputs\n")
            (inputs / "level.dat").write_bytes(case["packed"] if kind == "level" else default_level)
            for region in REGIONS:
                (inputs / region).write_bytes(bytes(8192))
            if kind == "chunk":
                (inputs / filename).write_bytes(region_bytes(case["raw"], x, z))
            before = {p.name: sha256(p.read_bytes()) for p in inputs.iterdir()}
            retained = parent / "retained"
            command = base_command + [str(inputs), str(retained)]
            if side == "java":
                command.append(str(parent / "native-audit.json"))
            try:
                process = subprocess.run(command, input=row, capture_output=True, timeout=30)
                (parent / "stdout.tsv").write_bytes(process.stdout)
                (parent / "stderr.log").write_bytes(process.stderr)
                after = {p.name: sha256(p.read_bytes()) for p in inputs.iterdir()}
                accepted = (process.returncode == 0 if case["valid"] else process.returncode != 0 and bool(process.stderr))
                item["sides"][side] = {"returncode": process.returncode, "copy_unchanged": before == after,
                                       "accepted": bool(accepted and before == after)}
                if case["valid"]:
                    files = {p.name: p.read_bytes() for p in retained.glob("*.nbt")}
                    expected = {} if kind == "absent" else {name + ".nbt": case["raw"]}
                    item["sides"][side]["retained_exact"] = files == expected
                    item["sides"][side]["accepted"] &= files == expected
                stdouts[side] = process.stdout
            except (OSError, subprocess.TimeoutExpired) as error:
                item["sides"][side] = {"accepted": False, "error": str(error)}
        item["accepted"] = all(value["accepted"] for value in item["sides"].values())
        if case["valid"] and item["accepted"]:
            item["exact_outputs"] = stdouts["java"] == stdouts["rust"]
            item["accepted"] &= item["exact_outputs"]
        results.append(item)
    failures = [item["name"] for item in results if not item["accepted"]]
    evidence = {"kind": "generated wire and shared strict validation; not full World simulation",
                "live_report": str(report_path), "results": results, "failures": failures}
    (output / "review.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(f"{'FAIL' if failures else 'PASS'}: {len(results)} world cases/{len(results) * 2} executions; {output / 'review.json'}")
    return 1 if failures else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    raise SystemExit(review(parser.parse_args().report))
