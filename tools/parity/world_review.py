"""Saved WORLD-LOAD-001 evidence and live TSV protocol review."""

import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from world_run import (CLASS_SHA256, JAR_SHA256, MARK_REGION, MARK_WORLD, PROBE,
                       _bind_retained_to_rows, _check_copy, _retained_manifest,
                       _validate_audit, compare_world_outputs, parse_world_fixture,
                       sha256, valid_world_record)


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def _read_log(process, side):
    result = process.get(side)
    require(isinstance(result, dict), f"required process missing: {side}")
    require(result.get("returncode") == 0 and not result.get("timeout") and not result.get("error"),
            f"required process failed: {side}")
    for suffix in ("stdout", "stderr"):
        try:
            path = Path(result[suffix]).resolve(strict=True)
            digest = sha256(path.read_bytes())
        except (KeyError, OSError, TypeError, ValueError):
            require(False, f"saved process log missing or invalid: {side}/{suffix}")
        require(digest == result.get(suffix + "_sha256"), f"saved process log hash mismatch: {side}/{suffix}")
    return result


def _copy_for_protocol(source, target):
    target.mkdir()
    (target / ".world-input-copy").write_text(MARK_WORLD)
    (target / ".region-input-copy").write_text(MARK_REGION)
    for item in Path(source).iterdir():
        if item.name.startswith("."):
            continue
        shutil.copyfile(item, target / item.name)


def _negative_zero_protocol_accepts(output):
    lines = output.splitlines()
    if len(lines) != 1:
        return False
    fields = lines[0].split(b"\t")
    return (len(fields) == 17 and fields[0:2] == [b"zero", b"chunk"]
            and fields[2] in (b"present", b"absent") and fields[3:5] == [b"0", b"0"])


