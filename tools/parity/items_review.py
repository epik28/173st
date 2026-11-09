"""Verify the retained native and Rust evidence for ITEMS-001."""
import json
from pathlib import Path
import sys

try:
    from tools.oracle import items_corpus
except ImportError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "oracle"))
    import items_corpus

try:
    from items_run import compare_items, normalize_stream, sha256
except ImportError:
    from tools.parity.items_run import compare_items, normalize_stream, sha256

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
    expected_root = (ROOT / "artifacts/parity/items").resolve()
    if not report_path.is_relative_to(expected_root) or report_path.name != "report.json":
        raise ValueError("report is outside the retained items artifact tree")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if report.get("schema_version") != 1 or report.get("suite") != "items":
        raise ValueError("unsupported items report schema")
    if report.get("scenario_id") != "ITEMS-001-main-inventory-native":
        raise ValueError("unexpected items scenario")
    fixture = (ROOT / "tests/fixtures/items/native.tsv").read_bytes()
    input_data = (ROOT / "tests/fixtures/items/input.tsv").read_bytes()
    audit = json.loads((ROOT / "tests/fixtures/items/audit.json").read_text(encoding="utf-8"))
    if report.get("reference_jar_sha256") != items_corpus.PINNED_JAR:
        raise ValueError("report is not bound to the pinned original client JAR")
    if report.get("expected_fixture_sha256") != sha256(fixture):
        raise ValueError("report expected fixture hash differs from committed C01 reference")
    if report.get("input_sha256") != sha256(input_data):
        raise ValueError("report input hash differs from frozen C01 suite")
    before, after = report.get("source_hashes"), report.get("source_hashes_after")
    if not isinstance(before, dict) or before != after:
        raise ValueError("tracked sources changed during recorded comparison")
    processes = report.get("processes")
    if not isinstance(processes, dict) or not processes:
        raise ValueError("report process audit missing")
    for name, process in processes.items():
        if process.get("returncode") != 0 or process.get("error") or process.get("timeout"):
            raise ValueError(f"{name} did not complete successfully")
        for stream in ("stdout", "stderr"):
            stream_path = _artifact(report_path, process.get(stream, ""))
            raw = stream_path.read_bytes()
            if sha256(raw) != process.get(stream + "_sha256"):
                raise ValueError(f"{name} {stream} hash mismatch")
    native, native_audits = [], []
    for name in ("native-run", "native-repeat"):
        if name not in processes:
            raise ValueError(f"{name} process audit is missing")
        stderr = Path(_artifact(report_path, processes[name]["stderr"])).read_bytes()
        if not stderr:
            raise ValueError(f"{name} native class/method audit is missing")
        raw = _artifact(report_path, report.get("log_paths", {}).get(name + "_output", "")).read_bytes()
        normalize_stream(raw, native=True)
        native.append(raw)
        saved_audit = _artifact(report_path, report.get("log_paths", {}).get(name + "_audit", ""))
        native_audit = json.loads(saved_audit.read_text(encoding="utf-8"))
        if json.loads(stderr.decode("utf-8")) != native_audit:
            raise ValueError(f"{name} stderr audit differs from retained audit copy")
        native_audits.append(native_audit)
    if native[0] != native[1] or native[0] != fixture:
        raise ValueError("retained native runs do not match each other and committed fixture")
    audit_keys = ("bootstrap", "bootstrap_stdout_sha256", "classes", "code_source",
                  "direct_calls", "jar_sha256", "native_methods", "rows")
    if (native_audits[0] != native_audits[1]
            or any(native_audits[0].get(key) != audit.get(key) for key in audit_keys)):
        raise ValueError("retained native class/method audit differs from committed C01 audit")
    summarized_audit = {key: native_audits[0][key] for key in
                        ("bootstrap", "bootstrap_stdout_sha256", "direct_calls", "native_methods", "rows")}
    summarized_audit["repeatable"] = True
    if report.get("native_audit") != summarized_audit:
        raise ValueError("report native audit summary differs from retained native calls")
    if report.get("reference_output_sha256") != sha256(native[0]):
        raise ValueError("native output digest mismatch")
    if report.get("reference_class_sha256") != audit.get("classes"):
        raise ValueError("native class hashes differ from C01 audit")
    if (audit.get("input_sha256") != sha256(input_data)
            or audit.get("output_sha256") != sha256(native[0])
            or audit.get("repeat_output_sha256") != sha256(native[1])):
        raise ValueError("C01 audit input/output hashes differ from retained captures")
    if report.get("case_count") != 6 or report.get("row_count") != 21:
        raise ValueError("native coverage counts differ from committed corpus")

    status = report.get("status")
    probe_present = (ROOT / "crates/beta-core/examples/items_probe.rs").is_file()
    if status == "not_implemented":
        if probe_present or report.get("rust_binary_sha256") is not None or report.get("rust_output_sha256") is not None:
            raise ValueError("not_implemented report contains Rust probe evidence")
    elif status in {"pass", "fail"}:
        if not probe_present:
            raise ValueError("Rust probe is absent from a completed comparison report")
        if (not isinstance(report.get("rust_revision"), str) or len(report["rust_revision"]) != 40
                or report.get("rust_dirty") is not False):
            raise ValueError("completed comparison is not bound to a clean committed Rust revision")
        binary = _artifact(report_path, report.get("log_paths", {}).get("rust_binary", ""))
        if sha256(binary.read_bytes()) != report.get("rust_binary_sha256"):
            raise ValueError("retained Rust binary digest mismatch")
        rust_path = _artifact(report_path, report.get("log_paths", {}).get("rust_output", ""))
        rust = rust_path.read_bytes()
        if sha256(rust) != report.get("rust_output_sha256"):
            raise ValueError("retained Rust output digest mismatch")
        comparison = compare_items(native[0], rust)
        expected_status = "pass" if comparison["mismatch_count"] == 0 else "fail"
        if status != expected_status or report.get("mismatch_count") != comparison["mismatch_count"]:
            raise ValueError("stored parity result differs from retained outputs")
        if report.get("first_mismatch") != comparison["first_mismatch"]:
            raise ValueError("stored first mismatch differs from retained outputs")
    elif status == "error":
        if not isinstance(report.get("reason"), str) or not report["reason"]:
            raise ValueError("error report has no reason")
    else:
        raise ValueError("unknown items report status")
    return {"status": status, "case_count": 6, "row_count": 21,
            "reference_sha256": sha256(native[0]), "verified_processes": len(processes)}


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 tools/parity/items_review.py REPORT.json")
    try:
        print(json.dumps(verify_report(sys.argv[1]), indent=2))
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"invalid items report: {error}", file=sys.stderr)
        raise SystemExit(1)
