#!/usr/bin/env python3
"""Run and verify the pinned native inventory interaction oracle twice."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/inventory_interaction/audit.json"
ORACLE = ROOT / "tools/oracle/src/InventoryInteractionOracle.java"
PINNED_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
PINNED_SIZE = 1465375


def digest(data):
    return hashlib.sha256(data).hexdigest()


def load_fixture(path=FIXTURE):
    value = json.loads(Path(path).read_text(encoding="utf-8"))
    if value.get("schema_version") != 1:
        raise ValueError("unsupported interaction fixture schema")
    reference = value.get("reference", {})
    if reference.get("jar_sha256") != PINNED_SHA or reference.get("jar_size") != PINNED_SIZE:
        raise ValueError("fixture does not pin the original client JAR")
    protocol = value.get("protocol", {})
    if protocol.get("expected_rows") != 15 or protocol.get("deterministic_repeats") != 2:
        raise ValueError("interaction protocol bounds were altered")
    if protocol.get("scenarios") != ["left_pick_place", "right_split", "merge_same", "swap_different", "close_return"]:
        raise ValueError("interaction scenario set was altered")
    cursor = value.get("cursor_rendering", {})
    if cursor.get("position") != "screen mouse coordinates minus inventory origin minus (8,8)" or cursor.get("icon_size") != [16, 16]:
        raise ValueError("cursor rendering geometry was altered")
    if cursor.get("draw_order") != ["slot contents", "cursor icon", "cursor count", "tooltip"]:
        raise ValueError("cursor rendering order was altered")
    return value


def verify(jar_path, fixture_path=FIXTURE):
    fixture = load_fixture(fixture_path)
    jar = Path(jar_path)
    data = jar.read_bytes()
    if len(data) != PINNED_SIZE or digest(data) != PINNED_SHA:
        raise ValueError("client JAR hash/size differs from pinned Beta 1.7.3")
    javac = shutil.which("javac")
    java = shutil.which("java")
    javap = shutil.which("javap")
    if not javac or not java or not javap:
        raise ValueError("javac, java and javap are required")
    with zipfile.ZipFile(jar) as archive:
        for cls, expected_hash in fixture["reference"]["native_classes"].items():
            if digest(archive.read(cls + ".class")) != expected_hash:
                raise ValueError("missing native class " + cls)
    with tempfile.TemporaryDirectory(prefix="inventory-interaction-audit-") as directory:
        classes = Path(directory) / "classes"
        classes.mkdir()
        built = subprocess.run([javac, "-cp", str(jar), "-d", str(classes), str(ORACLE)],
                               capture_output=True, text=True, timeout=30)
        if built.returncode:
            raise ValueError("oracle compilation failed: " + built.stderr.strip())
        for cls, name, descriptor in (fixture["reference"]["native_methods"]["click"],
                                      fixture["reference"]["native_methods"]["shift_transfer"],
                                      fixture["reference"]["native_methods"]["cursor_get"],
                                      fixture["reference"]["native_methods"]["cursor_set"],
                                      fixture["cursor_rendering"]["icon_method"],
                                      fixture["cursor_rendering"]["count_method"]):
            disassembly = subprocess.run([javap, "-classpath", str(jar), "-p", "-s", cls],
                                         capture_output=True, text=True, timeout=30)
            if disassembly.returncode or f"descriptor: {descriptor}" not in disassembly.stdout:
                raise ValueError(f"native descriptor missing: {cls}.{name}{descriptor}")
        runs = []
        for _ in range(2):
            result = subprocess.run([java, "-cp", f"{classes}:{jar}", "InventoryInteractionOracle", str(jar)],
                                    capture_output=True, text=True, timeout=30)
            if result.returncode:
                raise ValueError("native oracle failed: " + result.stderr.strip())
            runs.append(result.stdout)
        if runs[0] != runs[1]:
            raise ValueError("native interaction replay is not byte-stable")
        output = runs[0].encode()
        expected = fixture["protocol"]["expected_output_sha256"]
        if digest(output) != expected:
            raise ValueError(f"native output hash differs: {digest(output)}")
        rows = runs[0].splitlines()
        if not rows or rows[0] != "jar_sha256\t" + PINNED_SHA:
            raise ValueError("native output omitted the pinned JAR hash")
        data_rows = rows[1:]
        if len(data_rows) != fixture["protocol"]["expected_rows"]:
            raise ValueError("unexpected interaction row count")
        for row in data_rows:
            fields = row.split("\t")
            if len(fields) != 5 or len(fields[4].removeprefix("slots=").split(",")) != 36:
                raise ValueError("interaction row does not contain cursor and 36 slots")
    return fixture


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("jar", type=Path)
    args = parser.parse_args()
    try:
        verify(args.jar)
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"ERROR: {error}")
        return 1
    print("PASS: native inventory click corpus replayed twice with 36 slots and cursor")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
