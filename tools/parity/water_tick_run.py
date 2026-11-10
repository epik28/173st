"""Strict Rust/native comparison for the bounded Beta 1.7.3 water tick corpus."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tools.oracle import water_tick_corpus

ROOT = water_tick_corpus.ROOT
PROBE = Path("crates/beta-core/examples/water_tick_probe.rs")
SOURCES = (
    Path("crates/beta-core/src/lib.rs"),
    Path("crates/beta-core/src/water.rs"),
    PROBE,
    Path("crates/beta-core/tests/water_tick_fixture.rs"),
    Path("tools/parity/water_tick_run.py"),
    Path("tools/parity/run.py"),
    Path("Cargo.toml"),
    Path("Cargo.lock"),
    Path("config/local.toml"),
    Path("tools/oracle/water_tick_corpus.py"),
    Path("tools/oracle/src/WaterTickOracle.java"),
    Path("tests/fixtures/water_tick/native.tsv"),
    Path("tests/fixtures/water_tick/input.tsv"),
    Path("tests/fixtures/water_tick/audit.json"),
    Path("tests/fixtures/water_tick_sources/native.tsv"),
    Path("tests/fixtures/water_tick_sources/input.tsv"),
    Path("tests/fixtures/water_tick_sources/audit.json"),
)


class WaterTickRunError(ValueError):
    pass


def _hash(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _hash_sources(root: Path) -> dict[str, str]:
    return {str(path): _hash((root / path).read_bytes()) for path in SOURCES}


def _inputs(case: dict, step: dict, previous: dict[str, tuple[int, int]] | None) -> bytes:
    if previous is None:
        cells = {f"{x},{y},{z}": (0, 0)
                 for y in range(62, 66) for z in range(-2, 3) for x in range(-2, 3)}
        for x, y, z, block_id, metadata in case["inputs"]:
            cells[f"{x},{y},{z}"] = (block_id, metadata)
    else:
        cells = previous
    ordered = sorted(
        ((tuple(int(v) for v in key.split(",")), value) for key, value in cells.items()),
        key=lambda row: (-row[0][1], row[0][2], row[0][0]),
    )
    lines = [
        f"cell\t{pos[0]}\t{pos[1]}\t{pos[2]}\t{block[0]}\t{block[1]}"
        for pos, block in ordered
    ]
    for tick in step["queue_before"]:
        lines.append("queue\t" + "\t".join(str(value) for value in tick))
    lines.append(f"run\t{step['world_time']}\t64")
    return ("\n".join(lines) + "\n").encode("ascii")


def _expected(step: dict) -> bytes:
    processed = sum(tick[4] <= step["world_time"] for tick in step["queue_before"])
    lines = [f"result\tprocessed_ticks={processed}"]
    lines.extend("\t".join(event) for event in step["events"])
    queue = ",".join(":".join(str(value) for value in tick) for tick in step["queue_after"])
    lines.append(f"queue_after\t{queue}")
    for y in range(65, 61, -1):
        for z in range(-2, 3):
            for x in range(-2, 3):
                block_id, metadata = step["state"][f"{x},{y},{z}"]
                lines.append(f"state\t{x}\t{y}\t{z}\t{block_id}\t{metadata}")
    return ("\n".join(lines) + "\n").encode("ascii")


def run_water_tick(root: Path = ROOT, timeout: int = 120) -> tuple[dict, Path]:
    root = root.resolve()
    reference = water_tick_corpus.fixture_check()
    native_live = water_tick_corpus.verify_live(write=False)
    source_reference = water_tick_corpus.source_fixture_check()
    source_live = water_tick_corpus.verify_sources_live(write=False)
    before = _hash_sources(root)
    parent = root / "artifacts/parity/water-tick"
    parent.mkdir(parents=True, exist_ok=True)
    run_dir = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    logs = run_dir / "logs"
    logs.mkdir()
    started = time.monotonic()

    build = subprocess.run(
        ["cargo", "build", "--quiet", "--locked", "--offline", "-p", "beta-core",
         "--example", "water_tick_probe"],
        cwd=root, capture_output=True, timeout=timeout,
    )
    (logs / "build.stdout").write_bytes(build.stdout)
    (logs / "build.stderr").write_bytes(build.stderr)
    if build.returncode or build.stderr:
        raise WaterTickRunError(f"probe build failed: {build.stderr.decode(errors='replace')[-2000:]}")
    binary = root / "target/debug/examples/water_tick_probe"
    if not binary.is_file():
        raise WaterTickRunError("probe binary is missing after successful build")

    mismatches = []
    executions = 0
    datasets = (("base", reference), ("source-formation-1", source_reference))
    for dataset_name, dataset in datasets:
        for case in dataset["cases"]:
            previous = None
            for index, step in enumerate(case["steps"]):
                data = _inputs(case, step, previous)
                output = subprocess.run([str(binary)], cwd=root, input=data,
                                        capture_output=True, timeout=timeout)
                executions += 1
                label = f"{dataset_name}-{case['name']}-{index:02d}"
                (logs / f"{label}.stdin").write_bytes(data)
                (logs / f"{label}.stdout").write_bytes(output.stdout)
                (logs / f"{label}.stderr").write_bytes(output.stderr)
                expected = _expected(step)
                if output.returncode or output.stderr or output.stdout != expected:
                    mismatch = next((i for i, pair in enumerate(zip(output.stdout, expected))
                                     if pair[0] != pair[1]), min(len(output.stdout), len(expected)))
                    mismatches.append({
                        "dataset": dataset_name, "case": case["name"], "step": index,
                        "returncode": output.returncode,
                        "stderr_sha256": _hash(output.stderr),
                        "expected_bytes": len(expected), "actual_bytes": len(output.stdout),
                        "first_differing_byte": mismatch,
                    })
                previous = step["state"]

    after = _hash_sources(root)
    if after != before:
        mismatches.append({"error": "source or pinned fixture changed during comparison"})
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, check=True,
                              capture_output=True, text=True, timeout=10).stdout.strip()
    dirty = bool(subprocess.run(["git", "status", "--porcelain"], cwd=root, check=True,
                                capture_output=True, text=True, timeout=10).stdout)
    report_path = run_dir / "report.json"
    report = {
        "schema_version": 1,
        "suite": "water-tick",
        "scenario_id": "WATER-SIM-001-bounded-v1",
        "status": "pass" if not mismatches else "fail",
        "repository_root": str(root),
        "rust_revision": revision,
        "rust_dirty": dirty,
        "rust_binary_sha256": _hash(binary.read_bytes()),
        "source_hashes": before,
        "source_hashes_after": after,
        "native_fixture_sha256": reference["native_sha256"],
        "native_source_fixture_sha256": source_reference["native_sha256"],
        "native_live": native_live,
        "native_source_live": source_live,
        "case_count": reference["case_count"] + source_reference["case_count"],
        "tick_count": executions,
        "native_event_count": reference["events"] + source_reference["events"],
        "mismatch_count": len(mismatches),
        "mismatches": mismatches,
        "reason": ("all bounded water-tick transcripts match" if not mismatches
                   else f"{len(mismatches)} water-tick mismatch(es)"),
        "duration_ms": round((time.monotonic() - started) * 1000),
        "limits": ["synthetic loaded chunks only", "water IDs 8/9 only",
                   "source transitions limited to four pinned setups; no universal threshold claim",
                   "unloaded-neighbor behavior not claimed"],
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if mismatches:
        raise WaterTickRunError(f"water tick mismatch; inspect {report_path}")
    return report, report_path