def verify_world(report_path, report, protocol=False):
    report_path = Path(report_path).resolve(strict=True)
    directory = report_path.parent
    require(report.get("schema_version") == 1 and report.get("suite") == "world", "not a WORLD-LOAD-001 report")
    require(report.get("status") == "pass", "requires a successful live world report")
    processes = report.get("processes", {})
    required = ("version-java", "version-javac", "version-rustc", "version-cargo", "java-build",
                "java-run", "rust-build", "rust-run", "git-revision", "git-dirty")
    for key in required:
        _read_log(processes, key)
    try:
        expected_jar = Path(report["reference_jar_path"]).resolve(strict=True)
        jar_hash = sha256(expected_jar.read_bytes())
    except (KeyError, OSError, TypeError, ValueError):
        require(False, "configured original JAR is missing or invalid")
    require(jar_hash == JAR_SHA256 == report.get("reference_jar_sha256"),
            "configured original JAR hash differs from pin")
    require(report.get("runtime_versions", {}).get("java"), "Java runtime version missing")
    adapter = directory / "classes/WorldOracle.class"
    require(adapter.is_file() and not adapter.is_symlink()
            and sha256(adapter.read_bytes()) == report.get("adapter_class_sha256"),
            "saved WorldOracle class hash mismatch")
    binary = directory / "world_probe"
    require(binary.is_file() and not binary.is_symlink()
            and sha256(binary.read_bytes()) == report.get("rust_binary_sha256"),
            "saved world_probe binary hash mismatch")
    require(Path(report["processes"]["rust-build"]["executable"] if "executable" in report["processes"]["rust-build"] else str(binary)).exists(),
            "saved world probe build artifact missing")

    input_path = Path(report["log_paths"]["input"]).resolve(strict=True)
    fixture = input_path.read_bytes()
    require(sha256(fixture) == report.get("input_sha256"), "saved world fixture hash mismatch")
    cases = parse_world_fixture(fixture)
    require(len(cases) == report.get("case_count") == 749 and report.get("native_cases") == {"live": 749, "synthetic": 0},
            "live and synthetic row counts are mixed or invalid")
    reference = Path(processes["java-run"]["stdout"]).read_bytes()
    actual = Path(processes["rust-run"]["stdout"]).read_bytes()
    require(sha256(reference) == report.get("reference_output_sha256"), "saved original output hash mismatch")
    require(sha256(actual) == report.get("rust_output_sha256"), "saved Rust output hash mismatch")
    comparison = compare_world_outputs(reference, actual, cases)
    require(comparison["mismatch_count"] == 0 and comparison == {"mismatch_count": report.get("mismatch_count"),
            "first_mismatch": report.get("first_mismatch")}, "saved live outputs no longer match exactly")

    audit_path = Path(report["log_paths"]["reference_invocations"]).resolve(strict=True)
    audit = json.loads(audit_path.read_text())
    require(audit == report.get("reference_invocations"), "native audit differs from saved report")
    source_hashes = report.get("source_world_hashes", {})
    _validate_audit(audit, source_hashes, len(cases))
    require(report.get("native_calls") == audit["native_calls"], "native call summary differs from audit")
    require(report.get("reference_class_sha256") == CLASS_SHA256, "class pins differ from report")
    require(report.get("native_semantics") == {"metadata_constructor": "ei(nu)",
            "nbt_extraction": "ij.b(DataInput) + nu fields", "region_read": "qj.a(int,int)",
            "full_world_load": False, "native_chunk_loads": 0}, "native scope is misstated")
    require(report.get("source_world_hashes_after") == source_hashes and len(source_hashes) == 5,
            "world source hashes missing or changed")

    root = Path(__file__).resolve().parents[2]
    corpus_manifest = json.loads((root / "tests/fixtures/world/sources.json").read_text())
    source_world = Path(report["source_world_path"]).resolve(strict=True)
    for name, info in corpus_manifest["files"].items():
        source = source_world / name if name == "level.dat" else source_world / "region" / name
        require(source.is_file() and not source.is_symlink() and len(source.read_bytes()) == info["size"]
                and sha256(source.read_bytes()) == source_hashes.get(name), f"original source changed: {name}")
    require(report.get("source_hashes") == report.get("source_hashes_after"), "runner source snapshot changed")
    for name, digest in report["source_hashes"].items():
        path = root / name
        require((not path.exists()) if digest is None else (path.is_file() and sha256(path.read_bytes()) == digest),
                f"runner source snapshot does not match current file: {name}")

    copy_hashes = report.get("input_copy_hashes", {})
    require(copy_hashes == source_hashes == report.get("input_copy_hashes_after"), "native input copy hashes mismatch")
    _check_copy(report["log_paths"]["input_copy_dir"], copy_hashes)
    _check_copy(report["log_paths"]["rust_input_copy_dir"], copy_hashes)
    require(report.get("rust_input_copy_hashes") == copy_hashes, "Rust copy evidence mismatch")
    native_dir = Path(report["log_paths"]["payload_dir"]).resolve(strict=True)
    rust_dir = Path(report["log_paths"]["rust_payload_dir"]).resolve(strict=True)
    native_manifest = _retained_manifest(native_dir, audit)
    rust_manifest = _retained_manifest(rust_dir, {})
    require(native_manifest == report.get("retained_files") == rust_manifest,
            "retained files do not match actual preserved payload bytes")
    _bind_retained_to_rows(native_dir, native_manifest, reference, cases)
    _bind_retained_to_rows(rust_dir, rust_manifest, actual, cases)
    require(report.get("rust_dirty") is False, "successful parity report was produced from a dirty source tree")

    # Prove the comparator rejects scalar, array-hash and raw-payload mutations independently.
    _sensitivity_checks(directory, cases, reference, actual, native_dir, audit)
    if protocol:
        _verify_protocol(directory, report, cases)
    print(f"PASS: saved source/JAR/class/binary hashes, native audit, {len(cases)} ordered rows and retained bytes verified")


