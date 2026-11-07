"""Saved REGION-001 evidence and protocol review; synthetic checks stay separate."""

import hashlib
import json
from pathlib import Path
import subprocess

from run import (REGION_JAR_SHA256, REGION_MARKER, compare_region_outputs,
                 parse_region_fixture, region_keys, safe_region_file, sha256)


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def verify_region(report_path, report, protocol=False):
    report_path = Path(report_path).resolve(strict=True)
    directory = report_path.parent
    require(report.get("schema_version") == 1 and report.get("suite") == "region", "not a REGION-001 report")
    require(report.get("status") == "pass", "requires a successful live region report")
    processes = report.get("processes", {})
    for label in ("version-java", "version-javac", "version-rustc", "version-cargo",
                  "java-build", "java-run", "rust-build", "rust-run", "git-revision", "git-dirty"):
        process = processes.get(label)
        require(isinstance(process, dict) and process.get("returncode") == 0
                and not process.get("timeout") and not process.get("error"),
                f"required process missing or unsuccessful: {label}")
        require(sha256(Path(process["stdout"]).read_bytes()) == process.get("stdout_sha256")
                and sha256(Path(process["stderr"]).read_bytes()) == process.get("stderr_sha256"),
                f"saved process logs changed: {label}")

    jar = Path(report["reference_jar_path"]).resolve(strict=True)
    require(sha256(jar.read_bytes()) == REGION_JAR_SHA256 == report.get("reference_jar_sha256"),
            "original JAR hash does not match pinned client")
    adapter = directory / "classes/RegionOracle.class"
    require(adapter.is_file() and sha256(adapter.read_bytes()) == report.get("adapter_class_sha256"),
            "saved RegionOracle class hash mismatch")

    input_path = Path(report["log_paths"]["input"]).resolve(strict=True)
    fixture = input_path.read_bytes()
    require(sha256(fixture) == report.get("input_sha256"), "saved coordinate fixture hash mismatch")
    cases = parse_region_fixture(fixture)
    require(len(cases) == report.get("case_count") and len(cases) > 0, "case count mismatch")
    require(report.get("native_cases") == {"live": len(cases), "synthetic": 0},
            "live and synthetic counts are mixed or invalid")
    reference_path = Path(report["log_paths"]["java-run"]["stdout"]).resolve(strict=True)
    rust_path = Path(report["log_paths"]["rust-run"]["stdout"]).resolve(strict=True)
    reference, actual = reference_path.read_bytes(), rust_path.read_bytes()
    require(sha256(reference) == report.get("reference_output_sha256"), "saved Java output hash mismatch")
    require(sha256(actual) == report.get("rust_output_sha256"), "saved Rust output hash mismatch")
    comparison = compare_region_outputs(reference, actual, cases)
    require(comparison["mismatch_count"] == 0 and comparison == {
        "mismatch_count": report.get("mismatch_count"), "first_mismatch": report.get("first_mismatch")},
        "saved live outputs no longer compare exactly")

    audit_path = Path(report["log_paths"]["reference_invocations"]).resolve(strict=True)
    audit = json.loads(audit_path.read_text())
    require(audit == report.get("reference_invocations"), "native invocation audit differs from report")
    require(audit.get("class") == "qj" and audit.get("method") == "a(int,int)"
            and audit.get("class_sha256") == report.get("reference_class_sha256")
            and audit.get("jar_sha256") == REGION_JAR_SHA256
            and audit.get("native_reads") == len(cases) and audit.get("present") == 704
            and audit.get("absent") == len(cases)-704,
            "native invocation evidence is incomplete or inconsistent")
    stderr_path = Path(report["log_paths"]["java-run"]["stderr"]).resolve(strict=True)
    require(json.loads(stderr_path.read_text()) == audit, "native stderr audit differs")

    world = Path(report["source_world_path"]).resolve(strict=True)
    expected_sources = report.get("source_world_hashes", {})
    require(report.get("source_world_hashes_after") == expected_sources and len(expected_sources) == 4,
            "source world before/after hashes differ")
    for name, digest in expected_sources.items():
        require(Path(name).name == name and sha256((world/name).read_bytes()) == digest,
                f"source world hash mismatch: {name}")
    require(report.get("source_hashes") == report.get("source_hashes_after"),
            "source inputs changed during runner")
    for name, digest in report["source_hashes"].items():
        source = Path(__file__).resolve().parents[2] / name
        if digest is None:
            require(not source.exists(), f"unexpected source appeared: {name}")
        else:
            require(source.is_file() and sha256(source.read_bytes()) == digest,
                    f"source snapshot differs from current tree: {name}")

    copy_before, copy_after = report.get("input_copy_hashes", {}), report.get("input_copy_hashes_after", {})
    require(copy_before == copy_after == expected_sources, "native input copy hashes differ from pinned sources")
    payload_dir = Path(report["log_paths"]["payload_dir"]).resolve(strict=True)
    rust_payload_dir = Path(report["log_paths"]["rust_payload_dir"]).resolve(strict=True)
    payload_manifest = report.get("payload_files", {})
    present_cases = [case for case, line in zip(cases, reference.splitlines(keepends=True))
                     if line.split(b"\t", 2)[1] == b"present"]
    expected_names = {case["id"] + ".nbt" for case in present_cases}
    require(set(payload_manifest) == expected_names, "payload manifest does not cover present cases exactly")
    require({path.name for path in payload_dir.iterdir()} == expected_names
            and {path.name for path in rust_payload_dir.iterdir()} == expected_names,
            "payload output files differ from present cases")
    for name, info in payload_manifest.items():
        native = (payload_dir/name).read_bytes()
        rust = (rust_payload_dir/name).read_bytes()
        require(sha256(native) == info.get("sha256") and len(native) == info.get("length"),
                f"saved native payload mismatch: {name}")
        require(sha256(rust) == info.get("rust_sha256") == info.get("sha256")
                and len(rust) == info.get("rust_length") == info.get("length"),
                f"saved Rust payload mismatch: {name}")
    require(report.get("rust_dirty") is False, "live report was produced from a dirty tree")

    # A one-byte change in a copy must invalidate the saved payload evidence.
    selected = next(iter(sorted(expected_names)))
    original = (payload_dir/selected).read_bytes()
    mutated = original[:-1] + bytes([original[-1] ^ 1]) if original else b"\x00"
    mutated_result = {"name": selected, "kind": "synthetic one-byte payload mutation; not live parity",
                      "original_sha256": sha256(original), "mutated_sha256": sha256(mutated),
                      "detected": sha256(mutated) != payload_manifest[selected]["sha256"]}
    require(mutated_result["detected"], "payload mutation was not detected")
    (directory/"synthetic-payload-mutation.json").write_text(json.dumps(mutated_result, indent=2)+"\n")

    if protocol:
        _verify_protocol(directory, report, cases)
    print(f"PASS: saved region hashes, source/copy integrity, native audit, {len(cases)} ordered rows and payload bytes verified")


