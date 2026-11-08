"""Verify retained FLUID-001 native/Rust report contents and provenance."""

import json
from pathlib import Path
import sys

try:
    from tools.oracle import fluid_corpus
except ImportError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "oracle"))
    import fluid_corpus

try:
    from fluid_run import compare_geometry, geometry_rows, sha256
except ImportError:
    from tools.parity.fluid_run import compare_geometry, geometry_rows, sha256

ROOT = Path(__file__).resolve().parents[2]


def _artifact(report_path, value):
    path = Path(value)
    if not path.is_absolute():
        path = report_path.parent / path
    resolved = path.resolve(strict=True)
    if not resolved.is_relative_to(report_path.parent.resolve()):
        raise ValueError("retained artifact escapes its report directory")
    return resolved


def verify_report(path):
    report_path = Path(path).resolve(strict=True)
    expected_root = (ROOT / "artifacts/parity/fluid").resolve()
    if not report_path.is_relative_to(expected_root) or report_path.name != "report.json":
        raise ValueError("report is outside the retained fluid artifact tree")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if report.get("schema_version") != 1 or report.get("suite") != "fluid":
        raise ValueError("unsupported fluid report schema")
    if report.get("scenario_id") != "FLUID-001-water-native-geometry":
        raise ValueError("unexpected fluid scenario")
    if report.get("reference_jar_sha256") != fluid_corpus.JAR_SHA:
        raise ValueError("report is not bound to the pinned original client JAR")
    if report.get("expected_fixture_sha256") != sha256((ROOT / "tests/fixtures/fluid/native.tsv").read_bytes()):
        raise ValueError("report expected fixture hash differs from committed C01 reference")
    if report.get("input_sha256") != sha256((ROOT / "tests/fixtures/fluid/input.tsv").read_bytes()):
        raise ValueError("report input hash differs from frozen C01 suite")
    before, after = report.get("source_hashes"), report.get("source_hashes_after")
    if not isinstance(before, dict) or before != after:
        raise ValueError("tracked sources changed during recorded comparison")
    processes = report.get("processes")
    if not isinstance(processes, dict):
        raise ValueError("report process audit missing")
    for name, process in processes.items():
        if process.get("returncode") != 0 or process.get("error") or process.get("timeout"):
            raise ValueError(f"{name} did not complete successfully")
        for stream in ("stdout", "stderr"):
            stream_path = _artifact(report_path, process.get(stream, ""))
            raw = stream_path.read_bytes()
            if sha256(raw) != process.get(stream + "_sha256"):
                raise ValueError(f"{name} {stream} hash mismatch")
    native = []
    for name in ("native-run", "native-repeat"):
        if Path(_artifact(report_path, processes[name]["stderr"])).read_bytes():
            raise ValueError(f"{name} wrote unexpected stderr")
    for name in ("native-run_output", "native-repeat_output"):
        raw = _artifact(report_path, report.get("log_paths", {}).get(name, "")).read_bytes()
        fluid_corpus.parse_native(raw)
        native.append(raw)
    fixture = (ROOT / "tests/fixtures/fluid/native.tsv").read_bytes()
    if native[0] != native[1] or native[0] != fixture:
        raise ValueError("retained native runs do not match each other and committed fixture")
    if report.get("reference_output_sha256") != sha256(native[0]):
        raise ValueError("native output digest mismatch")
    if report.get("case_count") != 44 or report.get("vertex_count") != 1242:
        raise ValueError("native coverage counts differ from committed corpus")
    if report.get("native_audit", {}).get("repeatable") is not True:
        raise ValueError("native repeatability audit missing")
    if report.get("reference_class_sha256") != fluid_corpus.EXPECTED_CLASSES:
        raise ValueError("native class hashes differ from C01")

    status = report.get("status")
    probe_present = (ROOT / "crates/beta-viewer/examples/fluid_probe.rs").is_file()
    if status == "not_implemented":
        if probe_present or report.get("rust_binary_sha256") is not None or report.get("rust_output_sha256") is not None:
            raise ValueError("not_implemented report contains Rust probe evidence")
    elif status in {"pass", "fail"}:
        if not probe_present:
            raise ValueError("Rust probe is absent from a completed comparison report")
        if (not isinstance(report.get("rust_revision"), str)
                or len(report["rust_revision"]) != 40
                or report.get("rust_dirty") is not False):
            raise ValueError("completed comparison is not bound to a clean committed Rust revision")
        binary_path = _artifact(report_path, report.get("log_paths", {}).get("rust_binary", ""))
        if sha256(binary_path.read_bytes()) != report.get("rust_binary_sha256"):
            raise ValueError("retained Rust binary digest mismatch")
        rust_path = _artifact(report_path, report.get("log_paths", {}).get("rust_output", ""))
        rust = rust_path.read_bytes()
        if sha256(rust) != report.get("rust_output_sha256"):
            raise ValueError("retained Rust output digest mismatch")
        comparison = compare_geometry(native[0], rust)
        expected_status = "pass" if comparison["mismatch_count"] == 0 else "fail"
        if status != expected_status or report.get("mismatch_count") != comparison["mismatch_count"]:
            raise ValueError("stored parity result differs from retained outputs")
        if report.get("first_mismatch") != comparison["first_mismatch"]:
            raise ValueError("stored first mismatch differs from retained outputs")
    elif status == "error":
        if not isinstance(report.get("reason"), str) or not report["reason"]:
            raise ValueError("error report has no reason")
    else:
        raise ValueError("unknown fluid report status")
    return {"status": status, "case_count": 44, "vertex_count": 1242,
            "reference_sha256": sha256(native[0]), "verified_processes": len(processes)}


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 tools/parity/fluid_review.py REPORT.json")
    try:
        print(json.dumps(verify_report(sys.argv[1]), indent=2))
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"invalid fluid report: {error}", file=sys.stderr)
        raise SystemExit(1)