def _sensitivity_checks(directory, cases, reference, actual, payload_dir, audit):
    lines = actual.splitlines(keepends=True)
    results = []
    # Mutation 1: a scalar value in the level row.
    first = bytearray(lines[0])
    fields = first[:-1].split(b"\t")
    fields[2] = str(int(fields[2]) + 1).encode()
    scalar = b"\t".join(fields) + b"\n"
    results.append(("scalar", b"".join([scalar, *lines[1:]])))
    # Mutation 2: block array digest remains syntactically valid but changes.
    index = next(i for i, line in enumerate(lines) if line.split(b"\t")[2] == b"present")
    fields = lines[index][:-1].split(b"\t")
    fields[7] = (b"0" if fields[7][0:1] != b"0" else b"1") + fields[7][1:]
    changed = list(lines); changed[index] = b"\t".join(fields) + b"\n"
    results.append(("array-hash", b"".join(changed)))
    expected = compare_world_outputs(reference, actual, cases)
    review = []
    for label, candidate in results:
        comparison = compare_world_outputs(reference, candidate, cases)
        require(comparison["mismatch_count"] > expected["mismatch_count"], f"{label} mutation escaped comparator")
        review.append({"kind": f"synthetic {label} mutation; separate from live parity", "detected": True,
                       "mutated_sha256": sha256(candidate), "comparison": comparison})
    payload_name = sorted(p.name for p in payload_dir.glob("*.nbt"))[0]
    raw = (payload_dir / payload_name).read_bytes()
    altered = (raw[:-1] + bytes([raw[-1] ^ 1])) if raw else b"\x00"
    payload_id = payload_name[:-4]
    payload_case = next(case for case in cases if case["id"] == payload_id)
    row_for_payload = next(line for line in reference.splitlines(keepends=True)
                           if line.startswith(payload_id.encode() + b"\t"))
    altered_path = directory / "synthetic-mutated-payload.nbt"
    altered_path.write_bytes(altered)
    payload_detected = False
    with tempfile.TemporaryDirectory(prefix="payload-mutation-", dir=directory) as tmp:
        changed_dir = Path(tmp)
        (changed_dir / payload_name).write_bytes(altered)
        altered_manifest = {payload_name: {"sha256": sha256(altered), "length": len(altered)}}
        try:
            _bind_retained_to_rows(changed_dir, altered_manifest, row_for_payload, [payload_case])
        except WorldRunError:
            payload_detected = True
    require(payload_detected, "retained payload mutation escaped raw-byte verifier")
    mutated_audit = json.loads(json.dumps(audit))
    mutated_audit["native_calls"]["qj_a_int_int"] -= 1
    audit_detected = False
    try:
        _validate_audit(mutated_audit, mutated_audit.get("input_sha256"), mutated_audit.get("live_rows"))
    except WorldRunError:
        audit_detected = True
    require(audit_detected, "native audit mutation escaped pinned-call verifier")
    review.extend([{"kind": "synthetic raw payload byte mutation; separate from live parity", "detected": True,
                    "payload": str(altered_path), "original_sha256": sha256(raw), "mutated_sha256": sha256(altered)},
                   {"kind": "synthetic native audit call-count mutation; separate from live parity", "detected": True}])
    (directory / "mutation-review.json").write_text(json.dumps(review, indent=2) + "\n")


