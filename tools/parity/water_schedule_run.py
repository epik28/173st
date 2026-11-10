"""Strict Rust/native comparison for bounded water tick seeding actions."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tools.oracle import water_schedule_corpus

ROOT = water_schedule_corpus.ROOT
PROBE = Path("crates/beta-viewer/examples/water_schedule_probe.rs")
SOURCES = (
    Path("crates/beta-viewer/examples/water_schedule_probe.rs"),
    Path("crates/beta-viewer/Cargo.toml"),
    Path("crates/beta-viewer/src/lib.rs"),
    Path("crates/beta-viewer/src/main.rs"),
    Path("crates/beta-viewer/src/water_runtime.rs"),
    Path("crates/beta-core/src/water.rs"),
    Path("crates/beta-core/src/block.rs"),
    Path("tools/parity/water_schedule_run.py"),
    Path("tools/oracle/water_schedule_corpus.py"),
    Path("tools/oracle/src/WaterTickOracle.java"),
    Path("tests/fixtures/water_tick_schedule/input.tsv"),
    Path("tests/fixtures/water_tick_schedule/native.tsv"),
    Path("tests/fixtures/water_tick_schedule/audit.json"),
    Path("Cargo.toml"),
    Path("Cargo.lock"),
    Path("config/local.toml"),
)


class WaterScheduleRunError(ValueError):
    pass


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _source_hashes(root: Path) -> dict[str, str]:
    return {str(path): _sha((root / path).read_bytes()) for path in SOURCES}


def expected_stream(native: dict) -> bytes:
    lines: list[str] = []
    for case in native["cases"]:
        lines.append(f"case\t{case['name']}")
        lines.extend("\t".join(event) for event in case["events"])
        queue = "\t".join(
            ":".join(str(value) for value in tick)
            for tick in case["queue_after"]
        )
        lines.append("queue_after" + (f"\t{queue}" if queue else ""))
        cells = []
        for y in range(65, 61, -1):
            for z in range(-1, 2):
                for x in range(-1, 2):
                    block_id, metadata = case["state"][f"{x},{y},{z}"]
                    cells.append(f"{x},{y},{z}={block_id}:{metadata}")
        lines.append("state\t" + ";".join(cells) + ";")
        lines.append(f"#case_end\t{case['name']}")
    return ("\n".join(lines) + "\n").encode("ascii")


def run_water_schedule(root: Path = ROOT, timeout: int = 120) -> tuple[dict, Path]:
    root = root.resolve()
    reference = water_schedule_corpus.fixture_check()
    live = water_schedule_corpus.verify_live(write=False)
    input_path = root / "tests/fixtures/water_tick_schedule/input.tsv"
    input_bytes = input_path.read_bytes()
    expected = expected_stream(reference)
    before = _source_hashes(root)
    parent = root / "artifacts/parity/water-schedule"
    parent.mkdir(parents=True, exist_ok=True)
    run_dir = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    logs = run_dir / "logs"
    logs.mkdir()
    started = time.monotonic()

    build = subprocess.run(
        ["cargo", "build", "--quiet", "--locked", "--offline", "-p", "beta-viewer",
         "--example", "water_schedule_probe"],
        cwd=root, capture_output=True, timeout=timeout,
    )
    (logs / "build.stdout").write_bytes(build.stdout)
    (logs / "build.stderr").write_bytes(build.stderr)
    if build.returncode or build.stderr:
        raise WaterScheduleRunError(
            f"probe build failed: {build.stderr.decode(errors='replace')[-2000:]}"
        )
    binary = root / "target/debug/examples/water_schedule_probe"
    if not binary.is_file():
        raise WaterScheduleRunError("probe binary is missing after successful build")

    result = subprocess.run([str(binary)], cwd=root, input=input_bytes,
                            capture_output=True, timeout=timeout)
    (logs / "probe.stdin").write_bytes(input_bytes)
    (logs / "native-expected.tsv").write_bytes(expected)
    (logs / "probe.stdout").write_bytes(result.stdout)
    (logs / "probe.stderr").write_bytes(result.stderr)
    after = _source_hashes(root)
    report = {
        "schema_version": 1,
        "suite": "water-schedule",
        "status": "pass" if result.returncode == 0 and not result.stderr and result.stdout == expected else "fail",
        "reason": "ordered native seed/edit/schedule/neighbor transcript, deduped queue, and full state match" if result.returncode == 0 and not result.stderr and result.stdout == expected else "Rust water schedule probe differs from the pinned native transcript",
        "case_count": reference["case_count"],
        "native_event_count": reference["events"],
        "input_sha256": _sha(input_bytes),
        "expected_sha256": _sha(expected),
        "actual_sha256": _sha(result.stdout),
        "native_sha256": reference["native_sha256"],
        "native_live": live,
        "source_hashes_before": before,
        "source_hashes_after": after,
        "source_unchanged": before == after,
        "probe_exit_code": result.returncode,
        "probe_stderr": result.stderr.decode(errors="replace"),
        "duration_seconds": round(time.monotonic() - started, 3),
    }
    (run_dir / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    if report["status"] != "pass" or not report["source_unchanged"]:
        raise WaterScheduleRunError(
            f"water schedule strict parity failed; report: {run_dir / 'report.json'}"
        )
    return report, run_dir
