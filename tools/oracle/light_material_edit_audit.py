#!/usr/bin/env python3
"""Rebuild and verify synthetic native canopy/log and water edit captures."""
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
FIXTURE = ROOT / "tests/fixtures/light_material"
JAVA_SOURCES = {
    "material_edit": ROOT / "tools/oracle/src/LightMaterialEditOracle.java",
    "light_update_helper": ROOT / "tools/oracle/src/LightUpdateOracle.java",
}
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
REQUEST = b"material-edits-v1\n"
BOOTSTRAP_SHA = "0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e"
CLASS_NAMES = ("fd", "lm", "wi", "uu", "eb", "st")
CASE_NAMES = ("canopy-log-remove", "water-remove", "water-place")
MANIFEST_FIELDS = (
    "jar_sha256", "source_sha256", "input_sha256", "output_sha256",
    "capture_output_sha256", "class_sha256", "bootstrap_sha256", "capture_contract",
)
CAPTURE_CONTRACT = {
    "cases": list(CASE_NAMES),
    "seed": "synthetic blocks placed with native fd.a and light settled via native fd.j; no save files",
    "edit": "native fd.a(IIIII)Z followed by bounded native light-update queue drain",
    "samples": "ordered raw id, metadata, uu.q opacity, uu.s emission, sky, block, combined light rows before and after",
    "domain": {"loaded_chunks_x": [-1, 0, 1], "loaded_chunks_z": [-1, 0, 1], "y": [0, 127]},
}
EXPECTED_POINTS = {
    "canopy-log-remove": [(8, 65, 8), (8, 64, 8), (8, 66, 8), (7, 65, 8), (8, 65, 7), (8, 66, 7), (8, 67, 8)],
    "water-remove": [(8, 64, 8), (8, 65, 8), (7, 64, 8), (9, 64, 8), (8, 64, 7), (8, 64, 9)],
    "water-place": [(8, 64, 8), (8, 65, 8), (7, 64, 8), (9, 64, 8), (8, 64, 7), (8, 64, 9)],
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def pinned_jar(path: pathlib.Path) -> pathlib.Path:
    path = path.resolve()
    if not path.is_file() or sha(path.read_bytes()) != JAR_SHA:
        raise ValueError("wrong or missing pinned client JAR")
    return path


def class_hashes(jar: pathlib.Path) -> dict[str, str]:
    try:
        with zipfile.ZipFile(jar) as archive:
            return {name: sha(archive.read(f"{name}.class")) for name in CLASS_NAMES}
    except (KeyError, zipfile.BadZipFile) as exc:
        raise ValueError(f"cannot verify pinned native class entries: {exc}") from exc


def validate_output(output: bytes) -> None:
    if not output or len(output) > 2 * 1024 * 1024 or not output.endswith(b"\n") or b"\r" in output:
        raise ValueError("native edit output must be nonempty, bounded, LF-terminated ASCII")
    try:
        lines = output.decode("ascii").splitlines()
    except UnicodeDecodeError as exc:
        raise ValueError("native edit output must be ASCII") from exc
    cases: dict[str, dict] = {}
    for line_number, line in enumerate(lines, start=1):
        fields = line.split("\t")
        if fields[0] == "case" and len(fields) == 5:
            name = fields[1]
            if name not in CASE_NAMES or name in cases:
                raise ValueError(f"invalid or duplicate native edit case at row {line_number}")
            try:
                target = tuple(int(value) for value in fields[2].removeprefix("target=").split(","))
                replacement = int(fields[3].removeprefix("replacement="))
            except ValueError as exc:
                raise ValueError(f"invalid native edit case parameters at row {line_number}") from exc
            digest = fields[4].removeprefix("before_digest=")
            if len(target) != 3 or not fields[4].startswith("before_digest=") or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
                raise ValueError(f"invalid native pre-edit digest at row {line_number}")
            if not all(-16 <= value <= 31 for value in (target[0], target[2])) or not 0 <= target[1] <= 127:
                raise ValueError(f"native edit target outside loaded domain at row {line_number}")
            cases[name] = {"target": target, "replacement": replacement, "samples": {"before": [], "after": []}, "final": False}
        elif fields[0] == "final" and len(fields) == 3:
            if fields[1] not in cases or cases[fields[1]]["final"] or len(fields[2]) != 64 or any(c not in "0123456789abcdef" for c in fields[2]):
                raise ValueError(f"orphaned, duplicate, or malformed final digest at row {line_number}")
            cases[fields[1]]["final"] = True
        elif len(fields) == 12 and fields[1] in {"before", "after"}:
            name, phase = fields[:2]
            if name not in cases:
                raise ValueError(f"sample precedes case header at row {line_number}")
            try:
                x, y, z = map(int, fields[2:5])
                block_id, metadata, opacity, emission, sky, block, combined = map(int, fields[5:12])
            except ValueError as exc:
                raise ValueError(f"non-numeric native sample at row {line_number}") from exc
            if not (-16 <= x <= 31 and -16 <= z <= 31 and 0 <= y <= 127):
                raise ValueError(f"native sample outside loaded domain at row {line_number}")
            if not (0 <= block_id <= 255 and 0 <= metadata <= 15 and 0 <= opacity <= 255 and 0 <= emission <= 15 and all(0 <= light <= 15 for light in (sky, block, combined))):
                raise ValueError(f"native sample outside property range at row {line_number}")
            cases[name]["samples"][phase].append((x, y, z, block_id, metadata, opacity, emission, sky, block, combined))
        else:
            raise ValueError(f"invalid native edit output row {line_number}")

    if tuple(cases) != CASE_NAMES:
        raise ValueError("native edit output must cover the three declared scenarios in order")
    for name, case in cases.items():
        if not case["final"]:
            raise ValueError(f"missing final native state digest for {name}")
        expected = EXPECTED_POINTS[name]
        for phase in ("before", "after"):
            actual_points = [row[:3] for row in case["samples"][phase]]
            if actual_points != expected:
                raise ValueError(f"{name} {phase} samples differ from the fixed ordered protocol")
        target = case["target"]
        before_target = next(row for row in case["samples"]["before"] if row[:3] == target)
        after_target = next(row for row in case["samples"]["after"] if row[:3] == target)
        if before_target[3] == case["replacement"] or after_target[3] != case["replacement"]:
            raise ValueError(f"native edit transition missing at target for {name}")

    def sampled(name: str, phase: str, point: tuple[int, int, int]) -> tuple[int, ...]:
        return next(row for row in cases[name]["samples"][phase] if row[:3] == point)

    leaf = sampled("canopy-log-remove", "before", (8, 66, 7))
    if leaf[3:7] != (18, 0, 1, 0):
        raise ValueError("native canopy sample must retain registered leaf ID 18 opacity 1 emission 0")
    transitions = {
        "canopy-log-remove": ((17, 255, 0, 0), (0, 0, 0, 13)),
        "water-remove": ((9, 3, 0, 12), (0, 0, 0, 15)),
        "water-place": ((0, 0, 0, 15), (9, 3, 0, 12)),
    }
    for name, (before, after) in transitions.items():
        target = cases[name]["target"]
        before_row = sampled(name, "before", target)
        after_row = sampled(name, "after", target)
        if (before_row[3], before_row[5], before_row[6], before_row[7]) != before:
            raise ValueError(f"unexpected native material/light state before {name}")
        if (after_row[3], after_row[5], after_row[6], after_row[7]) != after:
            raise ValueError(f"unexpected native material/light state after {name}")


def capture(jar: pathlib.Path, build: pathlib.Path, request: bytes = REQUEST) -> bytes:
    javac, java = shutil.which("javac"), shutil.which("java")
    if not javac or not java:
        raise ValueError("javac and java are required for native material edit audit")
    classes = build / "classes"
    classes.mkdir(parents=True)
    command = [javac, "-encoding", "UTF-8", "-cp", str(jar), "-d", str(classes)]
    command.extend(str(source) for source in JAVA_SOURCES.values())
    compiled = subprocess.run(command, capture_output=True, timeout=60, check=False)
    if compiled.returncode:
        raise ValueError("javac failed: " + compiled.stderr.decode("utf-8", "replace")[-3000:])
    result = subprocess.run(
        [java, "-cp", f"{classes}{os.pathsep}{jar}", "LightMaterialEditOracle", str(jar)],
        input=request, capture_output=True, timeout=60, check=False,
    )
    if result.returncode:
        raise ValueError("native edit oracle failed: " + result.stderr.decode("utf-8", "replace")[-3000:])
    if result.stderr:
        raise ValueError("native edit oracle wrote unexpected stderr: " + result.stderr.decode("utf-8", "replace")[-1000:])
    validate_output(result.stdout)
    return result.stdout


def validate_evidence(captures: list[bytes], output: bytes, manifest: dict, jar: pathlib.Path, request: bytes = REQUEST) -> None:
    if len(captures) != 2:
        raise ValueError("exactly two independent native edit captures are required")
    if captures[0] != captures[1]:
        raise ValueError("independent native edit captures differ")
    if captures[0] != output:
        raise ValueError("native edit capture differs from committed native_edits.tsv")
    validate_output(output)
    if request != REQUEST:
        raise ValueError("input protocol must be the fixed material edit request")
    missing = [field for field in MANIFEST_FIELDS if field not in manifest]
    if missing:
        raise ValueError("edit audit manifest is missing fields: " + ", ".join(missing))
    actual = {
        "jar_sha256": sha(jar.read_bytes()),
        "source_sha256": {name: sha(path.read_bytes()) for name, path in JAVA_SOURCES.items()},
        "input_sha256": sha(request),
        "output_sha256": sha(output),
        "capture_output_sha256": [sha(captures[0]), sha(captures[1])],
        "class_sha256": class_hashes(jar),
        "bootstrap_sha256": BOOTSTRAP_SHA,
        "capture_contract": CAPTURE_CONTRACT,
    }
    for field in MANIFEST_FIELDS:
        if manifest[field] != actual[field]:
            raise ValueError(f"edit audit manifest {field} mismatch")


def run(jar_path: pathlib.Path, write: bool) -> None:
    jar = pinned_jar(jar_path)
    with tempfile.TemporaryDirectory(prefix="light-material-edit-") as temporary:
        base = pathlib.Path(temporary)
        captures = [capture(jar, base / f"run-{index}") for index in (1, 2)]
    if captures[0] != captures[1]:
        raise ValueError("two fresh native edit captures differ")
    output = captures[0]
    manifest = {
        "jar_sha256": sha(jar.read_bytes()),
        "source_sha256": {name: sha(path.read_bytes()) for name, path in JAVA_SOURCES.items()},
        "input_sha256": sha(REQUEST),
        "output_sha256": sha(output),
        "capture_output_sha256": [sha(captures[0]), sha(captures[1])],
        "class_sha256": class_hashes(jar),
        "bootstrap_sha256": BOOTSTRAP_SHA,
        "capture_contract": CAPTURE_CONTRACT,
    }
    if write:
        FIXTURE.mkdir(parents=True, exist_ok=True)
        (FIXTURE / "native_edits.tsv").write_bytes(output)
        (FIXTURE / "edit_audit.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    else:
        validate_evidence(captures, (FIXTURE / "native_edits.tsv").read_bytes(),
                          json.loads((FIXTURE / "edit_audit.json").read_text()), jar)
    print(f"PASS: {len(CASE_NAMES)} synthetic native edits; {len(output.splitlines())} rows; output SHA-256 {sha(output)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=pathlib.Path, default=ROOT.parent / ".local/minecraft/client-b1.7.3.jar")
    parser.add_argument("--write", action="store_true", help="replace the edit fixture after two fresh identical captures")
    args = parser.parse_args()
    try:
        run(args.jar, args.write)
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
