#!/usr/bin/env python3
"""Capture and verify a compact native Beta 1.7.3 in-water player corpus."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import tomllib
import zipfile

try:
    from . import movement_corpus as move
except ImportError:  # direct script invocation places tools/oracle on sys.path
    import movement_corpus as move

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/water_movement"
JAVA_SOURCE = ROOT / "tools/oracle/src/WaterMovementOracle.java"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASS_NAMES = ("sn", "ls", "gs", "eq", "fd", "uu", "ln", "rp", "bt", "in")
METHODS = {
    "sn.U()V": ("sn", "U", "()V"),
    "sn.k_()Z": ("sn", "k_", "()Z"),
    "sn.a(Lln;)Z": ("sn", "a", "(Lln;)Z"),
    "fd.a(Leq;Lln;Lsn;)Z": ("fd", "a", "(Leq;Lln;Lsn;)Z"),
    "sn.a(FFF)V": ("sn", "a", "(FFF)V"),
    "sn.b(DDD)V": ("sn", "b", "(DDD)V"),
    "ls.a_(FF)V": ("ls", "a_", "(FF)V"),
    "gs.a_(FF)V": ("gs", "a_", "(FF)V"),
    "ls.R()V": ("ls", "R", "()V"),
    "rp.d(I)F": ("rp", "d", "(I)F"),
    "in.b(D)I": ("in", "b", "(D)I"),
    "in.c(F)F": ("in", "c", "(F)F"),
    "in.a(F)F": ("in", "a", "(F)F"),
    "in.b(F)F": ("in", "b", "(F)F"),
    "uu.a(Lfd;IIILsn;Lbt;)V": ("uu", "a", "(Lfd;IIILsn;Lbt;)V"),
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def raw32(value: float) -> str:
    return struct.pack(">f", move.f32(value)).hex()


def cell_rows(cells: list[tuple[int, int, int, int, int]]) -> str:
    return ";".join(",".join(str(v) for v in cell) for cell in cells) or "-"


def box_row(boxes: list[tuple[float, ...]]) -> str:
    return ":".join(move.box(*box) for box in boxes) or "-"


def frame(strafe: float, forward: float, yaw: float = 0.0,
          jump: bool = False, sneak: bool = False) -> str:
    return ",".join((raw32(strafe), raw32(forward), raw32(yaw),
                     "1" if jump else "0", "1" if sneak else "0"))


def player_state(x=0.5, feet=1.0, z=0.5, velocity=(0.0, 0.0, 0.0), yaw=0.0,
                 flags=129, bbox=None) -> str:
    # The player anchors positionY at feet + native yOffset (1.62f).
    return move.state(x=x, y=feet + 1.62, z=z, velocity=velocity,
                      y_offset=1.62, flags=flags, yaw=yaw, bbox=bbox)


def row(case_id: str, state: str, water: list[tuple[int, int, int, int, int]],
        solids: list[tuple[float, ...]], frames: list[str]) -> str:
    return "\t".join((case_id, state, cell_rows(water), box_row(solids), ";".join(frames)))


def water_cells(xs: range, ys: range, zs: range, meta=0, block_id=9):
    return [(x, y, z, block_id, meta) for x in xs for y in ys for z in zs]


def corpus() -> bytes:
    floor = [(-8.0, -8.0, -8.0, 8.0, 1.0, 8.0)]
    pool = water_cells(range(-2, 4), range(1, 4), range(-2, 3))
    rows = [
        row("entry-side", player_state(x=0.45, velocity=(0.40, 0.0, 0.0), yaw=90),
            water_cells(range(1, 5), range(1, 4), range(-2, 3)), floor,
            [frame(0, 1, 90) for _ in range(5)] + [frame(0, 0) for _ in range(3)]),
        row("submerged-diagonal-release", player_state(x=0.5, z=0.5), pool, floor,
            [frame(0.65, 0.65, 35) for _ in range(8)] + [frame(0, 0) for _ in range(8)]),
        row("vertical-jump-and-coast", player_state(x=0.5, feet=1.0), pool, floor,
            [frame(0, 0, 0, jump=True)] + [frame(0, 0) for _ in range(11)]),
        row("surface-feet-wet-eyes-dry", player_state(x=0.5, feet=1.93),
            water_cells(range(-2, 3), range(1, 2), range(-2, 3), meta=0, block_id=8), floor,
            [frame(0, 0) for _ in range(6)]),
        row("leave-water-to-shore", player_state(x=-0.15, velocity=(0.35, 0, 0), yaw=90),
            water_cells(range(-4, 1), range(1, 4), range(-2, 3)), floor,
            [frame(0, 0) for _ in range(8)] + [frame(0, 0) for _ in range(5)]),
        row("wall-and-floor", player_state(x=1.25, feet=1.0, velocity=(0.08, 0, 0), yaw=90),
            water_cells(range(0, 4), range(1, 4), range(-2, 3)),
            [(-8.0, 0.0, -8.0, 8.0, 1.0, 8.0), (2.0, 1.0, -2.0, 3.0, 4.0, 3.0)],
            [frame(0, 1, 90) for _ in range(8)] + [frame(0, 0) for _ in range(4)]),
        row("partial-fluid-level", player_state(x=0.5, feet=0.68),
            water_cells(range(-1, 2), range(1, 3), range(-1, 2), meta=4, block_id=9), floor,
            [frame(0, 0) for _ in range(5)]),
    ]
    data = ("\n".join(rows) + "\n").encode("ascii")
    if len(data) > 4 * 1024 * 1024 or len(rows) > 512:
        raise RuntimeError("water movement corpus exceeds bounded protocol")
    return data


def paths() -> Path:
    config = tomllib.loads((ROOT / "config/local.toml").read_text())["paths"]
    jar = config.get("client_jar", str(Path(config["minecraft_reference"]) / "client-b1.7.3.jar"))
    return (ROOT / jar).resolve(strict=True)


def class_method_code(data: bytes, wanted_name: str, wanted_desc: str) -> bytes:
    """Read one method's original Code bytes from a JVM classfile."""
    if data[:4] != b"\xca\xfe\xba\xbe":
        raise RuntimeError("not a JVM classfile")
    pos = 8
    cp_count = struct.unpack_from(">H", data, pos)[0]
    pos += 2
    cp: list[object | None] = [None] * cp_count
    index = 1
    while index < cp_count:
        tag = data[pos]
        pos += 1
        if tag == 1:
            size = struct.unpack_from(">H", data, pos)[0]
            pos += 2
            cp[index] = data[pos:pos + size].decode("utf-8", "replace")
            pos += size
        elif tag in (3, 4):
            pos += 4
        elif tag in (5, 6):
            pos += 8
            index += 1
        elif tag in (7, 8, 16, 19, 20):
            cp[index] = struct.unpack_from(">H", data, pos)[0]
            pos += 2
        elif tag in (9, 10, 11, 12, 17, 18):
            pos += 4
        elif tag == 15:
            pos += 3
        else:
            raise RuntimeError(f"unknown constant-pool tag {tag}")
        index += 1
    _, this_class, super_class = struct.unpack_from(">HHH", data, pos)
    pos += 6
    del this_class, super_class
    interface_count = struct.unpack_from(">H", data, pos)[0]
    pos += 2 + 2 * interface_count

    def skip_attributes(offset: int, count: int) -> int:
        for _ in range(count):
            _, length = struct.unpack_from(">HI", data, offset)
            offset += 6 + length
        return offset

    field_count = struct.unpack_from(">H", data, pos)[0]
    pos += 2
    for _ in range(field_count):
        _, _, _, attrs = struct.unpack_from(">HHHH", data, pos)
        pos += 8
        pos = skip_attributes(pos, attrs)
    method_count = struct.unpack_from(">H", data, pos)[0]
    pos += 2
    found = None
    for _ in range(method_count):
        _, name_index, desc_index, attr_count = struct.unpack_from(">HHHH", data, pos)
        pos += 8
        name, desc = cp[name_index], cp[desc_index]
        for _ in range(attr_count):
            attr_name = cp[struct.unpack_from(">H", data, pos)[0]]
            attr_len = struct.unpack_from(">I", data, pos + 2)[0]
            start = pos + 6
            if name == wanted_name and desc == wanted_desc and attr_name == "Code":
                code_len = struct.unpack_from(">I", data, start + 4)[0]
                found = data[start + 8:start + 8 + code_len]
            pos = start + attr_len
    if found is None:
        raise RuntimeError(f"native bytecode method absent: {wanted_name}{wanted_desc}")
    return found


