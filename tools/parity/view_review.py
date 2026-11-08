"""Verify saved VIEW-001 native evidence and exact-output sensitivity."""

import json
from pathlib import Path
import sys
import subprocess
import tempfile

from view_run import JAR_SHA256, TERRAIN_SHA256, compare_view_outputs, sha256


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def verify_view(report_path, report, protocol=False):
    report_path = Path(report_path).resolve(strict=True)
    directory = report_path.parent
    require(report.get("schema_version") == 1 and report.get("suite") == "view", "not a VIEW-001 report")
    require(report.get("status") in {"pass", "not_implemented"}, "saved view run is not successful or not_implemented")
    require(report.get("reference_jar_sha256") == JAR_SHA256, "pinned client JAR hash differs")
    jar = Path(report["reference_jar_path"]).resolve(strict=True)
    require(sha256(jar.read_bytes()) == JAR_SHA256, "client JAR bytes changed")
    terrain = Path(report["terrain_path"]).resolve(strict=True)
    require(report.get("terrain_sha256") == TERRAIN_SHA256 and sha256(terrain.read_bytes()) == TERRAIN_SHA256,
            "terrain atlas bytes differ from pin")

    source_before = report.get("source_hashes", {})
    source_after = report.get("source_hashes_after", {})
    require(bool(source_before) and source_before == source_after, "tracked source inputs changed or are missing")
    for name, expected in source_before.items():
        path = Path(report["repository_root"]) / name
        actual = sha256(path.read_bytes()) if path.is_file() else None
        require(actual == expected, f"saved source hash changed: {name}")

    scene_path = Path(report["log_paths"]["input"]).resolve(strict=True)
    scene = scene_path.read_bytes()
    expected_path = Path(report["log_paths"]["expected_output"]).resolve(strict=True)
    expected = expected_path.read_bytes()
    require(sha256(scene) == report.get("scene_sha256"), "saved coordinate scene hash mismatch")
    require(sha256(expected) == report.get("expected_fixture_sha256"), "expected native fixture hash mismatch")
    require(report.get("case_count") == 25 and report.get("native_cases", {}).get("synthetic") == 0,
            "fixed live scene slot count or synthetic separation invalid")

    for label in ("reference_stdout", "repeat_stdout"):
        path = Path(report["log_paths"][label]).resolve(strict=True)
        data = path.read_bytes()
        comparison = compare_view_outputs(data, data)
        require(comparison["mismatch_count"] == 0, f"saved {label} violates native TSV protocol")
    reference_path = Path(report["log_paths"]["reference_stdout"]).resolve(strict=True)
    repeat_path = Path(report["log_paths"]["repeat_stdout"]).resolve(strict=True)
    reference, repeat = reference_path.read_bytes(), repeat_path.read_bytes()
    require(reference == repeat == expected and sha256(reference) == report.get("reference_output_sha256"),
            "repeated native output differs or hash mismatches")
    audit_path = Path(report["log_paths"]["reference_audit"]).resolve(strict=True)
    repeat_audit_path = Path(report["log_paths"]["repeat_audit"]).resolve(strict=True)
    audit = json.loads(audit_path.read_text())
    require(audit == report.get("native_audit") == json.loads(repeat_audit_path.read_text()),
            "native call audit mismatch")
    require(audit.get("jar_sha256") == JAR_SHA256 and audit.get("terrain_sha256") == TERRAIN_SHA256
            and audit.get("native_renderer_vertex_calls") == "0"
            and audit.get("native_world_or_chunk_loads") == "0",
            "native audit does not prove pinned mapping-only scope")
    adapter = directory / "native-run/classes/ViewOracle.class"
    require(adapter.is_file() and sha256(adapter.read_bytes()) == report.get("adapter_class_sha256"),
            "saved ViewOracle class hash mismatch")

    mapping_rows = sum(line.startswith(b"state\t") for line in reference.splitlines())
    neighbor_rows = sum(line.startswith(b"neighbor\t") for line in reference.splitlines())
    require(mapping_rows == report.get("mapping_case_count") == int(audit["mapping_cases"])
            and neighbor_rows == report.get("neighbor_pair_count") == int(audit["unique_neighbor_pairs"]),
            "mapping/neighbor counts disagree")
    require(report.get("source_world_hashes") == report.get("source_world_hashes_after")
            == report.get("input_copy_hashes") == report.get("input_copy_hashes_after"),
            "source world or copied inputs changed")
    input_dir = Path(report["log_paths"]["input_copy_dir"]).resolve(strict=True)
    require((input_dir / ".world-input-copy").read_text() == "WORLD-LOAD-001 copied inputs\n"
            and (input_dir / ".region-input-copy").read_text() == "REGION-001 copied inputs\n",
            "copied world markers missing")
    for name, expected in report["input_copy_hashes"].items():
        item = input_dir / name
        require(not item.is_symlink() and item.is_file() and sha256(item.read_bytes()) == expected,
                f"copied world input hash mismatch: {name}")

    if report["status"] == "not_implemented":
        require(not (directory / "view_probe").exists() and report.get("rust_output_sha256") is None,
                "not_implemented report unexpectedly contains a Rust probe result")
    else:
        require(report.get("rust_dirty") is False, "passing report must use a clean Rust tree")
        rust_path = Path(report["log_paths"]["rust_stdout"]).resolve(strict=True)
        actual = rust_path.read_bytes()
        require(sha256(actual) == report.get("rust_output_sha256"), "saved Rust output hash mismatch")
        comparison = compare_view_outputs(reference, actual)
        require(comparison == {"mismatch_count": report.get("mismatch_count"),
                               "first_mismatch": report.get("first_mismatch")}
                and comparison["mismatch_count"] == 0, "saved Rust output no longer matches native output")

    mutated = bytearray(reference)
    position = reference.find(b"state\t")
    require(position >= 0, "native mapping corpus has no state row")
    end = reference.find(b"\n", position)
    mutated[end - 1] = ord("0") if mutated[end - 1] != ord("0") else ord("1")
    mutation = compare_view_outputs(reference, bytes(mutated))
    require(mutation["mismatch_count"] != 0, "single-field output mutation escaped exact comparator")
    (directory / "synthetic-mutation-review.json").write_text(json.dumps({
        "kind": "synthetic single-byte mapping mutation; excluded from live parity",
        "detected": True, "original_sha256": sha256(reference),
        "mutated_sha256": sha256(bytes(mutated)), "comparison": mutation}, indent=2) + "\n")
    if protocol:
        try:
            from tools.oracle import view_corpus
        except ModuleNotFoundError:
            oracle_dir = str(Path(report["repository_root"]) / "tools/oracle")
            if oracle_dir not in sys.path:
                sys.path.insert(0, oracle_dir)
            import view_corpus
        invalid = [b"", b"#VIEW_NATIVE\t1\t0\n#states\n#neighbors\n",
                   b"#VIEW_NATIVE\t1\t-9\t64\t-5\t-3\t-3\n#states\nstate\t1\n#neighbors\n",
                   reference.replace(b"#neighbors\n", b"#neighbors\n" + next(
                       line + b"\n" for line in reference.splitlines() if line.startswith(b"state\t")))]
        rejected = 0
        for candidate in invalid:
            try:
                view_corpus.parse_stdout(candidate)
            except (UnicodeError, ValueError):
                rejected += 1
        require(rejected == len(invalid), "malformed/duplicate native TSV protocol accepted")
        (directory / "protocol-review.json").write_text(json.dumps({
            "invalid_cases": len(invalid), "rejected": rejected,
            "kind": "synthetic malformed/duplicate protocol inputs; separate from live corpus"}, indent=2) + "\n")
        classes = directory / "native-run/classes"
        classpath = f"{classes}:{jar}"
        java = ["java", "-XX:-UsePerfData", "-Xmx128m", "-cp", classpath, "ViewOracle"]
        error_cases = []
        with tempfile.TemporaryDirectory(prefix="view-invalid-", dir=directory) as temporary:
            scratch = Path(temporary)
            fake_jar = scratch / "wrong.jar"
            fake_jar.write_bytes(b"not the pinned game jar")
            fake_terrain = scratch / "wrong.png"
            fake_terrain.write_bytes(b"not a png")
            invocations = [
                ("malformed-args", java, False),
                ("missing-jar", java + [str(scratch / "missing.jar"), str(input_dir), str(terrain)], False),
                ("wrong-jar", java + [str(fake_jar), str(input_dir), str(terrain)], False),
                ("wrong-terrain", java + [str(jar), str(input_dir), str(fake_terrain)], False),
            ]
            for label, command, _ in invocations:
                process = subprocess.run(command, capture_output=True, timeout=20, check=False)
                accepted = process.returncode != 0 and bool(process.stderr)
                error_cases.append({"case": label, "returncode": process.returncode,
                                    "accepted": accepted, "stderr": process.stderr.decode("utf-8", "replace")[-500:]})
                require(accepted, f"native wrapper accepted invalid {label}")
        try:
            subprocess.run(["python3", "-c", "import time; time.sleep(0.1)"],
                           capture_output=True, timeout=0.001, check=False)
            timeout_detected = False
        except subprocess.TimeoutExpired:
            timeout_detected = True
        require(timeout_detected, "synthetic subprocess timeout was not reported")
        error_cases.append({"case": "synthetic-timeout", "accepted": timeout_detected,
                            "note": "generic subprocess timeout control; distinct from native live run"})
        (directory / "error-review.json").write_text(json.dumps(error_cases, ensure_ascii=False, indent=2) + "\n")
    print(f"PASS: saved native view report, scene/JAR/atlas/copy hashes and {mapping_rows} mapping cases verified")
