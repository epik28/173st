#!/usr/bin/env python3
"""Check the actual Java adapter, independently of the future Rust probe."""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BUILD = ROOT / "artifacts/oracle-check"


def main():
    BUILD.mkdir(parents=True, exist_ok=True)
    subprocess.run(["javac", "-d", str(BUILD),
                    str(ROOT / "tools/oracle/src/RandomOracle.java")], check=True, timeout=30)
    command = ["java", "-cp", str(BUILD), "RandomOracle"]

    def run(data):
        return subprocess.run(command, input=data, capture_output=True, timeout=10)

    fixture = (ROOT / "tests/fixtures/rng/next-int.tsv").read_bytes()
    first, second = run(fixture), run(fixture)
    assert first.returncode == second.returncode == 0
    assert first.stdout == second.stdout and not first.stderr and not second.stderr
    assert len(first.stdout.splitlines()) == 96
    (BUILD / "fixture.stdout.tsv").write_bytes(first.stdout)
    invalid = [
        "x\t9223372036854775808\t1\n", "x\t-9223372036854775809\t1\n",
        "x\tNaN\t1\n", "x\t0\t0\n", "x\t0\t-1\n",
        "x\t0\t100001\n", "x\t0\t1.5\n", "x\t0\t999999999999\n",
        "\t0\t1\n", "bad id\t0\t1\n", "é\t0\t1\n",
        "x\t0\t1\nx\t1\t1\n", "x\t0\n", "x\t0\t1\textra\n",
    ]
    for data in invalid:
        result = run(data.encode())
        assert result.returncode != 0 and result.stderr, repr(data)
    maximum = run(b"# comment\n\nx\t0\t100000\n")
    assert maximum.returncode == 0 and len(maximum.stdout.splitlines()) == 100000
    print("PASS: javac; deterministic 96 records including i64 boundaries; "
          f"{len(invalid)} invalid inputs rejected; count=100000 accepted")


if __name__ == "__main__":
    main()
