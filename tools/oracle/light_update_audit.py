#!/usr/bin/env python3
"""Rebuild and verify two independent bounded native light-update captures."""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIX = ROOT / "tests/fixtures/light_update"
SOURCE = ROOT / "tools/oracle/src/LightUpdateOracle.java"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
HASH_FIELDS = ("jar_sha256", "input_sha256", "output_sha256", "source_sha256", "class_sha256")
MANIFEST_FIELDS = HASH_FIELDS + ("seed_schema",)
EXPECTED_CLASS_NAMES = {"fd", "lm", "wi", "uu", "eb", "st"}
EXPECTED_SEED_SCHEMA = {
    "loaded_chunk_coordinates": {"x": [-1, 0, 1], "z": [-1, 0, 1]},
    "block_domain": {"y_min": 0, "y_max_inclusive": 127},
    "initial_block_id": 0,
    "initial_metadata": 0,
    "initial_block_light_nibble": 0,
    "initial_sky_light": "native lm.c() all-air profile: 0 at y=0, 15 at y=1..127; native.tsv state rows encode deviations",
    "old_id_override": "for nonzero old ID, set only the target cell with metadata 0 and settle native light queue before initial digest and before-phase queries",
    "provider": "xa.a(0) with native e() initialization; seed 0; synthetic world key light-update",
    "initialization": "all nine chunks allocate zeroed block, metadata, sky-light, block-light and height-map arrays; call lm.c() for each, then drain fd.j() to quiescence; native.tsv domain/state/height rows encode defaults and all deviations",
    "native_tsv_encoding": "one domain row per case; state rows encode every nondefault block/meta/block-light/sky cell; height rows encode every nondefault height cell; domain digest is the complete native initial state digest",
    "material_properties": "native uu.q[id] opacity and uu.s[id] emission",
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def validate_domain_records(output: bytes) -> None:
    """Validate the compressed initial domain and its default-plus-exception rows."""
    lines = output.decode("ascii").splitlines()
    cases: dict[str, list[str]] = {}
    domains: dict[str, dict[str, str]] = {}
    states: dict[str, list[list[str]]] = {}
    heights: dict[str, list[list[str]]] = {}
    for line in lines:
        fields = line.split("\t")
        if fields[0] == "case":
            if len(fields) != 11 or fields[1] in cases:
                raise ValueError("invalid or duplicate native domain case row")
            cases[fields[1]] = fields
        elif fields[0] == "domain":
            if len(fields) < 3 or fields[1] in domains:
                raise ValueError("invalid or duplicate native domain descriptor")
            domains[fields[1]] = dict(item.split("=", 1) for item in fields[2:])
        elif fields[0] == "state":
            if len(fields) != 9:
                raise ValueError("invalid native domain state row")
            states.setdefault(fields[1], []).append(fields)
        elif fields[0] == "height":
            if len(fields) != 5:
                raise ValueError("invalid native domain height row")
            heights.setdefault(fields[1], []).append(fields)

    if not cases or set(cases) != set(domains):
        raise ValueError("native domain descriptors must cover every case exactly once")
    expected = {
        "chunks_x": "-1,0,1",
        "chunks_z": "-1,0,1",
        "y": "0..127",
        "default_block": "0",
        "default_meta": "0",
        "default_block_light": "0",
        "default_sky": "0@y0;15@y1..127(native-lm.c-all-air)",
        "default_height": "0(native-lm.c-all-air)",
        "material": "uu.q[id],uu.s[id]",
        "init": "xa.a(0);lm.c();fd.j-quiescent",
    }
    for name, case in cases.items():
        descriptor = domains[name]
        if any(descriptor.get(key) != value for key, value in expected.items()):
            raise ValueError(f"native domain descriptor does not match the bounded default schema: {name}")
        initial = case[9].removeprefix("initial=")
        if descriptor.get("digest") != initial:
            raise ValueError(f"native domain digest differs from case initial digest: {name}")
        old, old_meta, x, y, z = map(int, (case[2], case[3], case[6], case[7], case[8]))
        expected_override = "-" if old == 0 else f"{x},{y},{z},{old},{old_meta};fd.a+fd.j-quiescent"
        if descriptor.get("old_override") != expected_override:
            raise ValueError(f"native domain old-ID override is missing or inconsistent: {name}")
        seen_states: set[tuple[int, int, int]] = set()
        for state in states.get(name, []):
            sx, sy, sz, block, meta, block_light, sky = map(int, state[2:9])
            cell = (sx, sy, sz)
            if cell in seen_states or not (-16 <= sx <= 31 and -16 <= sz <= 31 and 0 <= sy <= 127):
                raise ValueError(f"duplicate or out-of-domain native state exception: {name} {cell}")
            seen_states.add(cell)
            default_sky = 0 if sy == 0 else 15
            if block == 0 and meta == 0 and block_light == 0 and sky == default_sky:
                raise ValueError(f"redundant native state exception equals its declared default: {name} {cell}")
        seen_heights: set[tuple[int, int]] = set()
        for height in heights.get(name, []):
            hx, hz, value = map(int, height[2:5])
            column = (hx, hz)
            if column in seen_heights or not (-16 <= hx <= 31 and -16 <= hz <= 31) or value == 0:
                raise ValueError(f"duplicate or invalid native height exception: {name} {column}")
            seen_heights.add(column)
        if old != 0 and (x, y, z) not in seen_states:
            raise ValueError(f"native old-ID override is absent from state exceptions: {name}")


def validate_deltas(output: bytes) -> None:
    """Assert native light deltas and the loaded-to-unloaded chunk boundary."""
    validate_domain_records(output)
    rows: dict[tuple[str, str, int, int, int], list[str]] = {}
    for line in output.decode("ascii").splitlines():
        fields = line.split("\t")
        if len(fields) == 12 and fields[1] in {"before", "after"}:
            key = (fields[0], fields[1], int(fields[2]), int(fields[3]), int(fields[4]))
            if key in rows:
                raise ValueError(f"duplicate light query row: {key}")
            rows[key] = fields

    def row(case: str, phase: str, x: int, y: int, z: int) -> list[str]:
        key = (case, phase, x, y, z)
        if key not in rows:
            raise ValueError(f"missing required light query row: {key}")
        return rows[key]

    # Column 9 is direct sky nibble, column 10 block-light nibble, column 11
    # the native combined-light query. Check both the removed block and the
    # cell directly below it, where block opacity changes direct sky 14 <-> 15.
    expected = (
        ("place", "before", 8, 63, 8, 15),
        ("place", "after", 8, 63, 8, 14),
        ("remove", "before", 8, 63, 8, 14),
        ("remove", "after", 8, 63, 8, 15),
    )
    for case, phase, x, y, z, sky in expected:
        fields = row(case, phase, x, y, z)
        if int(fields[9]) != sky or int(fields[11]) != sky:
            raise ValueError(
                f"false sky/combined-light assertion at {case} {phase} ({x},{y},{z}): "
                f"sky={fields[9]} combined={fields[11]}, expected {sky}"
            )
    if int(row("place", "before", 8, 64, 8)[5]) != 0 or int(
        row("place", "after", 8, 64, 8)[5]
    ) != 1:
        raise ValueError("place mutation is absent from native capture")
    if int(row("remove", "before", 8, 64, 8)[5]) != 1 or int(
        row("remove", "after", 8, 64, 8)[5]
    ) != 0:
        raise ValueError("remove mutation is absent from native capture")

    # At the east edge of loaded chunk cx=1, the native block setter can edit
    # x=31, but the light queue's radius-one loaded-area check skips that
    # column while cx=2 is absent. The expected below-block sky is therefore
    # zero in this bounded native fixture. x=32 has no chunk data at all.
    target_before = row("edge-x31", "before", 31, 64, 8)
    target_after = row("edge-x31", "after", 31, 64, 8)
    if target_before[5:9] != ["0", "0", "0", "0"]:
        raise ValueError("edge-x31 place did not start from an air/meta-0 target")
    if target_after[5:9] != ["1", "0", "255", "0"]:
        raise ValueError("edge-x31 native setter did not place id 1/meta 0")

    below_before = row("edge-x31", "before", 31, 63, 8)
    below_after = row("edge-x31", "after", 31, 63, 8)
    if int(below_before[9]) != 15 or int(below_before[11]) != 15:
        raise ValueError("edge-x31 below-cell initial sky/combined light must be 15")
    if int(below_after[9]) != 0 or int(below_after[10]) != 0 or int(below_after[11]) != 0:
        raise ValueError("edge-x31 below-cell native sky/block/combined light must be 0")

    for phase in ("before", "after"):
        loaded_neighbor = row("edge-x31", phase, 30, 63, 8)
        if int(loaded_neighbor[9]) != 15 or int(loaded_neighbor[10]) != 0 or int(loaded_neighbor[11]) != 15:
            raise ValueError("edge-x31 loaded neighbor at x=30 must retain sky 15")

    for phase in ("before", "after"):
        unloaded = row("edge-x31", phase, 32, 63, 8)
        if unloaded[5:] != ["UNLOADED"] * 7:
            raise ValueError(f"edge-x31 x=32 {phase} query must be explicitly UNLOADED, not air/light data")


def validate_evidence(
    captures: list[bytes],
    capture_audits: list[dict],
    expected_output: bytes,
    input_bytes: bytes,
    expected_manifest: dict,
    source_bytes: bytes,
    jar_path: pathlib.Path,
) -> None:
    """Validate reproducibility, raw outputs and all pinned audit evidence."""
    if len(captures) != 2 or len(capture_audits) != 2:
        raise ValueError("exactly two independent captures and audit records are required")
    if captures[0] != captures[1]:
        raise ValueError("independent native captures differ")
    if captures[0] != expected_output:
        raise ValueError("native capture differs from committed native.tsv")
    validate_deltas(captures[0])

    missing = [field for field in MANIFEST_FIELDS if field not in expected_manifest]
    if missing:
        raise ValueError(f"committed audit manifest is missing fields: {', '.join(missing)}")
    if expected_manifest["jar_sha256"] != sha(jar_path.read_bytes()):
        raise ValueError("committed audit manifest JAR hash mismatch")
    if expected_manifest["input_sha256"] != sha(input_bytes):
        raise ValueError("committed audit manifest input hash mismatch")
    if expected_manifest["output_sha256"] != sha(expected_output):
        raise ValueError("committed audit manifest output hash mismatch")
    if expected_manifest["source_sha256"] != sha(source_bytes):
        raise ValueError("committed audit manifest source hash mismatch")
    if not isinstance(expected_manifest["class_sha256"], dict) or set(expected_manifest["class_sha256"]) != EXPECTED_CLASS_NAMES:
        raise ValueError("committed audit manifest class_sha256 must contain exactly fd/lm/wi/uu/eb/st")
    try:
        with zipfile.ZipFile(jar_path) as archive:
            for name, digest in expected_manifest["class_sha256"].items():
                entry = name.replace(".", "/") + ".class"
                if sha(archive.read(entry)) != digest:
                    raise ValueError(f"pinned JAR class hash mismatch: {name}")
    except (KeyError, zipfile.BadZipFile) as exc:
        raise ValueError(f"cannot verify pinned JAR class entries: {exc}") from exc
    if expected_manifest["seed_schema"] != EXPECTED_SEED_SCHEMA:
        raise ValueError("committed audit manifest seed_schema does not match the bounded native fixture contract")

    for index, audit in enumerate(capture_audits, start=1):
        missing = [field for field in MANIFEST_FIELDS if field not in audit]
        if missing:
            raise ValueError(f"capture {index} audit is missing fields: {', '.join(missing)}")
        for field in MANIFEST_FIELDS:
            if audit[field] != expected_manifest[field]:
                raise ValueError(f"capture {index} audit {field} differs from committed manifest")


def pinned_jar(path: pathlib.Path) -> pathlib.Path:
    path = path.resolve()
    if not path.is_file() or sha(path.read_bytes()) != JAR_SHA:
        raise ValueError("wrong or missing pinned client JAR")
    return path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--jar", default=str(ROOT.parent / ".local/minecraft/client-b1.7.3.jar"))
    args = parser.parse_args()
    try:
        jar = pinned_jar(pathlib.Path(args.jar))
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 2
    if not shutil.which("javac") or not shutil.which("java"):
        print("javac and java are required", file=sys.stderr)
        return 2

    input_bytes = (FIX / "input.tsv").read_bytes()
    expected_output = (FIX / "native.tsv").read_bytes()
    source_bytes = SOURCE.read_bytes()
    expected_manifest = json.loads((FIX / "audit.json").read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="light-update-audit-") as tmp:
        temp = pathlib.Path(tmp)
        classes = temp / "classes"
        classes.mkdir()
        subprocess.run(
            ["javac", "-cp", str(jar), "-d", str(classes), str(SOURCE)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        captures: list[bytes] = []
        capture_audits: list[dict] = []
        for index in (1, 2):
            audit_path = temp / f"capture-{index}.json"
            proc = subprocess.run(
                ["java", "-XX:-UsePerfData", "-cp", f"{classes}{__import__('os').pathsep}{jar}", "LightUpdateOracle", str(jar), str(audit_path), str(SOURCE)],
                input=input_bytes,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=True,
            )
            captures.append(proc.stdout)
            capture_audits.append(json.loads(audit_path.read_text(encoding="utf-8")))
        validate_evidence(
            captures,
            capture_audits,
            expected_output,
            input_bytes,
            expected_manifest,
            source_bytes,
            jar,
        )
    print(
        f"PASS: 2 native captures, {len(expected_output.splitlines())} rows, "
        f"output sha256={sha(expected_output)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