def _verify_protocol(directory, report, cases):
    protocol_dir = Path(tempfile.mkdtemp(prefix="protocol-", dir=directory))
    source = Path(report["log_paths"]["input_copy_dir"])
    java_run = report["processes"]["java-run"]
    java_command = java_run["command"][:]
    classpath = java_command[java_command.index("-cp") + 1]
    jar = report["reference_jar_path"]
    java_base = ["java", "-XX:-UsePerfData", "-Xmx256m", "-cp", classpath, "WorldOracle", jar]
    binary = Path(directory) / "world_probe"
    # Parser errors exercise duplicate IDs, extra fields, whitespace and safe coordinates.
    invalid = [b" \n", b"id\tchunk\tr.0.0.mcr\t-0\t0\textra\n",
               b"bad id\tchunk\tr.0.0.mcr\t0\t0\n", b"same\tchunk\tr.0.0.mcr\t0\t0\nsame\tchunk\tr.0.0.mcr\t1\t0\n",
               b"x\tchunk\t../r.0.0.mcr\t0\t0\n", b"x\tchunk\tr.1.0.mcr\t0\t0\n",
               b"x\tchunk\tr.0.0.mcr\t2147483648\t0\n", b"x\tlevel\tlevel.dat\t0\t0\n",
               b"x\tunknown\tlevel.dat\t-\t-\n"]
    valid_control = b"# empty input is valid\n"
    results, failures = [], []
    for index, data in enumerate(invalid + [valid_control]):
        expect_valid = index == len(invalid)
        for side, command in (("java", java_base), ("rust", [str(binary)])):
            run_dir = protocol_dir / f"case-{index}-{side}"
            run_dir.mkdir()
            copied = run_dir / "input"
            _copy_for_protocol(source, copied)
            retained = run_dir / "retained"  # Deliberately absent: WorldOracle CREATE_NEW contract.
            args = command + ([str(copied), str(retained), str(run_dir / "audit.json")] if side == "java"
                              else [str(copied), str(retained)])
            try:
                process = subprocess.run(args, input=data, capture_output=True, timeout=20, check=False)
                accepted = process.returncode == 0 if expect_valid else process.returncode != 0 and bool(process.stderr)
                row = {"side": side, "input_hex": data.hex(), "returncode": process.returncode,
                       "stdout_sha256": sha256(process.stdout), "stderr_sha256": sha256(process.stderr),
                       "expected_valid": expect_valid, "accepted": accepted,
                       "stderr": process.stderr.decode("utf-8", "replace")[-800:]}
            except subprocess.TimeoutExpired as error:
                row = {"side": side, "input_hex": data.hex(), "timeout": True, "expected_valid": expect_valid,
                       "accepted": False, "stdout_sha256": sha256(error.stdout or b""),
                       "stderr_sha256": sha256(error.stderr or b"")}
            results.append(row)
            if not accepted:
                failures.append(row)
    # Canonical -0 must remain accepted and print canonical integer coordinate 0.
    for side, command in (("java", java_base), ("rust", [str(binary)])):
        run_dir = protocol_dir / f"negative-zero-{side}"; run_dir.mkdir()
        copied = run_dir / "input"; _copy_for_protocol(source, copied)
        retained = run_dir / "retained"
        data = b"zero\tchunk\tr.0.0.mcr\t-0\t0\n"
        args = command + ([str(copied), str(retained), str(run_dir / "audit.json")] if side == "java"
                          else [str(copied), str(retained)])
        try:
            process = subprocess.run(args, input=data, capture_output=True, timeout=30, check=False)
            accepted = process.returncode == 0 and _negative_zero_protocol_accepts(process.stdout)
            row = {"side": side, "input_hex": data.hex(), "returncode": process.returncode,
                   "stdout_sha256": sha256(process.stdout), "stderr_sha256": sha256(process.stderr),
                   "expected_valid": True, "accepted": accepted}
        except subprocess.TimeoutExpired as error:
            row = {"side": side, "input_hex": data.hex(), "timeout": True, "expected_valid": True, "accepted": False}
        results.append(row)
        if not accepted:
            failures.append(row)
    # An otherwise valid Java oracle must not make a missing Rust binary a false protocol pass.
    if failures:
        (directory / "protocol-review.json").write_text(json.dumps({"kind": "synthetic world TSV protocol executions; separate from live cases",
                "results": results, "failures": failures}, indent=2) + "\n")
        raise SystemExit(f"world TSV protocol checks failed: {len(failures)}")
    (directory / "protocol-review.json").write_text(json.dumps({"kind": "synthetic world TSV protocol executions; separate from live cases",
            "results": results, "failures": []}, indent=2) + "\n")
    print(f"PASS: {len(results)} Java/Rust world protocol executions")
