"""Capture native ID-31 selection-boundary rays in isolated memory chunks."""
from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
SOURCE = ROOT / "tools/oracle/src/GrassRayOracle.java"
FIXTURE = ROOT / "tests/fixtures/block-placement/grass-rays-native.tsv"
INPUT = ROOT / "tests/fixtures/block-placement/grass-rays-input.tsv"
AUDIT = ROOT / "tests/fixtures/block-placement/grass-rays-audit.json"
IDS = ("x_mid_inside", "x_above_height", "x_min_exact_along_z", "x_min_below_along_z",
       "x_max_exact_along_z", "x_max_above_along_z", "y_bottom_exact", "y_top_exact",
       "vertical_x_below", "vertical_x_min_exact", "vertical_x_max_exact", "vertical_x_above",
       "diagonal_inside", "diagonal_above")
OUTPUT_FIELDS = ("id", "kind", "x", "y", "z", "side", "px", "py", "pz", "px_bits", "py_bits", "pz_bits")
INPUT_FIELDS = ("id", "sx", "sy", "sz", "ex", "ey", "ez")
CLASS_SHA = {
    "fd": "b91d4726c43ed2dad1a72fd8d925eaded8a7c365af5d9f37c3ad101b957a4100",
    "bt": "32018dcec6f1af1efa93baac09178456d9d2537367721875dae6547588a308f8",
    "vf": "fb28ca5382c97f87e9eccdbb8d0618f50fc329cef1d65500647a918aa9c32cfe",
    "uu": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "ru": "798fb8e40028cb1a9d031690c5ee44adb4fb2af14b75778d857290dcfb7371eb",
}