def _verify_protocol(directory, report, cases):
    java = Path(report["processes"]["java-run"]["command"][0])
    classpath = report["processes"]["java-run"]["command"][4]
    jar = Path(report["reference_jar_path"])
    binary = directory / "region_probe"
    java_base = [str(java), "-XX:-UsePerfData", "-Xmx256m", "-cp", str(classpath), "RegionOracle", str(jar)]
    invalid = [b" \n", b"x\tr.0.0.mcr\t0\n", b"x\t../r.0.0.mcr\t0\t0\n",
               b"x\tr.0.0.mcr\t-0\t0\n", b"x\tr.1.0.mcr\t0\t0\n",
               b"bad id\tr.0.0.mcr\t0\t0\n", b"x\tr.0.0.mcr\t2147483648\t0\n",
               b"x\tr.0.0.mcr\t0\t0\nx\tr.0.0.mcr\t0\t0\n"]
    result_rows, failures = [], []
    for index, data in enumerate(invalid):
        for side, command in (("java", java_base), ("rust", [str(binary)])):
            run_dir = directory / "protocol" / f"invalid-{index}-{side}"
            inp, out = run_dir / "input", run_dir / "payloads"
            inp.mkdir(parents=True, exist_ok=True)
            out.mkdir()
            (inp/".region-input-copy").write_text(REGION_MARKER)
            for name, digest in report["input_copy_hashes"].items():
                # Use the run's preserved copy; never reach the configured source world.
                preserved = directory / "input" / name
                require(preserved.is_file() and sha256(preserved.read_bytes()) == digest,
                        "preserved region input copy missing for protocol review")
                (inp/name).write_bytes(preserved.read_bytes())
            command = command + ([str(inp), str(out), str(run_dir/"audit.json")] if side == "java" else [str(inp), str(out)])
            process = subprocess.run(command, input=data, capture_output=True, timeout=20)
            accepted = process.returncode != 0 and bool(process.stderr)
            row = {"side": side, "input_hex": data.hex(), "returncode": process.returncode,
                   "stderr": process.stderr.decode("utf-8", "replace"), "accepted": accepted}
            result_rows.append(row)
            if not accepted:
                failures.append(row)
    # Missing marker and symlink/path escape checks are executed only on copied synthetic directories.
    if failures:
        (directory/"protocol-review.json").write_text(json.dumps({"kind": "synthetic protocol rejection checks",
            "results": result_rows, "failures": failures}, indent=2)+"\n")
        raise SystemExit(f"region TSV protocol rejection failures: {len(failures)}")
    (directory/"protocol-review.json").write_text(json.dumps({"kind": "synthetic TSV protocol rejection checks; separate from live cases",
        "results": result_rows, "failures": []}, indent=2)+"\n")
    print(f"PASS: {len(result_rows)} live-adapter protocol rejection executions")
