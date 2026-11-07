"""Saved NBT evidence and live protocol checks, separate from the parity run."""

import json
from pathlib import Path
import subprocess

from run import (canonical_nbt, compare_outputs, inspect_nbt_outputs,
                 parse_nbt_fixture, sha256)


def verify_nbt(report_path, report, protocol):
    directory = report_path.resolve().parent
    saved = {}
    for label, path, key in (
        ("input", report["log_paths"]["input"], "input_sha256"),
        ("java", report["log_paths"]["java-run"]["stdout"], "reference_output_sha256"),
        ("rust", report["log_paths"]["rust-run"]["stdout"], "rust_output_sha256"),
        ("trees", report["log_paths"]["canonical_trees"], "canonical_trees_sha256"),
        ("binary", directory / "nbt_probe", "rust_binary_sha256"),
        ("adapter", directory / "classes/NbtOracle.class", "adapter_class_sha256"),
        ("jar", report["reference_jar_path"], "reference_jar_sha256"),
    ):
        saved[label] = Path(path).read_bytes()
        if sha256(saved[label]) != report[key]:
            raise SystemExit(f"NBT saved bytes do not match {key}")
    for label in ("java-run", "rust-run", "java-build", "rust-build"):
        process = report["processes"][label]
        if process["returncode"] != 0 or process["error"] or process["timeout"]:
            raise SystemExit("NBT report contains unsuccessful process")
    if report["source_hashes"] != report["source_hashes_after"]:
        raise SystemExit("NBT sources changed during the live run")
    cases = parse_nbt_fixture(saved["input"])
    audit = report["reference_invocations"]
    stderr = Path(report["log_paths"]["java-run"]["stderr"]).read_text()
    if (audit.get("named_reads") != len(cases) or audit.get("named_writes") != len(cases)
            or audit.get("jar_sha256") != report["reference_jar_sha256"]
            or audit.get("classes") != ["ij", "nu"]
            or f"jar_sha256={report['reference_jar_sha256']}" not in stderr
            or f"cases={len(cases)} classes=ij,nu" not in stderr):
        raise SystemExit("NBT original invocation audit missing")
    keys = [(case["id"], 0, "nbt") for case in cases]
    if compare_outputs(saved["java"], saved["rust"], keys)["mismatch_count"]:
        raise SystemExit("NBT saved outputs differ")
    trees = {"input": [{"id": case["id"], "tree": canonical_nbt(case["data"])} for case in cases],
             "java": inspect_nbt_outputs(saved["java"], cases),
             "rust": inspect_nbt_outputs(saved["rust"], cases)}
    if trees != json.loads(saved["trees"]) or trees["java"] != trees["rust"]:
        raise SystemExit("NBT canonical evidence differs")
    mutations = []
    # Offsets belong to a selected real output; mutate only a copy of that output.
    for kind, width in (("endian", 2), ("length", 2), ("tag-type", 1),
                        ("modified-utf8", 1), ("payload", 1)):
        rows = saved["rust"].splitlines(keepends=True)
        fields = rows[0].decode("ascii").rstrip("\n").split("\t")
        wire = bytearray.fromhex(fields[2])
        if kind == "endian":
            offset = wire.index(b"\x02\x00\x05short") + 8
            wire[offset:offset + width] = wire[offset:offset + width][::-1]
        elif kind == "length":
            wire[1:3] = b"\xff\xff"
        elif kind == "tag-type":
            wire[0] = 12
        elif kind == "modified-utf8":
            offset = wire.index(b"\xc0\x80")
            wire[offset] = 0xff
        else:
            offset = wire.index(b"\x01\x00\x04byte") + 7
            wire[offset] ^= 1
        fields[2] = wire.hex()
        rows[0] = ("\t".join(fields) + "\n").encode("ascii")
        altered = b"".join(rows)
        result = compare_outputs(saved["java"], altered, keys)
        if result["mismatch_count"] != 1 or result["first_mismatch"]["record"] != 0:
            raise SystemExit(f"NBT comparator missed {kind}")
        path = directory / f"synthetic-nbt-{kind}.tsv"
        path.write_bytes(altered)
        mutations.append({"kind": kind, "path": str(path), "sha256": sha256(altered), "comparison": result})
    (directory / "nbt-mutations.json").write_text(json.dumps({
        "kind": "synthetic mutations of copied live output; not live parity",
        "mutations": mutations}, indent=2) + "\n")
    print(f"PASS: NBT saved hashes, canonical trees, consumed bytes and five synthetic mutations ({len(cases)} cases)")
    if not protocol:
        return 0

    commands = {"java": ["java", "-Xmx256m", "-cp", str(directory / "classes") + ":" + report["reference_jar_path"],
                         "NbtOracle", report["reference_jar_path"]],
                "rust": [str(directory / "nbt_probe")]}
    empty = b"0a000000"
    invalid = [b" \n", b"\t\n", b"x\n", b"x\t\n", b"x\t0\n", b"x\tgg\n", b"x\t+1\n",
               b"x\t0a 00000\n", b"x\t0a000000\textra\n", b"\t0a000000\n", b"bad id\t0a000000\n",
               "é\t0a000000\n".encode(), b"x\t0a000000\nx\t0a000000\n"]
    root = Path(__file__).resolve().parents[2]
    invalid.extend(row + b"\n" for row in (root / "tests/fixtures/nbt/invalid.tsv").read_bytes().splitlines()
                   if row and not row.startswith(b"#"))
    valid = [b"\n", b"# comment\n", b"x\t" + empty + b"\n", b"x\t0A000000\r\n",
             b"x\t080000000100\n", b"x\t0800000002c080\n",
             b"x\t0800000006eda0bdedb880\n",
             b"x\t0a00000100017801010001780200\n"]
    results, failures = [], []

    def execute(name, command, data, accepted):
        try:
            process = subprocess.run(command, input=data, capture_output=True, timeout=20)
            try:
                passed = accepted(process)
            except ValueError:
                passed = False
            result = {"side": name, "input_hex": data[:512].hex(), "input_sha256": sha256(data),
                      "returncode": process.returncode, "stdout_hex": process.stdout[:512].hex(),
                      "stdout_sha256": sha256(process.stdout), "stderr": process.stderr.decode("utf-8", "replace"),
                      "accepted": bool(passed)}
        except (OSError, subprocess.TimeoutExpired) as error:
            process = None
            result = {"side": name, "input_hex": data[:512].hex(), "error": str(error), "accepted": False}
        results.append(result)
        if not result["accepted"]:
            failures.append(result)
        return process

    for data in invalid:
        for side, command in commands.items():
            execute(side + "-reject", command, data, lambda p: p.returncode != 0 and bool(p.stderr))
    for data in valid:
        outputs = []
        for side, command in commands.items():
            def accepts(process):
                if process.returncode:
                    return False
                if data.startswith((b"\n", b"#")):
                    return not process.stdout
                inspect_nbt_outputs(process.stdout, parse_nbt_fixture(data))
                return True
            process = execute(side + "-accept", command, data, accepts)
            outputs.append(process.stdout if process else None)
        if outputs[0] != outputs[1]:
            failures.append({"side": "valid-comparison", "input_hex": data.hex()})

    missing = commands["java"][:-1] + [str(directory / "absent-client.jar")]
    execute("java-missing-jar", missing, b"x\t" + empty + b"\n", lambda p: p.returncode != 0 and bool(p.stderr))
    missing_classes = ["java", "-cp", str(directory / "classes"), "NbtOracle", report["reference_jar_path"]]
    execute("java-missing-original-class", missing_classes, b"x\t" + empty + b"\n",
            lambda p: p.returncode != 0 and b"NoClassDefFoundError" in p.stderr)
    # Separate format evidence: original Beta has no tag 11, Rust extension does.
    for row in (root / "tests/fixtures/nbt/wire-only-int-array.tsv").read_bytes().splitlines():
        data = row + b"\n"
        execute("java-no-int-array", commands["java"], data, lambda p: p.returncode != 0 and bool(p.stderr))
        def wire_accepts(process):
            return process.returncode == 0 and bool(inspect_nbt_outputs(process.stdout, parse_nbt_fixture(data)))
        execute("rust-wire-int-array", commands["rust"], data, wire_accepts)
    # Native quirks are measured, not hidden by normalizing the comparator or
    # changing the Rust API requirements. These are NOT equal-output parity.
    native_differences = [
        ("empty-list-type", "0900000600000000", "0900000100000000", True, None),
        ("negative-list-length", "09000001ffffffff", "0900000100000000", False, b"negative"),
        ("unpaired-surrogate", "0800000003eda080", "0800000003eda080", False, b"unpaired"),
        ("nonempty-end-list", "0900000000000001", "0900000000000001", False, b"list element type"),
        ("unknown-empty-list", "0900000c00000000", "0900000100000000", False, b"unknown"),
    ]
    for name, original, rewritten, rust_valid, error in native_differences:
        data = f"{name}\t{original}\n".encode("ascii")
        expected_java = f"{name}\t0\t{rewritten}\t{len(bytes.fromhex(original))}\n".encode("ascii")
        execute("java-native-difference-" + name, commands["java"], data,
                lambda p: p.returncode == 0 and p.stdout == expected_java)
        expected_rust = f"{name}\t0\t{original}\t{len(bytes.fromhex(original))}\n".encode("ascii")
        execute("rust-contract-difference-" + name, commands["rust"], data,
                lambda p: (p.returncode == 0 and p.stdout == expected_rust) if rust_valid
                else (p.returncode != 0 and error in p.stderr and not p.stdout))
    # Allocation/depth protection belongs to our safe reader, not the original
    # unbounded implementation. Do not run hostile lengths against the JAR.
    guarded = [("negative-int-array", "0b0000ffffffff", b"negative"),
               ("huge-array", "0700007fffffff", b"collection elements"),
               ("huge-list", "090000037fffffff", b"collection elements"),
               ("collection-over-limit", "070000001e8481", b"collection elements")]
    for name, wire, error in guarded:
        execute("rust-safety-" + name, commands["rust"], f"{name}\t{wire}\n".encode(),
                lambda p: p.returncode != 0 and error in p.stderr and not p.stdout)
    for depth in (512, 513):
        wire = b"\x0a\x00\x00" * (depth + 1) + b"\x00" * (depth + 1)
        data = f"depth-{depth}\t{wire.hex()}\n".encode()
        expected = f"depth-{depth}\t0\t{wire.hex()}\t{len(wire)}\n".encode()
        execute(f"rust-depth-{depth}", commands["rust"], data,
                lambda p: (p.returncode == 0 and p.stdout == expected) if depth == 512
                else (p.returncode != 0 and b"depth limit" in p.stderr and not p.stdout))
    review = {"kind": "live NBT protocol and separately labelled wire extension checks",
              "report": str(report_path.resolve()), "native_differences": [item[0] for item in native_differences],
              "results": results, "failures": failures}
    (directory / "protocol-review.json").write_text(json.dumps(review, indent=2) + "\n")
    if failures:
        print(f"FAIL: NBT protocol ({len(failures)} failures); see {directory / 'protocol-review.json'}")
        return 1
    print(f"PASS: NBT protocol ({len(results)} executions), including missing JAR/classes and wire-only IntArray")
    return 0
