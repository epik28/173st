#!/usr/bin/env python3
"""Verify a saved live report and detect one explicitly synthetic mutation."""

import argparse
import json
from pathlib import Path
import subprocess

from run import (compare_outputs, expected_keys, parse_fixture, sha256,
                 extended_keys, parse_extended_fixture)


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
    extended = report["suite"] == "rng-extended"
    parse = parse_extended_fixture if extended else parse_fixture
    make_keys = extended_keys if extended else expected_keys
    keys = make_keys(parse(input_data))
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
        class_name = "RandomExtendedOracle" if extended else "RandomOracle"
        example = "rng_extended_probe" if extended else "rng_probe"
        commands = {"java": ["java", "-cp", str(directory / "classes"), class_name],
                    "rust": [str(directory / example)]}
        invalid = [
            b" \n", b"\t\n", b"x\t9223372036854775808\t1\n",
            b"x\t-9223372036854775809\t1\n", b"x\tNaN\t1\n", b"x\t0\t0\n",
            b"x\t0\t-1\n", b"x\t0\t100001\n", b"x\t0\t1.5\n",
            b"x\t0\t999999999999\n", b"\t0\t1\n", b"bad id\t0\t1\n",
            "é\t0\t1\n".encode(), b"x\t0\t1\nx\t1\t1\n",
            b"x\t0\n", b"x\t0\t1\textra\n",
        ]
        valid_inputs = [b"\n", b"# comment\n"]
        if extended:
            invalid = [b" \n", b"\t\n", b"x\t0\tunknown\n", b"x\t0\t\n",
                       b"x\t0\tint;\n", b"x\t0\t;int\n", b"x\t0\tint;;int\n", b"x\t0\tint \n",
                       b"x\t9223372036854775808\tint\n", b"x\t0\tseed:-9223372036854775809\n",
                       b"x\t0\tbound:2147483648\n", b"x\t0\tbound:abc\n", b"x\t0\tseed:1:2\n",
                       b"x\t0\tint\nx\t1\tint\n", b"bad id\t0\tint\n",
                       b"x\t0\t" + b";".join([b"int"]*100001) + b"\n",
                       b"x\t0\t" + b";".join([b"int"]*50001) + b"\ny\t0\t" + b";".join([b"int"]*50000) + b"\n"]
            tail = b"int;long;float;double;boolean;bound:17"
            valid_inputs.extend([
                b"control\t42\t" + tail + b"\ninvalid\t42\tbound:0;bound:-1;" + tail + b"\n",
                b"fresh\t-9223372036854775808\t" + tail + b"\nreseed\t1\tlong;seed:-9223372036854775808;" + tail + b"\n",
                b"maximum\t0\t" + b";".join([b"int"]*100000) + b"\n"])
        results, failures = [], []
        for data, valid in [(value, False) for value in invalid] + [(value, True) for value in valid_inputs]:
            valid_outputs = {}
            for side, command in commands.items():
                process = subprocess.run(command, input=data, capture_output=True, timeout=20)
                passed = (process.returncode == 0) if valid else (process.returncode != 0 and bool(process.stderr))
                if valid:
                    valid_outputs[side] = process.stdout
                    case_keys = make_keys(parse(data)) if data.strip() and not data.startswith(b"#") else []
                    passed = passed and not compare_outputs(process.stdout, process.stdout, case_keys)["mismatch_count"]
                result = {"side": side, "input_hex": data[:512].hex(), "input_sha256": sha256(data),
                          "expected_valid": valid, "returncode": process.returncode,
                          "stdout_sha256": sha256(process.stdout), "stdout_preview": process.stdout[:512].decode("utf-8", "replace"),
                          "stderr": process.stderr.decode("utf-8", "replace"), "accepted": passed}
                results.append(result)
                if not passed:
                    failures.append(result)
            if valid and valid_outputs["java"] != valid_outputs["rust"]:
                failures.append({"side": "comparison", "input_hex": data[:512].hex(), "returncode": "outputs differ"})
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
