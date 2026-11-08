"""Verify retained BLOCK-001 ray-only parity reports and their artifacts."""

import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

try:
    from tools.oracle import block_corpus
    from tools.parity import block_run
except ImportError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "oracle"))
    import block_corpus
    from tools.parity import block_run

def _artifact(report_path, value):
    path = Path(value)
    if not path.is_absolute():
        path = report_path.parent / path
    resolved = path.resolve(strict=True)
    if not resolved.is_relative_to(report_path.parent.resolve()):
        raise ValueError("retained block artifact escapes its report directory")
    return resolved


def verify_report(path, root=ROOT):
    report_path = Path(path).resolve(strict=True)
    report_tree = (root / "artifacts/parity/block").resolve()
    if not report_path.is_relative_to(report_tree) or report_path.name != "report.json":
        raise ValueError("report is outside the retained block artifact tree")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if (report.get("schema_version") != 1 or report.get("suite") != "block"
            or report.get("scenario_id") != "BLOCK-001-ray-only-v1"):
        raise ValueError("unsupported block report schema/scenario")
    if report.get("scope") != "ray-only" or report.get("actions_compared") is not False:
        raise ValueError("block report must be explicitly ray-only")
    if report.get("action_status") != "not_implemented":
        raise ValueError("action parity cannot be reported without native action evidence")
    if report.get("reference_source") != "live pinned Beta 1.7.3 client oracle":
        raise ValueError("reference is not classified as live native evidence")
    if report.get("reference_jar_sha256") != block_corpus.JAR_SHA:
        raise ValueError("report is not bound to pinned original client JAR")
    fixture = (root / "tests/fixtures/block/native.tsv").read_bytes()
    inputs = (root / "tests/fixtures/block/input.tsv").read_bytes()
    if report.get("expected_fixture_sha256") != block_run.sha256(fixture):
        raise ValueError("report expected output hash differs from pinned C01 fixture")
    if report.get("input_fixture_sha256") != block_run.sha256(inputs):
        raise ValueError("report input hash differs from pinned Java-generated ray input fixture")
    expected_path = _artifact(report_path, report.get("log_paths", {}).get("retained_expected", ""))
    if expected_path.read_bytes() != fixture:
        raise ValueError("retained native reference differs from committed C01 fixture")
    probe_input_path = _artifact(report_path, report.get("log_paths", {}).get("probe_input", ""))
    if probe_input_path.read_bytes() != inputs or block_run.sha256(inputs) != report.get("probe_input_sha256"):
        raise ValueError("retained probe request differs from pinned native-generated input fixture")
    before, after = report.get("source_hashes"), report.get("source_hashes_after")
    if not isinstance(before, dict) or before != after:
        raise ValueError("tracked sources changed during recorded comparison")
    if before != block_run.source_snapshot(root):
        raise ValueError("report source hashes differ from current reviewed inputs")
    processes = report.get("processes")
    if not isinstance(processes, dict) or not processes:
        raise ValueError("report process audit missing")
    for name, process in processes.items():
        if report.get("status") != "error" and (process.get("returncode") != 0 or process.get("timeout") or process.get("error")):
            raise ValueError(f"{name} did not complete successfully")
        for stream in ("stdout", "stderr"):
            stream_path = _artifact(report_path, process.get(stream, ""))
            raw = stream_path.read_bytes()
            if block_run.sha256(raw) != process.get(stream + "_sha256"):
                raise ValueError(f"{name} {stream} hash mismatch")

    if report.get("status") in {"pass", "fail", "not_implemented"}:
        outputs = []
        audits = []
        input_outputs = []
        for run_name in ("native-run", "native-repeat"):
            proc = processes[run_name]
            if proc.get("returncode") != 0 or proc.get("timeout") or proc.get("error"):
                raise ValueError(f"{run_name} did not complete successfully")
            audit_path = _artifact(report_path, proc["stderr"])
            audit = json.loads(audit_path.read_text(encoding="utf-8"))
            if (audit.get("jar_sha256") != block_corpus.JAR_SHA or audit.get("classes") != block_corpus.CLASS_SHA
                    or audit.get("world_constructors") != 0 or audit.get("native_cases") != 17):
                raise ValueError(f"{run_name} native provenance audit mismatch")
            audits.append(audit)
            out_path = _artifact(report_path, report["log_paths"][run_name + "_output"])
            native = out_path.read_bytes()
            block_corpus.parse(native)
            outputs.append(native)
            input_proc = processes[run_name + "-inputs"]
            if input_proc.get("returncode") != 0 or input_proc.get("timeout") or input_proc.get("error"):
                raise ValueError(f"{run_name} input generator did not complete")
            emitted = _artifact(report_path, input_proc["stdout"]).read_bytes()
            block_corpus.parse_inputs(emitted)
            if emitted != inputs:
                raise ValueError("retained Java inputs differ from pinned case table")
            input_outputs.append(emitted)
        if outputs[0] != outputs[1] or outputs[0] != fixture or input_outputs[0] != input_outputs[1]:
            raise ValueError("native repetitions differ from fixture or each other")
        if audits[0] != audits[1]:
            raise ValueError("native provenance audits differ between repetitions")
        if report.get("reference_output_sha256") != block_run.sha256(outputs[0]):
            raise ValueError("native output hash mismatch")
        if report.get("reference_class_sha256") != block_corpus.CLASS_SHA:
            raise ValueError("native class hashes differ from pinned C01 map")
        if report.get("case_count") != 17 or report.get("native_audit", {}).get("repeatable") is not True:
            raise ValueError("native ray coverage/repeatability audit missing")

    status = report.get("status")
    probe_present = (root / "crates/beta-core/examples/block_probe.rs").is_file()
    if status == "not_implemented":
        if probe_present or report.get("rust_binary_sha256") is not None or report.get("probe_output_sha256") is not None:
            raise ValueError("not_implemented report contains Rust probe evidence")
    elif status in {"pass", "fail"}:
        if not probe_present:
            raise ValueError("completed ray comparison has no Rust block probe")
        binary = _artifact(report_path, report["log_paths"]["rust_binary"])
        output = _artifact(report_path, report["log_paths"]["probe_output"])
        if block_run.sha256(binary.read_bytes()) != report.get("rust_binary_sha256"):
            raise ValueError("retained Rust binary hash mismatch")
        rust = output.read_bytes()
        if _artifact(report_path, processes["rust-run"]["stderr"]).read_bytes():
            raise ValueError("Rust block probe wrote unexpected stderr")
        if block_run.sha256(rust) != report.get("probe_output_sha256"):
            raise ValueError("retained Rust output hash mismatch")
        actual = block_run.compare_ray_outputs(fixture, rust)
        expected_status = "pass" if actual["mismatch_count"] == 0 else "fail"
        if status != expected_status or report.get("mismatch_count") != actual["mismatch_count"]:
            raise ValueError("stored ray result differs from retained probe output")
        if report.get("first_mismatch") != actual["first_mismatch"]:
            raise ValueError("stored first mismatch differs from retained probe output")
    elif status == "error":
        if not isinstance(report.get("reason"), str) or not report["reason"]:
            raise ValueError("error report has no reason")
    else:
        raise ValueError("unknown block report status")
    return {"status": status, "scope": "ray-only", "action_status": "not_implemented",
            "case_count": report.get("case_count", 0), "reference_sha256": report.get("reference_output_sha256"),
            "verified_processes": len(processes)}


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 tools/parity/block_review.py REPORT.json")
    try:
        print(json.dumps(verify_report(sys.argv[1]), indent=2))
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"invalid block report: {error}", file=sys.stderr)
        raise SystemExit(1)
