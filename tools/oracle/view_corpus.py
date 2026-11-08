#!/usr/bin/env python3
"""Run the original Beta client against a fixed copied 5x5 VIEW-001 scene."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import zipfile

try:
    from . import world_corpus
except ImportError:  # Direct script execution adds tools/oracle rather than tools to sys.path.
    import world_corpus

ROOT = Path(__file__).resolve().parents[2]
JAR_SHA = world_corpus.JAR_SHA
TERRAIN_SHA = "9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf"
SCENE_CHUNKS = [(x, z) for z in range(-3, 2) for x in range(-3, 2)]
EXPECTED_HEADER = "#VIEW_NATIVE\t1\t-9\t64\t-5\t-3\t-3"
EXPECTED_CLASSES = {
    "uu_sha256": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "cv_sha256": "ce8f363e1f7566530ac31680eabb8c715d9ddc7c97f31cf0a62f953e069a6863",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def source(root=ROOT):
    paths = world_corpus.config(root)
    jar = (root / paths["client_jar"]).resolve(strict=True)
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured client JAR hash mismatch")
    terrain_value = paths.get("terrain_png")
    if terrain_value:
        terrain = (root / terrain_value).resolve()
    else:
        terrain = (root / paths["minecraft_reference"] / "terrain.png").resolve()
    terrain.parent.mkdir(parents=True, exist_ok=True)
    if not terrain.exists():
        with zipfile.ZipFile(jar) as archive:
            contents = archive.read("terrain.png")
        if digest(contents) != TERRAIN_SHA:
            raise ValueError("original terrain.png hash mismatch")
        with terrain.open("xb") as output:
            output.write(contents)
    if terrain.is_symlink() or not terrain.is_file() or digest(terrain.read_bytes()) != TERRAIN_SHA:
        raise ValueError("configured terrain.png must match the pinned original resource")
    return jar, terrain


def fixture_rows():
    rows = ["# VIEW-001 original-client fixed scene; chunk coordinates are spawn chunk ±2"]
    rows.extend(f"{x}\t{z}" for x, z in SCENE_CHUNKS)
    return "\n".join(rows) + "\n"


def parse_stdout(raw):
    text = raw.decode("utf-8", errors="strict")
    lines = text.splitlines()
    if not lines or not lines[0].startswith("#VIEW_NATIVE\t1\t"):
        raise ValueError("native output header missing")
    header_fields = lines[0].split("\t")
    if len(header_fields) != 7:
        raise ValueError("native scene header has the wrong field count")
    if lines[0] != EXPECTED_HEADER:
        raise ValueError("pinned test world spawn or 5x5 scene origin changed")
    spawn_x, spawn_y, spawn_z, first_chunk_x, first_chunk_z = map(int, header_fields[2:])
    if (first_chunk_x, first_chunk_z) != (spawn_x // 16 - 2, spawn_z // 16 - 2):
        raise ValueError("native scene bounds do not match the spawn chunk")
    sections = {"states": [], "neighbors": []}
    section = None
    for line in lines[1:]:
        if line == "#states":
            section = "states"
            continue
        if line == "#neighbors":
            section = "neighbors"
            continue
        if section is None:
            raise ValueError("native output row before section")
        fields = line.split("\t")
        expected = 10 if section == "states" else 7
        if len(fields) != expected or fields[0] != ("state" if section == "states" else "neighbor"):
            raise ValueError("malformed native output row")
        sections[section].append(tuple(fields))
    if not sections["states"]:
        raise ValueError("fixed scene contains no non-air blocks")
    if sections["states"] != sorted(sections["states"], key=lambda row: (int(row[1]), int(row[2]), int(row[3]))):
        raise ValueError("state rows are not sorted")
    if sections["neighbors"] != sorted(sections["neighbors"], key=lambda row: tuple(map(int, row[1:6]))):
        raise ValueError("neighbor rows are not sorted")
    if len({(row[1], row[2], row[3]) for row in sections["states"]}) != len(sections["states"]):
        raise ValueError("duplicate native block/context state")
    if len(set(sections["neighbors"])) != len(sections["neighbors"]):
        raise ValueError("duplicate neighbor state pair")
    for row in sections["states"]:
        block_id, metadata, above_id, render_type, opaque, full, alpha, tiles, supported = row[1:]
        if not (0 <= int(block_id) <= 255 and 0 <= int(metadata) <= 15):
            raise ValueError("native block state is outside Beta byte/nibble range")
        if not -1 <= int(above_id) <= 255:
            raise ValueError("native block context is outside byte range")
        if render_type == "-1" and opaque == "0" and full == "0":
            if alpha != "------" or tiles != "-" or supported != "0":
                raise ValueError("undefined native block must be explicitly unsupported")
            continue
        if not (0 <= int(render_type) <= 255 and opaque in {"0", "1"} and full in {"0", "1"}):
            raise ValueError("invalid native render classification")
        if len(alpha) != 6 or any(value not in "01" for value in alpha):
            raise ValueError("invalid atlas alpha mask")
        tile_ids = [int(value) for value in tiles.split(",")]
        if len(tile_ids) != 6 or any(not 0 <= value <= 255 for value in tile_ids):
            raise ValueError("invalid native face tile mapping")
        expected_supported = render_type == "0" and opaque == "1" and full == "1" and alpha == "111111"
        if supported != str(int(expected_supported)):
            raise ValueError("native supported-state classification is inconsistent")
    return text, lines[0], sections


def run_native(run_dir, input_dir, jar, terrain, root=ROOT, timeout=120):
    classes = run_dir / "classes"
    classes.mkdir()
    source_file = root / "tools/oracle/src/ViewOracle.java"
    subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(source_file)],
                   check=True, capture_output=True, timeout=40)
    command = ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", f"{classes}:{jar}",
               "ViewOracle", str(jar), str(input_dir), str(terrain)]
    result = subprocess.run(command, capture_output=True, timeout=timeout)
    (run_dir / "native.stdout.tsv").write_bytes(result.stdout)
    (run_dir / "native.stderr.log").write_bytes(result.stderr)
    if result.returncode:
        raise RuntimeError(f"native view oracle failed ({result.returncode}): "
                           f"{result.stderr.decode(errors='replace')[-3000:]}")
    text, header, sections = parse_stdout(result.stdout)
    audit_lines = [line for line in result.stderr.decode("utf-8", errors="strict").splitlines() if line.strip()]
    if len(audit_lines) != 1:
        raise ValueError("native audit must be one JSON line on stderr")
    audit = json.loads(audit_lines[0])
    if audit.get("jar_sha256") != JAR_SHA or audit.get("terrain_sha256") != TERRAIN_SHA:
        raise ValueError("native oracle audit source hash mismatch")
    if int(audit.get("scene_chunks", "-1")) != 25:
        raise ValueError("native oracle did not read exactly the 25 scene slots")
    state_count = len({(row[1], row[2]) for row in sections["states"]})
    if int(audit.get("unique_states", "-1")) != state_count:
        raise ValueError("native unique state count disagrees with TSV")
    if int(audit.get("mapping_cases", "-1")) != len(sections["states"]):
        raise ValueError("native context mapping count disagrees with TSV")
    if int(audit.get("world_aware_tile_cases", "-1")) != len(sections["states"]) - state_count:
        raise ValueError("native world-aware tile context count disagrees with TSV")
    if int(audit.get("unique_neighbor_pairs", "-1")) != len(sections["neighbors"]):
        raise ValueError("native neighbor pair count disagrees with TSV")
    if int(audit.get("native_calls_tile", "-1")) != 6 * len(sections["states"]):
        raise ValueError("native face-tile call count mismatch")
    if int(audit.get("native_calls_visibility", "-1")) != 6 * len(sections["states"]) + len(sections["neighbors"]):
        raise ValueError("native face-visibility call count mismatch")
    if any(audit.get(key) != value for key, value in EXPECTED_CLASSES.items()):
        raise ValueError("pinned original renderer/block class hash mismatch")
    if audit.get("native_calls_ij") != "26" or audit.get("native_calls_ei") != "1" or audit.get("native_calls_qj") != "25":
        raise ValueError("native metadata/NBT/region call count mismatch")
    if int(audit.get("native_world_or_chunk_loads", "-1")) != 0 or int(audit.get("native_renderer_vertex_calls", "-1")) != 0:
        raise ValueError("unexpected native World load or unreported renderer path")
    return text, header, sections, audit


def verify(root=ROOT, write=False):
    jar, terrain = source(root)
    artifacts = root / world_corpus.config(root).get("artifacts", "artifacts") / "oracle-view"
    artifacts.mkdir(parents=True, exist_ok=True)
    run_dir = Path(tempfile.mkdtemp(prefix="run-", dir=artifacts))
    input_dir, evidence = world_corpus.snapshot(run_dir, root)
    try:
        first = run_native(run_dir, input_dir, jar, terrain, root)
        repeat_dir = run_dir / "repeat"
        repeat_dir.mkdir()
        second = run_native(repeat_dir, input_dir, jar, terrain, root)
        if first[:2] != second[:2] or first[3] != second[3]:
            raise AssertionError("two original-client scene runs differ")
        world_corpus.verify_snapshot(run_dir, evidence, root)
    finally:
        world_corpus.verify_snapshot(run_dir, evidence, root)
    output, header, sections, audit = first
    expected = fixture_rows()
    fixture_dir = root / "tests/fixtures/view"
    fixture_dir.mkdir(parents=True, exist_ok=True)
    live_path, scene_path, audit_path = (fixture_dir / name for name in ("live.tsv", "scene.tsv", "audit.json"))
    audit_record = {"header": header, "native_audit": audit,
                    "live_sha256": digest(output.encode()), "scene_sha256": digest(expected.encode()),
                    "source_before": evidence["source_before"], "source_after": evidence["source_after"],
                    "copied_before": evidence["copied_before"], "copied_after": evidence["copied_after"]}
    audit_text = json.dumps(audit_record, ensure_ascii=False, indent=2) + "\n"
    if write:
        live_path.write_text(output)
        scene_path.write_text(expected)
        audit_path.write_text(audit_text)
    elif not live_path.is_file() or live_path.read_text() != output:
        raise AssertionError("native output differs from committed VIEW-001 fixture; use --write after review")
    elif not scene_path.is_file() or scene_path.read_text() != expected:
        raise AssertionError("fixed scene coordinate fixture changed")
    elif not audit_path.is_file() or audit_path.read_text() != audit_text:
        raise AssertionError("native provenance audit differs from pinned sources/calls")
    return {"scene_chunks": 25, "unique_states": len({(row[1], row[2]) for row in sections["states"]}),
            "mapping_cases": len(sections["states"]),
            "unique_neighbor_pairs": len(sections["neighbors"]),
            "supported_states": sum(row[9] == "1" for row in sections["states"]),
            "unsupported_states": sum(row[9] == "0" for row in sections["states"]),
            "native_calls": audit, "repeatable": True, "run_dir": str(run_dir)}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="write the reviewed native result to the tracked fixture")
    args = parser.parse_args()
    print(json.dumps(verify(write=args.write), ensure_ascii=False, indent=2))
