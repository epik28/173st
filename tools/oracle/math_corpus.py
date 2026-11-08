#!/usr/bin/env python3
"""Generate and verify the Beta 1.7.3 native MathHelper behavior corpus."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/math"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASS_SHA = ""
MULTIPLIER = struct.unpack(">f", struct.pack(">f", 10430.378))[0]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def f32(value):
    return struct.unpack(">f", struct.pack(">f", value))[0]


def f32_bits(value):
    return struct.pack(">f", value).hex()


def f64_bits(value):
    return struct.pack(">d", value).hex()


def next_f32(value, direction):
    bits = struct.unpack(">I", struct.pack(">f", value))[0]
    if value == 0.0:
        bits = 1 if direction > 0 else 0x80000001
    elif (value > 0) == (direction > 0):
        bits += 1
    else:
        bits -= 1
    return struct.unpack(">f", struct.pack(">I", bits))[0]


def trig_angles():
    """Find one finite f32 argument mapping to each original LUT index."""
    for index in range(65536):
        base = f32(index / MULTIPLIER)
        candidates = [base]
        lo = hi = base
        for _ in range(16):
            lo, hi = next_f32(lo, -1), next_f32(hi, 1)
            candidates.extend((lo, hi))
        angle = next((x for x in candidates if int(f32(x * MULTIPLIER)) & 0xffff == index), None)
        if angle is None:
            raise RuntimeError(f"cannot address trig LUT index {index}")
        yield index, angle


def corpus():
    rows, coverage = [], set()

    def add(case, op, *args):
        rows.append("\t".join((case, op, *args)))

    for index, angle in trig_angles():
        coverage.add(index)
        bits = f32_bits(angle)
        add(f"sin-lut-{index:05d}", "sin", bits)
        add(f"cos-lut-{index:05d}", "cos", bits)

    f32_values = [0, 0x80000000, 0x3f800000, 0xbf800000, 0x00000001,
                  0x80000001, 0x007fffff, 0x00800000, 0x7f7fffff,
                  0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00001,
                  0xffc00001, 0x40490fdb, 0xc0490fdb]
    f64_values = [0, 0x8000000000000000, 0x3ff0000000000000,
                  0xbff0000000000000, 1, 0x8000000000000001,
                  0x0010000000000000, 0x7fefffffffffffff,
                  0xffefffffffffffff, 0x7ff0000000000000,
                  0xfff0000000000000, 0x7ff8000000000001,
                  0xfff8000000000001]
    for i, b in enumerate(f32_values):
        raw = f"{b:08x}"
        for op in ("sqrt_f32", "floor_f32", "abs_f32"):
            add(f"{op}-edge-{i}", op, raw)
    for i, b in enumerate(f64_values):
        raw = f"{b:016x}"
        add(f"sqrt-f64-edge-{i}", "sqrt_f64_to_f32", raw)
        add(f"floor-f64-edge-{i}", "floor_f64", raw)
        for j, c in enumerate(f64_values):
            if j < 4 or i < 4:
                add(f"absmax-{i}-{j}", "abs_max_f64", raw, f"{c:016x}")
    i32 = [-2147483648, -2147483647, -1000001, -3, -2, -1, 0, 1, 2, 3,
           1000001, 2147483646, 2147483647]
    for i, n in enumerate(i32):
        for j, d in enumerate(i32):
            add(f"floor-div-{i}-{j}", "floor_div_i32", str(n), str(d))
    for name, raw in (("null", "null"), ("empty", ""), ("space", "20"),
                      ("unicode", "e29883"), ("text", "6265746120312e372e33")):
        add(f"null-empty-{name}", "null_or_empty", raw)
    return rows, coverage


def paths():
    config = tomllib.loads((ROOT / "config/local.toml").read_text())["paths"]
    jar = (ROOT / config["client_jar"]).resolve(strict=True)
    return jar


def run_oracle(rows, jar):
    raw_jar = jar.read_bytes()
    if sha(raw_jar) != JAR_SHA:
        raise RuntimeError("unexpected original client JAR SHA-256")
    with zipfile.ZipFile(jar) as archive:
        class_bytes = archive.read("in.class")
    with tempfile.TemporaryDirectory(prefix="math-oracle-") as tmp:
        classes = Path(tmp) / "classes"
        classes.mkdir()
        subprocess.run(["javac", "-cp", str(jar), "-d", str(classes),
                        str(ROOT / "tools/oracle/src/MathOracle.java")], check=True)
        result = subprocess.run(["java", "-cp", f"{classes}:{jar}", "MathOracle", str(jar)],
                                input=("\n".join(rows) + "\n").encode(),
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
                                timeout=60)
        if result.returncode:
            raise RuntimeError(result.stderr.decode(errors="replace"))
    return result.stdout, class_bytes


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    rows, coverage = corpus()
    input_bytes = ("\n".join(rows) + "\n").encode()
    jar = paths()
    output, class_bytes = run_oracle(rows, jar)
    repeat_output, repeat_class = run_oracle(rows, jar)
    if output != repeat_output or class_bytes != repeat_class:
        raise RuntimeError("native MathHelper oracle was not repeatable")
    actual_rows = output.decode("ascii").splitlines()
    if len(actual_rows) != len(rows) or any(len(x.split("\t")) != 2 for x in actual_rows):
        raise RuntimeError("original oracle output shape mismatch")
    audit = {
        "kind": "native Beta 1.7.3 MathHelper oracle",
        "jar_sha256": JAR_SHA,
        "class_name": "in",
        "class_sha256": sha(class_bytes),
        "operations": len(rows),
        "operations_by_method": {},
        "trig_lut_indices_per_method": {"sin": len(coverage), "cos": len(coverage)},
        "input_sha256": sha(input_bytes),
        "output_sha256": sha(output),
        "repeat_output_sha256": sha(repeat_output),
        "repeatable": True,
    }
    for row in rows:
        op = row.split("\t", 2)[1]
        audit["operations_by_method"][op] = audit["operations_by_method"].get(op, 0) + 1
    if args.write:
        FIXTURE.mkdir(parents=True, exist_ok=True)
        (FIXTURE / "input.tsv").write_bytes(input_bytes)
        (FIXTURE / "reference.tsv").write_bytes(output)
        (FIXTURE / "audit.json").write_text(json.dumps(audit, indent=2) + "\n")
    else:
        for name, expected in (("input.tsv", input_bytes), ("reference.tsv", output)):
            if (FIXTURE / name).read_bytes() != expected:
                raise RuntimeError(f"committed {name} differs from live original oracle")
        if json.loads((FIXTURE / "audit.json").read_text()) != audit:
            raise RuntimeError("committed audit differs from live original oracle")
    print(json.dumps({"status": "PASS", "operations": len(rows),
                      "sin_cos_lut_coverage": len(coverage), "class_sha256": sha(class_bytes),
                      "input_sha256": sha(input_bytes), "output_sha256": sha(output)}, indent=2))


if __name__ == "__main__":
    main()
