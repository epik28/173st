#!/usr/bin/env python3
"""Exercise the pinned native Beta 1.7.3 save metadata reader on synthetic saves."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools.oracle.menu_parity_audit import default_jar
from tools.oracle.native_menu_capture import create_synthetic_saves


FIXTURE = ROOT / "tests/fixtures/menu_parity/synthetic_saves.json"
EXPECTED = ROOT / "tests/fixtures/menu_parity/synthetic_save_list.tsv"
ORACLE = ROOT / "tools/oracle/src/MenuSaveListOracle.java"
EXPECTED_CLASS_HASHES = {
    "hi": "f4a89d4268ba827357b151cc73c404be042d76132090224a3834a4ef2b5f43cf",
    "tq": "339175cb6bdd4f97024f30c9c501b4efe9221f60672a9fb3b017890b5b2272c8",
    "vb": "ff8c4833109f9bd4439ebc4ddb718ad303fec9074a2f66a744f06de754d169a9",
    "ei": "5d01d1dda7be346c57fd00846d2425615186c3ef1c65c06aa0bd0f45aa9f7fef",
    "as": "c840daad77df4a1ad92367abce5b3d2035362bc2b010df1ff7b4faee3bbfb652",
    "nu": "a1a5701ec2180d01ddc604cc8c2e5aff1176b10faf83bdb64d2f1cc8c806ac4a",
}
EXPECTED_JAR = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
EXPECTED_FIXTURE_SHA256 = "09a415e91dba34a43ab385fc4e9eeead0a7215ef7fbe75728b1d971ef4e21496"
EXPECTED_OUTPUT_SHA256 = "766def05c9bbb201f8cf5eff7bbceaa141a0e87b9a60706217b9d0f8760d03fc"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def load_fixture(path=FIXTURE):
    try:
        data = Path(path).read_bytes()
        fixture = json.loads(data.decode("utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read synthetic save fixture: {error}") from error
    if fixture.get("schema_version") != 1:
        raise ValueError("unsupported synthetic save fixture schema")
    if sha256(data) != EXPECTED_FIXTURE_SHA256:
        raise ValueError("synthetic save fixture hash differs from the reviewed contract")
    if fixture.get("reference") != {"client_jar_sha256": EXPECTED_JAR, "classes": EXPECTED_CLASS_HASHES}:
        raise ValueError("synthetic save native class provenance differs from the pinned client")
    contract = fixture.get("native_contract", {})
    if contract != {
        "storage_type": "Scaevolus' McRegion",
        "metadata_lookup": "level.dat, then level.dat_old if level.dat parse fails",
        "row_filter": "directory is listed only when one metadata file parses to a Data compound",
        "conversion_warning": "level version != 19132",
        "sort": "descending LastPlayed, then directory name ascending",
        "does_not_create_or_load_world": True,
    }:
        raise ValueError("native save-list semantics differ from the reviewed contract")
    return fixture


def verify(jar_path=None, fixture_path=FIXTURE, expected_path=EXPECTED, work_dir=None):
    fixture = load_fixture(fixture_path)
    jar = Path(jar_path or default_jar()).resolve()
    if sha256(jar.read_bytes()) != EXPECTED_JAR:
        raise ValueError("client JAR differs from the pinned Beta 1.7.3 artifact")
    try:
        with zipfile.ZipFile(jar) as archive:
            for name, wanted in EXPECTED_CLASS_HASHES.items():
                if sha256(archive.read(name + ".class")) != wanted:
                    raise ValueError(f"native save-list class hash differs: {name}")
    except (KeyError, zipfile.BadZipFile) as error:
        raise ValueError(f"pinned client archive lacks native save-list classes: {error}") from error

    javac, java = shutil.which("javac"), shutil.which("java")
    if not javac or not java:
        raise ValueError("javac and java are required for the native metadata audit")
    temporary = tempfile.TemporaryDirectory(prefix="menu-save-list-audit-") if work_dir is None else None
    work = Path(temporary.name if temporary else work_dir)
    classes = work / "classes"
    saves = work / "saves"
    classes.mkdir(parents=True, exist_ok=True)
    saves.mkdir(parents=True, exist_ok=True)
    create_synthetic_saves(saves)
    try:
        compile_result = subprocess.run([javac, "-cp", str(jar), "-d", str(classes), str(ORACLE)],
                                        capture_output=True, text=True, timeout=30)
        if compile_result.returncode:
            raise ValueError(f"native save oracle compilation failed: {compile_result.stderr.strip()}")
        result = subprocess.run([java, "-cp", str(classes) + os.pathsep + str(jar),
                                 "MenuSaveListOracle", str(saves)],
                                capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise ValueError(f"native save oracle failed: {result.stderr.strip()}")
        if result.stderr.count("java.util.zip.ZipException: Not in GZIP format") != 2:
            raise ValueError("native corrupt/fallback metadata diagnostics differ from the two synthetic cases")
        expected_bytes = Path(expected_path).read_bytes()
        if sha256(expected_bytes) != EXPECTED_OUTPUT_SHA256:
            raise ValueError("saved native save-list reference hash differs")
        expected = expected_bytes.decode("utf-8")
        if result.stdout != expected:
            raise ValueError("native save-list output differs from the frozen synthetic reference")
    finally:
        if temporary is not None:
            temporary.cleanup()
    return fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, default=default_jar())
    args = parser.parse_args()
    try:
        verify(args.jar)
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("PASS: native metadata list rows, LastPlayed sorting, conversion flag, level.dat_old fallback, invalid metadata filtering")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
