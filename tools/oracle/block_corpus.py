"""Reproduce the pinned Beta 1.7.3 native block ray corpus."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASS_SHA = {
    "fd": "b91d4726c43ed2dad1a72fd8d925eaded8a7c365af5d9f37c3ad101b957a4100",
    "vf": "fb28ca5382c97f87e9eccdbb8d0618f50fc329cef1d65500647a918aa9c32cfe",
    "bt": "32018dcec6f1af1efa93baac09178456d9d2537367721875dae6547588a308f8",
    "uu": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "ob": "c367f0a0b4467234a3d30690106b2adde8ad74f7c80db285fefe1f75101c3147",
    "pj": "c2735d4e58bdbcf882d930f4b8925df7c218389e7a4c378319a444d303d92590",
    "iz": "3e8a07bf8c43729c8367bab454af876e698505620f230e012432c04e3828d6d6",
    "gm": "f9d971a83efa0c6399f1fc4bc0bc06c6a6249908afc6b46f5889a49365c190c7",
}
FIXTURE = ROOT / "tests/fixtures/block/native.tsv"
INPUT_FIXTURE = ROOT / "tests/fixtures/block/input.tsv"
AUDIT = ROOT / "tests/fixtures/block/audit.json"
FIELDS = ("id", "source", "kind", "x", "y", "z", "side", "px", "py", "pz",
          "px_bits", "py_bits", "pz_bits")
EXPECTED_ORDER = ("face_west", "face_east", "face_down", "face_up", "face_north", "face_south",
                  "miss", "negative_voxel", "tie_corner", "endpoint_before", "endpoint_face",
                  "shape_slab", "liquid_stop_false", "liquid_stop_true", "range_before", "range_at", "range_after")
EXPECTED_IDS = set(EXPECTED_ORDER)
INPUT_FIELDS = ("id", "sx", "sy", "sz", "ex", "ey", "ez", "stop_liquid",
                "block_x", "block_y", "block_z", "block_id", "metadata", "shape")
INPUT_HEADER = (b"#BLOCK_CASES\t1\tray-only\n#id\tsx\tsy\tsz\tex\tey\tez\tstop_liquid\t"
                b"block_x\tblock_y\tblock_z\tblock_id\tmetadata\tshape\n")


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def parse(raw: bytes) -> list[dict[str, str]]:
    if len(raw) > 64 * 1024 or not raw.endswith(b"\n") or b"\r" in raw:
        raise ValueError("native block TSV violates size or line-ending limits")
    try:
        lines = raw.decode("ascii").splitlines()
    except UnicodeDecodeError as exc:
        raise ValueError("native block TSV must be ASCII") from exc
    if len(lines) < 3 or lines[0] != f"#BLOCK_NATIVE\t1\t{JAR_SHA}":
        raise ValueError("wrong native block header/JAR")
    if tuple(lines[1].removeprefix("#").split("\t")) != FIELDS:
        raise ValueError("wrong native block header fields")
    rows = []
    for line in lines[2:]:
        columns = line.split("\t")
        if len(columns) != len(FIELDS):
            raise ValueError("malformed block ray row")
        row = dict(zip(FIELDS, columns, strict=True))
        if row["source"] != "live" or row["kind"] not in {"block", "miss"}:
            raise ValueError("native fixture row has wrong source or hit kind")
        if row["kind"] == "miss":
            if any(row[k] != "-" for k in FIELDS[3:]):
                raise ValueError("miss row must not contain hit data")
        else:
            if row["side"] not in {"0", "1", "2", "3", "4", "5"}:
                raise ValueError("invalid native hit side")
            for k in ("x", "y", "z"):
                if not row[k].lstrip("-").isdigit():
                    raise ValueError("invalid native block coordinate")
            for k in ("px_bits", "py_bits", "pz_bits"):
                if len(row[k]) != 16 or any(c not in "0123456789abcdef" for c in row[k]):
                    raise ValueError("invalid native raw double word")
        rows.append(row)
    if tuple(r["id"] for r in rows) != EXPECTED_ORDER:
        raise ValueError("native block case set/order incomplete or duplicated")
    return rows


def parse_inputs(raw: bytes) -> list[dict[str, str]]:
    if len(raw) > 4096 or not raw.endswith(b"\n") or b"\r" in raw or not raw.startswith(INPUT_HEADER):
        raise ValueError("wrong or over-limit block case input header")
    try:
        lines = raw.decode("ascii").splitlines()
    except UnicodeDecodeError as exc:
        raise ValueError("block case input must be ASCII") from exc
    rows = []
    for line, expected_id in zip(lines[2:], EXPECTED_ORDER, strict=True):
        cols = line.split("\t")
        if len(cols) != len(INPUT_FIELDS) or cols[0] != expected_id:
            raise ValueError("block input cases malformed, duplicated, extra, or out of order")
        row = dict(zip(INPUT_FIELDS, cols, strict=True))
        for key in ("sx", "sy", "sz", "ex", "ey", "ez"):
            try:
                number = float(row[key])
            except ValueError as exc:
                raise ValueError("block ray endpoint is not a decimal double") from exc
            if not __import__("math").isfinite(number):
                raise ValueError("block ray endpoint must be finite")
        for key in ("stop_liquid", "block_x", "block_y", "block_z", "block_id", "metadata"):
            if not row[key].lstrip("-").isdigit():
                raise ValueError("block scene integer is malformed")
        if row["stop_liquid"] not in {"0", "1"}:
            raise ValueError("stop_liquid must be 0 or 1")
        if row["shape"] not in {"cube", "slab", "liquid"}:
            raise ValueError("unknown fixture shape")
        bounds = {"block_x": (-(2**31), 2**31-1), "block_y": (0, 127),
                  "block_z": (-(2**31), 2**31-1), "block_id": (0, 255), "metadata": (0, 15)}
        for key, (low, high) in bounds.items():
            if not low <= int(row[key]) <= high:
                raise ValueError("block scene integer outside native bounds")
        rows.append(row)
    if len(lines) != len(EXPECTED_ORDER) + 2:
        raise ValueError("block input has wrong case count")
    return rows


def capture() -> tuple[bytes, bytes, dict]:
    jar = (ROOT / "../.local/minecraft/client-b1.7.3.jar").resolve(strict=True)
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("pinned client JAR hash mismatch")
    source = ROOT / "tools/oracle/src/BlockOracle.java"
    source_sha = digest(source.read_bytes())
    with tempfile.TemporaryDirectory(prefix="block-native-") as temp:
        classes = Path(temp) / "classes"
        classes.mkdir()
        subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(source)],
                       check=True, capture_output=True, timeout=40)
        result = subprocess.run(["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", f"{classes}:{jar}",
                                 "BlockOracle", str(jar)], check=True, capture_output=True, timeout=60)
        inputs = subprocess.run(["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", f"{classes}:{jar}",
                                 "BlockOracle", str(jar), "--inputs"], check=True, capture_output=True, timeout=60)
    parse(result.stdout)
    parse_inputs(inputs.stdout)
    if inputs.stderr:
        raise ValueError("native input generator wrote unexpected stderr")
    audit = json.loads(result.stderr.decode("utf-8"))
    if audit.get("jar_sha256") != JAR_SHA or audit.get("classes") != CLASS_SHA:
        raise ValueError("native block class hash mapping mismatch")
    if audit.get("world_constructors") != 0 or audit.get("native_cases") != len(EXPECTED_IDS):
        raise ValueError("unexpected native world construction or case count")
    audit["oracle_source_sha256"] = source_sha
    audit["input_sha256"] = digest(inputs.stdout)
    return result.stdout, inputs.stdout, audit


def verify(write: bool = False) -> dict:
    first, input_data, audit = capture()
    second, repeat_input, repeat_audit = capture()
    if first != second or input_data != repeat_input or audit != repeat_audit:
        raise AssertionError("two original-client ray captures differ")
    audit["output_sha256"] = digest(first)
    audit["repeat_output_sha256"] = digest(second)
    audit["repeat_identical"] = True
    audit["actions"] = {
        "break": "not_verified",
        "place": "not_verified",
        "limitation": "No isolated native Player/controller and disposable edit adapter was completed; no action result is claimed.",
    }
    if write:
        FIXTURE.parent.mkdir(parents=True, exist_ok=True)
        FIXTURE.write_bytes(first)
        INPUT_FIXTURE.write_bytes(input_data)
        AUDIT.write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n")
    elif not FIXTURE.exists() or FIXTURE.read_bytes() != first:
        raise AssertionError("native block capture differs from committed fixture; use --write after review")
    elif not AUDIT.exists() or json.loads(AUDIT.read_text()) != audit:
        raise AssertionError("native block audit differs from committed evidence")
    elif not INPUT_FIXTURE.exists() or INPUT_FIXTURE.read_bytes() != input_data:
        raise AssertionError("native input definitions differ from committed C01 fixture")
    return {"cases": len(parse(first)), "sha256": digest(first), "input_sha256": digest(input_data), "repeat_identical": True,
            "jar_sha256": JAR_SHA, "actions": "not_verified"}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    print(json.dumps(verify(args.write), indent=2))
