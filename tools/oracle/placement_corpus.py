"""Reproduce bounded native placement actions in disposable memory chunks."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
SOURCE = ROOT / "tools/oracle/src/PlacementOracle.java"
FIXTURE_DIR = ROOT / "tests/fixtures/block-placement"
OUTPUT = FIXTURE_DIR / "native.tsv"
AUDIT = FIXTURE_DIR / "audit.json"
FIELDS = ("id", "target_id", "target_meta", "x", "y", "z", "side", "raycast_kind", "raycast_side", "actions",
          "adjacent_before", "adjacent_after", "stack_before", "stack_after",
          "damage_before", "damage_after", "height_before", "height_after",
          "sky_before", "sky_after", "blocklight_before", "blocklight_after",
          "neighborhood_before", "neighborhood_after")
ORDER = tuple([f"target{bid}_side{side}" for bid in (1, 31) for side in range(6)] +
              ["repeat_top", "occupied_top", "chunk_edge_east", "place_at_127"])
EXPECTED_CLASSES = {
    "fd": "b91d4726c43ed2dad1a72fd8d925eaded8a7c365af5d9f37c3ad101b957a4100",
    "lm": "ce93bdfff48935ca907ef111233cab0645dfb720ec9e9b5795fef65e9d093aa2",
    "uu": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "gm": "f9d971a83efa0c6399f1fc4bc0bc06c6a6249908afc6b46f5889a49365c190c7",
    "ck": "fe3d757cd56ac42ef8e0c0aaecf5fc7878406fe66db88ecf8061456f50cdd2eb",
    "iz": "3e8a07bf8c43729c8367bab454af876e698505620f230e012432c04e3828d6d6",
    "ob": "c367f0a0b4467234a3d30690106b2adde8ad74f7c80db285fefe1f75101c3147",
    "gs": "52e2d97748d9a306703e018822fece7d0ef9ca8ddf3448a1d0e568f2fbe29202",
}


def sha(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def parse(raw: bytes) -> list[dict[str, str]]:
    if len(raw) > 128 * 1024 or not raw.endswith(b"\n") or b"\r" in raw:
        raise ValueError("placement native TSV exceeds limits or has invalid line endings")
    lines = raw.decode("ascii").splitlines()
    if len(lines) != len(ORDER) + 2 or lines[0] != f"#BLOCK_PLACE_NATIVE\t1\t{JAR_SHA}":
        raise ValueError("placement native TSV has wrong schema, JAR, or case count")
    if tuple(lines[1].removeprefix("#").split("\t")) != FIELDS:
        raise ValueError("placement native TSV fields differ")
    rows = []
    int_fields = {"x", "y", "z", "target_id", "target_meta", "stack_before", "stack_after",
                  "damage_before", "damage_after", "height_before", "height_after",
                  "sky_before", "sky_after", "blocklight_before", "blocklight_after"}
    for line, case_id in zip(lines[2:], ORDER, strict=True):
        columns = line.split("\t")
        if len(columns) != len(FIELDS) or columns[0] != case_id:
            raise ValueError("placement case rows are malformed, duplicated, or reordered")
        row = dict(zip(FIELDS, columns, strict=True))
        if row["target_id"] not in {"1", "31"} or row["target_meta"] != "0":
            raise ValueError("unexpected native target block state")
        if row["side"] not in {"0", "1", "2", "3", "4", "5"}:
            raise ValueError("native placement side is invalid")
        if row["raycast_kind"] not in {"block", "miss", "other"} or row["raycast_side"] not in {"-", "0", "1", "2", "3", "4", "5"}:
            raise ValueError("native targeting result is invalid")
        if any(not row[key].lstrip("-").isdigit() or row[key].startswith("-") and not row[key][1:]
               for key in int_fields):
            raise ValueError("native placement integer field is invalid")
        if not row["actions"] or any(v not in {"true", "false"} for v in row["actions"].split(",")):
            raise ValueError("native action result is invalid")
        for name in ("neighborhood_before", "neighborhood_after"):
            cells = row[name].split(",")
            if len(cells) != 27 or any("=" not in cell or ":" not in cell for cell in cells):
                raise ValueError("native neighborhood snapshot is incomplete")
        rows.append(row)
    return rows


def capture() -> tuple[bytes, dict]:
    jar = (ROOT / "../.local/minecraft/client-b1.7.3.jar").resolve(strict=True)
    if sha(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured pinned Beta 1.7.3 JAR hash mismatch")
    source_hash = sha(SOURCE.read_bytes())
    with tempfile.TemporaryDirectory(prefix="block-placement-native-") as temp:
        classes = Path(temp) / "classes"
        classes.mkdir()
        subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(SOURCE)],
                       check=True, capture_output=True, timeout=40)
        proc = subprocess.run(["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
                               f"{classes}:{jar}", "PlacementOracle", str(jar)],
                              check=True, capture_output=True, timeout=60)
    parse(proc.stdout)
    audit = json.loads(proc.stderr.decode("utf-8"))
    if (audit.get("jar_sha256") != JAR_SHA or audit.get("classes") != EXPECTED_CLASSES
            or audit.get("native_cases") != len(ORDER) or audit.get("captures") != 2
            or audit.get("repeat_identical") is not True or audit.get("world_constructors") != 0
            or audit.get("save_access") is not False or audit.get("selected_item_id") != 1
            or audit.get("selected_block_id") != 1):
        raise ValueError("placement native provenance or isolation audit mismatch")
    audit["oracle_source_sha256"] = source_hash
    return proc.stdout, audit


def verify(write: bool = False) -> dict:
    first, audit = capture()
    second, repeat_audit = capture()
    if first != second or audit != repeat_audit:
        raise AssertionError("independent native placement captures differ")
    audit["output_sha256"] = sha(first)
    audit["independent_repeat_identical"] = True
    if write:
        FIXTURE_DIR.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(first)
        AUDIT.write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n")
    elif not OUTPUT.exists() or OUTPUT.read_bytes() != first:
        raise AssertionError("live native placement differs from retained fixture")
    elif not AUDIT.exists() or json.loads(AUDIT.read_text()) != audit:
        raise AssertionError("live native placement audit differs from retained provenance")
    return {"cases": len(parse(first)), "sha256": sha(first), "repeat_identical": True,
            "actions": "native confirmed in disposable worlds", "jar_sha256": JAR_SHA}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    print(json.dumps(verify(parser.parse_args().write), indent=2))