def source_audit(jar: Path) -> tuple[dict, dict]:
    with zipfile.ZipFile(jar) as archive:
        classes = {name: sha(archive.read(name + ".class")) for name in CLASS_NAMES}
        methods = {label: sha(class_method_code(archive.read(class_name + ".class"), method_name, desc))
                   for label, (class_name, method_name, desc) in METHODS.items()}
    return classes, methods


def run_oracle(data: bytes, jar: Path, classes_dir: Path, audit_path: Path) -> tuple[bytes, dict]:
    result = subprocess.run(["java", "-cp", f"{classes_dir}:{jar}", "WaterMovementOracle",
                             str(jar), str(audit_path)], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, check=False, timeout=60)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("ascii", errors="replace"))
    if not result.stdout.endswith(b"\n"):
        raise RuntimeError("native output lacks final LF")
    return result.stdout, json.loads(audit_path.read_text())


def validate_output(data: bytes, output: bytes, audit: dict) -> None:
    input_rows = [line.decode("ascii").split("\t") for line in data.splitlines()]
    output_rows = [line.decode("ascii").split("\t") for line in output.splitlines()]
    expected = sum(len(row[4].split(";")) for row in input_rows)
    if len(output_rows) != expected or audit.get("state_count") != expected:
        raise RuntimeError("native tick state count differs from fixture schedule")
    if audit.get("case_count") != len(input_rows):
        raise RuntimeError("native scenario count differs from fixture")
    state_keys = [(row[0], int(row[1])) for row in output_rows]
    expected_keys = [(row[0], tick) for row in input_rows for tick in range(len(row[4].split(";")))]
    if state_keys != expected_keys:
        raise RuntimeError("native output case/tick order differs")
    in_water = sum(row[3] == "1" for row in output_rows)
    eye_water = sum(row[4] == "1" for row in output_rows)
    if in_water != audit.get("in_water_states") or eye_water != audit.get("eyes_in_water_states"):
        raise RuntimeError("native water state audit differs from emitted states")
    if any(len(row) != 7 or len(row[2].split(",")) != 18 or
           not row[5].isdecimal() or not row[6].isdecimal()
           for row in output_rows):
        raise RuntimeError("native state row has invalid schema")
    if audit.get("native_world_constructors") != 0:
        raise RuntimeError("synthetic world constructor was run")
    if audit.get("environment") != "synthetic-sparse-water-and-solid-boxes":
        raise RuntimeError("native run did not use declared synthetic world")
    counts = audit.get("invocations", {})
    if counts.get("entity_update") != expected or counts.get("player_travel") != expected:
        raise RuntimeError("native update/travel invocation totals differ")
    if counts.get("water_contact_queries") != expected:
        raise RuntimeError("native water contact query count differs from update schedule")
    if not counts.get("water_box_queries") or not counts.get("block_id_queries") or not counts.get("metadata_queries"):
        raise RuntimeError("native water queries were not exercised")


