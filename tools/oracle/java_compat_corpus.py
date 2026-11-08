"""Build a deterministic corpus from direct Java casts and String.hashCode."""
import argparse
import hashlib
import json
import random
import struct
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ORACLE = Path("tools/oracle/src/JavaCompatOracle.java")
INPUT = Path("tests/fixtures/java/input.tsv")
REFERENCE = Path("tests/fixtures/java/reference.tsv")
AUDIT = Path("tests/fixtures/java/audit.json")
JVM_CASES = 4096
STRING_CASES = 2048
SEED = 0x173B17A3


def sha(data):
    return hashlib.sha256(data).hexdigest()


def f32(value):
    return struct.unpack(">I", struct.pack(">f", value))[0]


def f64(value):
    return struct.unpack(">Q", struct.pack(">d", value))[0]


def build_cases():
    cases = []
    for op, width, edges in (
        ("f32_to_i32", 8, (0, 0x80000000, 0x7F800000, 0xFF800000, 0x7FC00001,
                            0x3F800000, 0xBF800000, 0x4EFFFFFF, 0x4F000000,
                            0xCF000000, 0xCF000001, 0x00000001, 0x80000001)),
        ("f64_to_i32", 16, (0, 0x8000000000000000, 0x7FF0000000000000,
                             0xFFF0000000000000, 0x7FF8000000000001,
                             f64(1.0), f64(-1.0), f64(2147483647.0),
                             f64(2147483648.0), f64(-2147483648.0),
                             f64(-2147483649.0), 1, 0x8000000000000001)),
        ("f32_to_i64", 8, (0, 0x80000000, 0x7F800000, 0xFF800000, 0x7FC00001,
                            f32(1.0), f32(-1.0), 0x5EFFFFFF, 0x5F000000,
                            0xDF000000, 0xDF000001, 1, 0x80000001)),
        ("f64_to_i64", 16, (0, 0x8000000000000000, 0x7FF0000000000000,
                             0xFFF0000000000000, 0x7FF8000000000001,
                             f64(1.0), f64(-1.0), 0x43DFFFFFFFFFFFFF,
                             0x43E0000000000000, 0xC3E0000000000000,
                             0xC3E0000000000001, 1, 0x8000000000000001)),
    ):
        rng = random.Random(SEED ^ sum(op.encode("ascii")))
        values = list(edges) + [rng.getrandbits(width * 4) for _ in range(JVM_CASES)]
        for index, value in enumerate(values):
            cases.append((f"{op}-{index}", op, f"{value:0{width}x}"))

    strings = ["", "ascii", "雪", "é", "e\u0301", "😀", "A😀Z", "\u0000", "A" * 10000]
    rng = random.Random(SEED ^ 0x48415348)
    while len(strings) < STRING_CASES + 9:
        length = rng.randrange(0, 129)
        scalars = []
        for _ in range(length):
            point = rng.randrange(0x110000)
            if 0xD800 <= point <= 0xDFFF:
                point = 0xFFFD
            scalars.append(chr(point))
        strings.append("".join(scalars))
    for index, value in enumerate(strings):
        cases.append((f"string-{index}", "string_hash_code", value.encode("utf-8").hex()))
    return cases


def input_bytes(cases):
    return ("".join("\t".join(row) + "\n" for row in cases)).encode("ascii")


def run_oracle(data):
    with tempfile.TemporaryDirectory(prefix="java-compat-") as tmp:
        classes = Path(tmp)
        build = subprocess.run(["javac", "-encoding", "UTF-8", "-d", str(classes), str(ROOT / ORACLE)],
                               capture_output=True, timeout=30, check=False, cwd=ROOT)
        if build.returncode:
            raise RuntimeError("javac failed: " + build.stderr.decode("utf-8", "replace"))
        result = subprocess.run(["java", "-cp", str(classes), "JavaCompatOracle"], input=data,
                                capture_output=True, timeout=60, check=False, cwd=ROOT)
        if result.returncode:
            raise RuntimeError("Java oracle failed: " + result.stderr.decode("utf-8", "replace"))
        version = subprocess.run(["java", "-version"], capture_output=True, timeout=10,
                                 check=False, cwd=ROOT)
        return result.stdout, (version.stdout + version.stderr).decode("utf-8", "replace").strip()


def verify_or_write(write=False):
    cases = build_cases()
    data = input_bytes(cases)
    first, version = run_oracle(data)
    second, version_again = run_oracle(data)
    if first != second or version != version_again:
        raise RuntimeError("Java oracle output or runtime version is not repeatable")
    if not first.endswith(b"\n") or len(first.splitlines()) != len(cases):
        raise RuntimeError("Java oracle returned malformed row count/framing")
    audit = {
        "schema_version": 1, "suite": "java", "case_count": len(cases),
        "seed": SEED, "random_conversion_cases_per_operation": JVM_CASES,
        "string_cases": len(cases) - 4 * (JVM_CASES + 13),
        "operations": {op: sum(row[1] == op for row in cases)
                       for op in sorted({row[1] for row in cases})},
        "oracle": str(ORACLE), "oracle_sha256": sha((ROOT / ORACLE).read_bytes()),
        "java_version": version, "input_sha256": sha(data),
        "reference_sha256": sha(first), "repeat_reference_sha256": sha(second),
    }
    if write:
        for path, payload in ((INPUT, data), (REFERENCE, first)):
            (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
            (ROOT / path).write_bytes(payload)
        (ROOT / AUDIT).write_text(json.dumps(audit, ensure_ascii=False, indent=2) + "\n")
    else:
        for path, expected in ((INPUT, data), (REFERENCE, first)):
            if (ROOT / path).read_bytes() != expected:
                raise RuntimeError(f"fixture mismatch: {path}")
        if json.loads((ROOT / AUDIT).read_text()) != audit:
            raise RuntimeError("fixture audit mismatch")
    return audit


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        audit = verify_or_write(args.write)
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"ERROR: {error}") from error
    print(json.dumps({"status": "PASS", **audit}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