def sha(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def parse_output(raw: bytes) -> list[dict[str, str]]:
    if len(raw) > 32 * 1024 or not raw.endswith(b"\n") or b"\r" in raw:
        raise ValueError("grass ray native TSV size or line endings invalid")
    lines = raw.decode("ascii").splitlines()
    if len(lines) != len(IDS) + 2 or lines[0] != f"#BLOCK_GRASS_RAY_NATIVE\t1\t{JAR_SHA}":
        raise ValueError("grass ray native header or row count mismatch")
    if tuple(lines[1].removeprefix("#").split("\t")) != OUTPUT_FIELDS:
        raise ValueError("grass ray native fields mismatch")
    rows = []
    for line, expected_id in zip(lines[2:], IDS, strict=True):
        cols = line.split("\t")
        if len(cols) != len(OUTPUT_FIELDS) or cols[0] != expected_id:
            raise ValueError("grass ray native rows reordered or malformed")
        row = dict(zip(OUTPUT_FIELDS, cols, strict=True))
        if row["kind"] not in {"block", "miss"}:
            raise ValueError("invalid grass ray hit kind")
        if row["kind"] == "miss":
            if any(row[k] != "-" for k in OUTPUT_FIELDS[2:]):
                raise ValueError("grass ray miss carries hit data")
        else:
            if row["side"] not in {"0", "1", "2", "3", "4", "5"}:
                raise ValueError("invalid grass ray face")
            if tuple(row[k] for k in ("x", "y", "z")) != ("0", "10", "0"):
                raise ValueError("grass ray hit is outside fixture block")
            for key in ("px", "py", "pz"):
                if not math.isfinite(float(row[key])):
                    raise ValueError("grass ray hit position is not finite")
            for key in ("px_bits", "py_bits", "pz_bits"):
                if len(row[key]) != 16 or any(c not in "0123456789abcdef" for c in row[key]):
                    raise ValueError("invalid raw position word")
            for value_key, bits_key in (("px", "px_bits"), ("py", "py_bits"), ("pz", "pz_bits")):
                if struct.pack(">d", float(row[value_key])).hex() != row[bits_key]:
                    raise ValueError("raw position word does not encode reported hit position")
        rows.append(row)
    expected = {"x_mid_inside": ("block", "4"), "x_above_height": ("miss", "-"),
                "x_min_exact_along_z": ("block", "2"), "x_min_below_along_z": ("miss", "-"),
                "x_max_exact_along_z": ("block", "2"), "x_max_above_along_z": ("miss", "-"),
                "y_bottom_exact": ("block", "0"), "y_top_exact": ("block", "1"),
                "vertical_x_below": ("miss", "-"), "vertical_x_min_exact": ("block", "0"),
                "vertical_x_max_exact": ("block", "0"), "vertical_x_above": ("miss", "-"),
                "diagonal_inside": ("block", "4"), "diagonal_above": ("miss", "-")}
    if {r["id"]: (r["kind"], r["side"]) for r in rows} != expected:
        raise ValueError("native grass rays differ from expected shape boundaries")
    return rows


def parse_input(raw: bytes) -> list[dict[str, str]]:
    if len(raw) > 8192 or not raw.endswith(b"\n") or b"\r" in raw:
        raise ValueError("grass ray input TSV size or line endings invalid")
    lines = raw.decode("ascii").splitlines()
    if len(lines) != len(IDS) + 2 or lines[0] != f"#BLOCK_GRASS_RAY_INPUT\t1\t{JAR_SHA}":
        raise ValueError("grass ray input header or row count mismatch")
    if tuple(lines[1].removeprefix("#").split("\t")) != INPUT_FIELDS:
        raise ValueError("grass ray input fields mismatch")
    rows = []
    for line, expected_id in zip(lines[2:], IDS, strict=True):
        cols = line.split("\t")
        if len(cols) != len(INPUT_FIELDS) or cols[0] != expected_id:
            raise ValueError("grass ray inputs reordered or malformed")
        row = dict(zip(INPUT_FIELDS, cols, strict=True))
        if any(not math.isfinite(float(row[k])) for k in INPUT_FIELDS[1:]):
            raise ValueError("grass ray endpoint is not finite")
        rows.append(row)
    return rows


def capture(option: str) -> tuple[bytes, dict]:
    jar = (ROOT / "../.local/minecraft/client-b1.7.3.jar").resolve(strict=True)
    if sha(jar.read_bytes()) != JAR_SHA:
        raise ValueError("pinned Beta JAR hash mismatch")
    source_sha = sha(SOURCE.read_bytes())
    with tempfile.TemporaryDirectory(prefix="grass-rays-") as temp:
        classes = Path(temp) / "classes"
        classes.mkdir()
        subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(SOURCE)],
                       check=True, capture_output=True, timeout=40)
        proc = subprocess.run(["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", f"{classes}:{jar}",
                               "GrassRayOracle", str(jar), option], check=True,
                              capture_output=True, timeout=60)
    (parse_input if option == "--inputs" else parse_output)(proc.stdout)
    audit = json.loads(proc.stderr.decode("utf-8"))
    if (audit.get("jar_sha256") != JAR_SHA or audit.get("classes") != CLASS_SHA
            or audit.get("native_cases") != len(IDS) or audit.get("captures") != 2
            or audit.get("repeat_identical") is not True or audit.get("world_constructors") != 0
            or audit.get("save_access") is not False
            or audit.get("ray_method") != "fd.a(bt,bt,boolean,boolean):vf"
            or audit.get("shape_dispatch") != "uu.a(fd,int,int,int,bt,bt):vf"
            or audit.get("shape_handler") != "ru extends wb; ru.<init>(II) with block id 31 calls a(0.5f-0.4f,0,0.5f-0.4f,0.5f+0.4f,0.8f,0.5f+0.4f)"):
        raise ValueError("grass ray native provenance or isolation audit mismatch")
    audit["oracle_source_sha256"] = source_sha
    return proc.stdout, audit


def verify(write: bool = False) -> dict:
    first, audit = capture("--rays")
    second, repeat_audit = capture("--rays")
    input_first, input_audit = capture("--inputs")
    input_second, repeat_input_audit = capture("--inputs")
    if first != second or audit != repeat_audit or input_first != input_second or input_audit != repeat_input_audit:
        raise AssertionError("independent native grass-ray captures differ")
    audit["output_sha256"] = sha(first)
    audit["input_sha256"] = sha(input_first)
    audit["independent_repeat_identical"] = True
    audit["limitations"] = "The rays establish native selection bounds for ID 31/meta 0; no interaction result is inferred."
    if write:
        FIXTURE.parent.mkdir(parents=True, exist_ok=True)
        FIXTURE.write_bytes(first)
        INPUT.write_bytes(input_first)
        AUDIT.write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n")
    elif not FIXTURE.exists() or FIXTURE.read_bytes() != first:
        raise AssertionError("native grass-ray capture differs from fixture")
    elif not INPUT.exists() or INPUT.read_bytes() != input_first:
        raise AssertionError("native grass-ray input differs from fixture")
    elif not AUDIT.exists() or json.loads(AUDIT.read_text()) != audit:
        raise AssertionError("native grass-ray provenance differs from audit")
    return {"cases": len(parse_output(first)), "output_sha256": sha(first),
            "input_sha256": sha(input_first), "repeat_identical": True, "jar_sha256": JAR_SHA}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    print(json.dumps(verify(parser.parse_args().write), indent=2))