def run(write: bool = False) -> dict:
    data = corpus()
    jar = paths()
    if sha(jar.read_bytes()) != JAR_SHA:
        raise RuntimeError("unexpected original client JAR SHA-256")
    classes_expected, methods = source_audit(jar)
    with tempfile.TemporaryDirectory(prefix="water-movement-") as tmp:
        classes_dir = Path(tmp) / "classes"
        classes_dir.mkdir()
        subprocess.run(["javac", "-cp", str(jar), "-d", str(classes_dir), str(JAVA_SOURCE)],
                       check=True, timeout=60, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        first_path, second_path = Path(tmp) / "first.json", Path(tmp) / "second.json"
        first, audit_first = run_oracle(data, jar, classes_dir, first_path)
        second, audit_second = run_oracle(data, jar, classes_dir, second_path)
    if first != second or audit_first != audit_second:
        raise RuntimeError("independent native JVM captures differ")
    validate_output(data, first, audit_first)
    if audit_first.get("class_sha256") != classes_expected:
        raise RuntimeError("native class hashes differ from pinned archive contents")
    if audit_first.get("jar_sha256") != JAR_SHA:
        raise RuntimeError("native audit JAR hash mismatch")
    source_sha = sha(JAVA_SOURCE.read_bytes())
    audit = dict(audit_first)
    audit.update(input_sha256=sha(data), output_sha256=sha(first), repeat_output_sha256=sha(second),
                 oracle_sha256=source_sha, method_code_sha256=methods)
    if write:
        FIXTURE.mkdir(parents=True, exist_ok=True)
        (FIXTURE / "input.tsv").write_bytes(data)
        (FIXTURE / "reference.tsv").write_bytes(first)
        (FIXTURE / "audit.json").write_text(json.dumps(audit, indent=2) + "\n")
    else:
        for name, expected in (("input.tsv", data), ("reference.tsv", first)):
            if (FIXTURE / name).read_bytes() != expected:
                raise RuntimeError(f"committed {name} differs from native oracle")
        if json.loads((FIXTURE / "audit.json").read_text()) != audit:
            raise RuntimeError("committed water movement audit differs")
    return {"status": "PASS", "case_count": audit["case_count"],
            "state_count": audit["state_count"], "in_water_states": audit["in_water_states"],
            "eyes_in_water_states": audit["eyes_in_water_states"],
            "input_sha256": audit["input_sha256"], "output_sha256": audit["output_sha256"],
            "method_code_sha256": audit["method_code_sha256"]}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="explicitly capture and replace native fixture")
    args = parser.parse_args()
    print(json.dumps(run(args.write), indent=2))
