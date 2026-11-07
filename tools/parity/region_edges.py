#!/usr/bin/env python3
"""Execute separately labelled synthetic McRegion cases against both readers."""

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile

from run import REGION_JAR_SHA256, sha256


ROOT = Path(__file__).resolve().parents[2]


def review(report_path):
    report_path = Path(report_path).resolve(strict=True)
    report = json.loads(report_path.read_text())
    if report.get("suite") != "region" or report.get("status") != "pass":
        raise ValueError("requires a successful REGION-001 report")
    directory = report_path.parent
    jar = Path(report["reference_jar_path"])
    binary = directory / "region_probe"
    if (sha256(jar.read_bytes()) != REGION_JAR_SHA256
            or sha256(binary.read_bytes()) != report["rust_binary_sha256"]):
        raise ValueError("saved binary/original JAR hash mismatch")
    spec = importlib.util.spec_from_file_location("region_edge_corpus", ROOT / "tools/oracle/region_corpus.py")
    corpus = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(corpus)
    commands = {
        "java": ["java", "-XX:-UsePerfData", "-Xmx256m", "-cp",
                 f"{directory / 'classes'}:{jar}", "RegionOracle", str(jar)],
        "rust": [str(binary)],
    }
    cases = []
    for name, (contents, valid) in corpus.synthetic_cases().items():
        cases.append((name, "r.0.0.mcr", 0, contents, valid,
                      name in {"oversized-length", "overlap"}))
    for name, filename, coord, contents in corpus.coordinate_cases():
        cases.append((name, filename, coord, contents, True, False))

    # Native GZIPInputStream ignores malformed following headers, but not
    # corruption after a complete header. These cases were measured on qj.
    import gzip
    base = corpus.synthetic_cases()["gzip"][0]
    size = int.from_bytes(base[8192:8196], "big")
    first = base[8197:8196 + size]
    following = gzip.compress(b"next member", mtime=0)
    for name, tail, valid in (
        ("gzip-tail-lone-magic", b"\x1f", True),
        ("gzip-tail-partial-header", b"\x1f\x8b", True),
        ("gzip-next-truncated-trailer", following[:-2], False),
        ("gzip-next-corrupt-trailer", following[:-1] + bytes([following[-1] ^ 1]), False),
    ):
        compressed = first + tail
        contents = bytearray(base)
        contents[8192:8196] = (len(compressed) + 1).to_bytes(4, "big")
        contents[8197:8197 + len(compressed)] = compressed
        cases.append((name, "r.0.0.mcr", 0, bytes(contents), valid, False))

    output = Path(tempfile.mkdtemp(prefix="edge-review-", dir=directory))
    results = []
    for name, filename, coord, contents, valid, native_difference in cases:
        row = f"{name}\t{filename}\t{coord}\t{coord}\n".encode("ascii")
        result = {"name": name, "kind": "synthetic; not live world parity",
                  "input_sha256": sha256(contents), "safe_contract_valid": valid,
                  "native_difference": native_difference, "sides": {}}
        stdout, payloads = {}, {}
        for side, base_command in commands.items():
            case = output / name / side
            inputs = case / "input"
            inputs.mkdir(parents=True)
            (inputs / ".region-input-copy").write_text(corpus.MARKER)
            path = inputs / filename
            path.write_bytes(contents)
            (case / "stdin.tsv").write_bytes(row)
            payload_dir = case / "payloads"
            command = base_command + [str(inputs), str(payload_dir)]
            if side == "java":
                command.append(str(case / "native-audit.json"))
            expected_success = valid or (side == "java" and native_difference)
            try:
                process = subprocess.run(command, input=row, capture_output=True, timeout=30, cwd=ROOT)
                (case / "stdout.tsv").write_bytes(process.stdout)
                (case / "stderr.log").write_bytes(process.stderr)
                unchanged = sha256(path.read_bytes()) == result["input_sha256"]
                accepted = (process.returncode == 0 if expected_success
                            else process.returncode != 0 and bool(process.stderr))
                result["sides"][side] = {"returncode": process.returncode,
                    "expected_success": expected_success, "copy_unchanged": unchanged,
                    "accepted": bool(accepted and unchanged), "command": command}
                stdout[side] = process.stdout
                payloads[side] = {p.name: p.read_bytes() for p in payload_dir.glob("*.nbt")}
            except (OSError, subprocess.TimeoutExpired) as error:
                result["sides"][side] = {"accepted": False, "error": str(error)}
        result["accepted"] = all(side["accepted"] for side in result["sides"].values())
        if valid and result["accepted"]:
            result["exact_outputs"] = stdout["java"] == stdout["rust"]
            result["exact_payloads"] = payloads["java"] == payloads["rust"]
            result["accepted"] = result["exact_outputs"] and result["exact_payloads"]
        results.append(result)
    failures = [item["name"] for item in results if not item["accepted"]]
    evidence = {"kind": "generated wire/protocol cases; separate from live world cases",
                "live_report": str(report_path), "cases": results, "failures": failures,
                "runtime_java": report["runtime_versions"]["java"]}
    (output / "review.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(f"{'FAIL' if failures else 'PASS'}: {len(results)} synthetic cases / "
          f"{2 * len(results)} reader executions; {output / 'review.json'}")
    return 1 if failures else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    raise SystemExit(review(parser.parse_args().report))
