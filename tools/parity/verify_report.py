#!/usr/bin/env python3
"""Verify a saved live report and detect one explicitly synthetic mutation."""

import argparse
import json
from pathlib import Path
import subprocess

from run import (compare_outputs, expected_keys, parse_fixture, sha256,
                 extended_keys, parse_extended_fixture, parse_chunk_fixture)


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
    chunk = report["suite"] == "chunk"
    extended = report["suite"] == "rng-extended"
    parse = parse_chunk_fixture if chunk else (parse_extended_fixture if extended else parse_fixture)
    make_keys = extended_keys if (chunk or extended) else expected_keys
    if chunk:
        jar = Path(report["reference_jar_path"])
        if sha256(jar.read_bytes()) != report["reference_jar_sha256"]:
            raise SystemExit("original JAR hash no longer matches report")
        audit = json.loads(Path(report["log_paths"]["reference_invocations"]).read_text())
        if audit != report["reference_invocations"] or any(audit.get(key, 0) <= 0 for key in
                ("native_constructors", "native_gets", "native_sets", "raw_reads")):
            raise SystemExit("original native invocations not verified")
    keys = make_keys(parse(input_data))
    if compare_outputs(reference, actual, keys)["mismatch_count"]:
        raise SystemExit("saved outputs no longer match")
    lines = actual.splitlines(keepends=True)
    fields = lines[0].decode("ascii").rstrip("\n").split("\t")
    components = fields[-1].split(",")
    value = int(components[0])
    components[0] = str(value + 1 if value < 2**31 - 1 else value - 1)
    fields[-1] = ",".join(components)
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
    if chunk:
        mutations = []
        for kind in ("swapped-nibble-order", "wrong-index"):
            for index, line in enumerate(actual.splitlines(keepends=True)):
                key = keys[index]
                fields = line.decode("ascii").rstrip("\n").split("\t")
                if kind == "swapped-nibble-order" and key[2] == "byte" and key[3][0] == "data":
                    value = int(fields[-1])
                    replacement = (value >> 4) | ((value & 15) << 4)
                    if replacement == value:
                        continue
                elif kind == "wrong-index" and key[2] == "index" and fields[-1].isdigit():
                    replacement = int(fields[-1]) + 1
                else:
                    continue
                fields[-1] = str(replacement)
                modified = actual.splitlines(keepends=True)
                modified[index] = ("\t".join(fields) + "\n").encode()
                copied = b"".join(modified)
                comparison = compare_outputs(reference, copied, keys)
                if comparison["mismatch_count"] != 1:
                    raise SystemExit(f"{kind} mutation not detected")
                path = directory / f"synthetic-{kind}.tsv"
                path.write_bytes(copied)
                mutations.append({"kind": "synthetic " + kind + "; not live parity", "path": str(path),
                                  "comparison": comparison})
                break
            else:
                raise SystemExit(f"fixture lacks {kind} coverage")
        (directory / "chunk-mutations.json").write_text(json.dumps(mutations, indent=2) + "\n")
        print("PASS: synthetic nibble-order and index mutations detected")
    if args.protocol:
        class_name = "RandomExtendedOracle" if extended else "RandomOracle"
        example = "rng_extended_probe" if extended else "rng_probe"
        if chunk:
            class_name, example = "ChunkOracle", "chunk_probe"
        commands = {"java": ["java", "-cp", str(directory / "classes"), class_name],
                    "rust": [str(directory / example)]}
        if chunk:
            commands["java"].append(str(jar))
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
        if chunk:
            invalid = [b" \n", b"\t\n", b"x\tunknown\n", b"x\t\n", b"x\tget:0:0:0;\n",
                       b"x\tget:0:0:0;;coord:0\n", b"x\tget:-1:0:0\n", b"x\tget:256:0:0\n",
                       b"x\tset:0:0:0:256:0:0:0\n", b"x\tcoord:2147483648\n", b"x\tbyte:data:16384\n",
                       b"x\tbyte:blocks:-1\n", b"x\tbyte:unknown:0\n", b"x\tget:0:0\n",
                       b"x\tcoord:0\nx\tcoord:1\n", b"x\tget:0:0:0 \n", b"bad id\tcoord:0\n",
                       b"x\t" + b";".join([b"coord:0"]*100001) + b"\n",
                       b"x\t" + b";".join([b"coord:0"]*50001) + b"\ny\t" + b";".join([b"coord:0"]*50000) + b"\n"]
            valid_inputs.extend([
                b"x\tset:0:0:0:255:3:7:11;set:0:1:0:128:12:4:9;get:0:0:0;get:0:1:0;byte:data:0;set:0:0:0:0:16:0:0;get:0:0:0;set:16:0:0:255:255:255:255;get:0:0:0;index:0:128:0;get:0:0:16\n",
                b"maximum\t" + b";".join([b"coord:0"]*100000) + b"\n"])
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
        if chunk:
            missing_command = commands["java"][:-1] + [str(directory / "absent-client.jar")]
            result = subprocess.run(missing_command, input=b"x\tcoord:0\n", capture_output=True, timeout=10)
            missing = {"side": "java-missing-jar", "input_hex": "not_applicable", "returncode": result.returncode,
                       "accepted": result.returncode != 0 and bool(result.stderr),
                       "stderr": result.stderr.decode("utf-8", "replace")}
            results.append(missing)
            if not missing["accepted"]:
                failures.append(missing)
            # Explicitly synthetic copy of OUR wrapper with a missing reflected method.
            synthetic = directory / "synthetic-reflection-error"
            synthetic.mkdir(exist_ok=True)
            source = Path(__file__).resolve().parents[2] / "tools/oracle/src/ChunkOracle.java"
            altered = source.read_text().replace('type.getMethod("a",', 'type.getMethod("missing_method",')
            (synthetic / "ChunkOracle.java").write_text(altered)
            build = subprocess.run(["javac", "-d", str(synthetic), str(synthetic / "ChunkOracle.java")],
                                   capture_output=True, timeout=30)
            if build.returncode:
                raise SystemExit("synthetic reflection-error wrapper did not compile")
            result = subprocess.run(["java", "-cp", str(synthetic), "ChunkOracle", str(jar)],
                                    input=b"x\tcoord:0\n", capture_output=True, timeout=10)
            reflection = {"side": "java-synthetic-reflection-error", "input_hex": "not_applicable",
                          "returncode": result.returncode, "kind": "synthetic mutation of our wrapper; original JAR unchanged",
                          "accepted": result.returncode != 0 and b"NoSuchMethodException" in result.stderr,
                          "stderr": result.stderr.decode("utf-8", "replace")}
            results.append(reflection)
            if not reflection["accepted"]:
                failures.append(reflection)
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
