#!/usr/bin/env python3
"""Verify a saved live report and detect one explicitly synthetic mutation."""

import argparse
import json
from pathlib import Path
import subprocess

from run import compare_outputs, expected_keys, parse_fixture, sha256


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("--protocol", action="store_true", help="also check live rejection of invalid TSV")
    args = parser.parse_args()
    report = json.loads(args.report.read_text())
    if report["status"] != "pass":
        raise SystemExit("requires a successful live report")
    input_data = Path(report["log_paths"]["input"]).read_bytes()
    reference = Path(report["log_paths"]["java-run"]["stdout"]).read_bytes()
    actual = Path(report["log_paths"]["rust-run"]["stdout"]).read_bytes()
    for data, key in ((input_data, "input_sha256"), (reference, "reference_output_sha256"),
                      (actual, "rust_output_sha256")):
        if sha256(data) != report[key]:
            raise SystemExit(f"saved bytes do not match {key}")
    keys = expected_keys(parse_fixture(input_data))
    if compare_outputs(reference, actual, keys)["mismatch_count"]:
        raise SystemExit("saved outputs no longer match")
    lines = actual.splitlines(keepends=True)
    fields = lines[0].decode("ascii").rstrip("\n").split("\t")
    value = int(fields[2])
    fields[2] = str(value + 1 if value < 2**31 - 1 else value - 1)
    lines[0] = ("\t".join(fields) + "\n").encode("ascii")
    mutated = b"".join(lines)
    result = compare_outputs(reference, mutated, keys)
    if result["mismatch_count"] != 1 or result["first_mismatch"]["record"] != 0:
        raise SystemExit("comparator did not detect the single changed value")
    directory = args.report.resolve().parent
    mutation_path = directory / "synthetic-mutated-rust.tsv"
    mutation_path.write_bytes(mutated)
    sensitivity = {"kind": "synthetic mutation of copied live output; not a live parity result",
                   "live_report": str(args.report.resolve()), "mutation_path": str(mutation_path),
                   "original_output_sha256": sha256(actual), "mutated_output_sha256": sha256(mutated),
                   "comparison": result}
    (directory / "comparator-sensitivity.json").write_text(
        json.dumps(sensitivity, ensure_ascii=False, indent=2) + "\n")
    print(f"PASS: saved hashes and outputs verified; one synthetic mutation detected: {directory}")
    if args.protocol:
        commands = {"java": ["java", "-cp", str(directory / "classes"), "RandomOracle"],
                    "rust": [str(directory / "rng_probe")]}
        invalid = [
            b" \n", b"\t\n", b"x\t9223372036854775808\t1\n",
            b"x\t-9223372036854775809\t1\n", b"x\tNaN\t1\n", b"x\t0\t0\n",
            b"x\t0\t-1\n", b"x\t0\t100001\n", b"x\t0\t1.5\n",
            b"x\t0\t999999999999\n", b"\t0\t1\n", b"bad id\t0\t1\n",
            "é\t0\t1\n".encode(), b"x\t0\t1\nx\t1\t1\n",
            b"x\t0\n", b"x\t0\t1\textra\n",
        ]
        results, failures = [], []
        for data, valid in [(value, False) for value in invalid] + [(b"\n", True), (b"# comment\n", True)]:
            for side, command in commands.items():
                process = subprocess.run(command, input=data, capture_output=True, timeout=10)
                passed = (process.returncode == 0) if valid else (process.returncode != 0 and bool(process.stderr))
                result = {"side": side, "input_hex": data.hex(), "expected_valid": valid,
                          "returncode": process.returncode, "stdout": process.stdout.decode("utf-8", "replace"),
                          "stderr": process.stderr.decode("utf-8", "replace"), "accepted": passed}
                results.append(result)
                if not passed:
                    failures.append(result)
        review = {"kind": "live TSV protocol validation; separate from RNG output comparison",
                  "live_report": str(args.report.resolve()), "results": results, "failures": failures}
        (directory / "protocol-review.json").write_text(json.dumps(review, ensure_ascii=False, indent=2) + "\n")
        if failures:
            for result in failures:
                print(f"FAIL: {result['side']} input hex={result['input_hex']}: exit {result['returncode']}")
            return 1
        print(f"PASS: live protocol checks ({len(results)} executions)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
