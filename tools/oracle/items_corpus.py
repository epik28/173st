#!/usr/bin/env python3
"""Repeat the bounded native ItemStack/inventory capture from the pinned client JAR."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/items"
SOURCE = ROOT / "tools/oracle/src/ItemsOracle.java"
PINNED_JAR = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def jar_path() -> Path:
    config = tomllib.loads((ROOT / "config/local.toml").read_text())
    return (ROOT / config["paths"]["client_jar"]).resolve(strict=True)


def capture(jar: Path, out: Path) -> tuple[bytes, dict]:
    out.mkdir(parents=True, exist_ok=True)
    classes = out / "classes"
    classes.mkdir(exist_ok=True)
    if sha(jar.read_bytes()) != PINNED_JAR:
        raise ValueError("configured client JAR does not match the pinned Beta 1.7.3 client")
    subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(SOURCE)], check=True)
    proc = subprocess.run(
        ["java", "-cp", f"{classes}:{jar}", "ItemsOracle", str(jar)],
        input=(FIXTURES / "input.tsv").read_bytes(), stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=True, timeout=30,
    )
    audit = json.loads(proc.stderr)
    if audit["jar_sha256"] != PINNED_JAR or Path(audit["code_source"]).resolve() != jar.resolve():
        raise ValueError("native classes did not load from the pinned JAR")
    return proc.stdout, audit


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="replace expected output with two identical native captures")
    parser.add_argument("--out", type=Path, default=ROOT / "artifacts/oracle-items-check")
    args = parser.parse_args()
    jar = jar_path()
    first, audit1 = capture(jar, args.out / "first")
    second, audit2 = capture(jar, args.out / "repeat")
    if first != second or audit1 != audit2:
        raise SystemExit("native repeat capture or audit differed")
    if args.write:
        (FIXTURES / "native.tsv").write_bytes(first)
        audit = dict(audit1)
        audit.update(input_sha256=sha((FIXTURES / "input.tsv").read_bytes()),
                     output_sha256=sha(first), repeat_output_sha256=sha(second))
        (FIXTURES / "audit.json").write_text(json.dumps(audit, indent=2, sort_keys=True) + "\n")
    else:
        expected = (FIXTURES / "native.tsv").read_bytes()
        pinned_audit = json.loads((FIXTURES / "audit.json").read_text())
        if first != expected:
            raise SystemExit(f"native TSV differs: actual sha256={sha(first)} expected={sha(expected)}")
        if any(audit1.get(k) != pinned_audit.get(k) for k in ("jar_sha256", "classes", "native_methods", "bootstrap", "bootstrap_stdout_sha256", "direct_calls", "rows")):
            raise SystemExit("native audit differs from committed audit")
        if pinned_audit["input_sha256"] != sha((FIXTURES / "input.tsv").read_bytes()) or pinned_audit["output_sha256"] != sha(first):
            raise SystemExit("fixture input/output hash does not match audit")
    print(f"PASS native item capture: {audit1['rows']} ordered rows; repeat identical; output sha256={sha(first)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"items oracle error: {exc}", file=sys.stderr)
        raise SystemExit(2)
