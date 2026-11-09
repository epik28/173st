#!/usr/bin/env python3
"""Capture and verify the original Beta 1.7.3 block opacity/emission registry."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCE = ROOT / "tools/oracle/src/LightMaterialOracle.java"
FIXTURE = ROOT / "tests/fixtures/light_material"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
REQUEST = b"registry\tall\n"
BOOTSTRAP_SHA = "0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e"
CLASS_NAMES = ("uu",)
MANIFEST_FIELDS = (
    "jar_sha256", "source_sha256", "input_sha256", "output_sha256",
    "capture_output_sha256", "class_sha256", "bootstrap_sha256", "registry_schema",
)
REGISTRY_SCHEMA = {
    "columns": ["id", "registered", "opacity_raw", "emission_raw"],
    "ids": {"first": 0, "last": 255, "ordered": True, "unique": True},
    "native_fields": {"registered": "uu.m[id] != null", "opacity_raw": "uu.q[id]", "emission_raw": "uu.s[id]"},
    "unregistered_entries": "uu.m[id] is null; raw array zeros are retained as unregistered defaults, not material properties",
    "opacity_range": [0, 255],
    "emission_range": [0, 15],
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_registry(output: bytes) -> list[tuple[int, int, int, int]]:
    if not output.endswith(b"\n") or b"\r" in output:
        raise ValueError("registry output must use LF line endings and end with LF")
    try:
        lines = output.decode("ascii").splitlines()
    except UnicodeDecodeError as exc:
        raise ValueError("registry output must be ASCII") from exc
    if len(lines) != 256:
        raise ValueError(f"registry must contain exactly 256 rows, got {len(lines)}")
    rows: list[tuple[int, int, int, int]] = []
    for expected_id, line in enumerate(lines):
        fields = line.split("\t")
        if len(fields) != 4:
            raise ValueError(f"registry row {expected_id} must have four tab-separated fields")
        if any(not field.isdecimal() or (len(field) > 1 and field.startswith("0")) for field in fields):
            raise ValueError(f"registry row {expected_id} must use canonical decimal fields")
        try:
            row = tuple(int(field, 10) for field in fields)
        except ValueError as exc:
            raise ValueError(f"registry row {expected_id} contains a non-decimal field") from exc
        block_id, registered, opacity, emission = row
        if block_id != expected_id:
            raise ValueError(f"registry IDs must be ordered exactly once; expected {expected_id}, got {block_id}")
        if registered not in (0, 1):
            raise ValueError(f"invalid registration flag at ID {block_id}")
        if not 0 <= opacity <= 255 or not 0 <= emission <= 15:
            raise ValueError(f"native material property out of range at ID {block_id}")
        if registered == 0 and (opacity != 0 or emission != 0):
            raise ValueError(f"unregistered ID {block_id} must retain the native arrays' zero defaults")
        rows.append(row)
    return rows


def pinned_jar(path: pathlib.Path) -> pathlib.Path:
    path = path.resolve()
    if not path.is_file() or sha(path.read_bytes()) != JAR_SHA:
        raise ValueError("wrong or missing pinned client JAR")
    return path


def class_hashes(jar: pathlib.Path) -> dict[str, str]:
    try:
        with zipfile.ZipFile(jar) as archive:
            result = {name: sha(archive.read(f"{name}.class")) for name in CLASS_NAMES}
    except (KeyError, zipfile.BadZipFile) as exc:
        raise ValueError(f"cannot verify pinned JAR class entries: {exc}") from exc
    return result


def capture(jar: pathlib.Path, build: pathlib.Path) -> bytes:
    javac, java = shutil.which("javac"), shutil.which("java")
    if not javac or not java:
        raise ValueError("javac and java are required for the native material audit")
    classes = build / "classes"
    classes.mkdir(parents=True)
    compiled = subprocess.run(
        [javac, "-encoding", "UTF-8", "-cp", str(jar), "-d", str(classes), str(SOURCE)],
        capture_output=True, timeout=40, check=False,
    )
    if compiled.returncode:
        raise ValueError("javac failed: " + compiled.stderr.decode("utf-8", "replace")[-3000:])
    result = subprocess.run(
        [java, "-cp", f"{classes}{os.pathsep}{jar}", "LightMaterialOracle", str(jar)],
        input=REQUEST, capture_output=True, timeout=40, check=False,
    )
    if result.returncode:
        raise ValueError("native oracle failed: " + result.stderr.decode("utf-8", "replace")[-3000:])
    if result.stderr:
        raise ValueError("native oracle wrote unexpected stderr: " + result.stderr.decode("utf-8", "replace")[-1000:])
    parse_registry(result.stdout)
    return result.stdout


def validate_evidence(
    captures: list[bytes], expected_output: bytes, expected_manifest: dict,
    source_bytes: bytes, jar: pathlib.Path, request: bytes = REQUEST,
) -> None:
    if len(captures) != 2:
        raise ValueError("exactly two independent native captures are required")
    if captures[0] != captures[1]:
        raise ValueError("independent native material captures differ")
    if captures[0] != expected_output:
        raise ValueError("native material capture differs from committed native.tsv")
    parse_registry(captures[0])
    if request != REQUEST:
        raise ValueError("input protocol must be the fixed registry request")
    missing = [field for field in MANIFEST_FIELDS if field not in expected_manifest]
    if missing:
        raise ValueError("committed audit manifest is missing fields: " + ", ".join(missing))
    actual = {
        "jar_sha256": sha(jar.read_bytes()),
        "source_sha256": sha(source_bytes),
        "input_sha256": sha(request),
        "output_sha256": sha(expected_output),
        "capture_output_sha256": [sha(captures[0]), sha(captures[1])],
        "class_sha256": class_hashes(jar),
        "bootstrap_sha256": BOOTSTRAP_SHA,
        "registry_schema": REGISTRY_SCHEMA,
    }
    for field in MANIFEST_FIELDS:
        if expected_manifest[field] != actual[field]:
            raise ValueError(f"audit manifest {field} mismatch")


def run(jar_path: pathlib.Path, write: bool) -> None:
    jar = pinned_jar(jar_path)
    source_bytes = SOURCE.read_bytes()
    with tempfile.TemporaryDirectory(prefix="light-material-oracle-") as temporary:
        base = pathlib.Path(temporary)
        captures = [capture(jar, base / f"run-{index}") for index in (1, 2)]
    if captures[0] != captures[1]:
        raise ValueError("two fresh native captures differ")
    output = captures[0]
    parse_registry(output)
    manifest = {
        "jar_sha256": sha(jar.read_bytes()),
        "source_sha256": sha(source_bytes),
        "input_sha256": sha(REQUEST),
        "output_sha256": sha(output),
        "capture_output_sha256": [sha(captures[0]), sha(captures[1])],
        "class_sha256": class_hashes(jar),
        "bootstrap_sha256": BOOTSTRAP_SHA,
        "registry_schema": REGISTRY_SCHEMA,
    }
    if write:
        FIXTURE.mkdir(parents=True, exist_ok=True)
        (FIXTURE / "native.tsv").write_bytes(output)
        (FIXTURE / "audit.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    else:
        expected_output = (FIXTURE / "native.tsv").read_bytes()
        expected_manifest = json.loads((FIXTURE / "audit.json").read_text())
        validate_evidence(captures, expected_output, expected_manifest, source_bytes, jar)
    print(f"PASS: 256 native block IDs; two identical captures; output SHA-256 {sha(output)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=pathlib.Path, default=ROOT.parent / ".local/minecraft/client-b1.7.3.jar")
    parser.add_argument("--write", action="store_true", help="replace the committed native capture after two fresh identical runs")
    args = parser.parse_args()
    try:
        run(args.jar, args.write)
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
