#!/usr/bin/env python3
"""Snapshot real McRegion inputs and measure the original qj on copies only."""

import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import tomllib
import zlib

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/region"
MARKER = "REGION-001 copied inputs\n"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def configuration(root=ROOT):
    return tomllib.loads((root / "config/local.toml").read_text())["paths"]


def source_hashes(root=ROOT):
    config = configuration(root)
    manifest = json.loads((root / "tests/fixtures/region/sources.json").read_text())
    source = (root / config["prism_instance"] / manifest["world_relative_path"]).resolve(strict=True)
    result = {}
    for name, expected in manifest["files"].items():
        path = source / name
        if Path(name).name != name or path.is_symlink() or path.resolve(strict=True).parent != source:
            raise ValueError("source path escape")
        if path.stat().st_size != expected["size"]:
            raise ValueError(f"source size mismatch: {name}")
        actual = digest(path.read_bytes())
        if actual != expected["sha256"]:
            raise ValueError(f"source hash mismatch: {name}")
        result[name] = actual
    return source, result


def snapshot(directory, root=ROOT):
    artifacts = (root / configuration(root).get("artifacts", "artifacts")).resolve()
    directory = Path(directory).resolve(strict=True)
    if not directory.is_relative_to(artifacts) or directory == artifacts:
        raise ValueError("snapshot must be in a run-specific artifacts directory")
    source, before = source_hashes(root)
    target = directory / "input"
    target.mkdir()  # Refuse existing folders or symlinks; never overwrite a snapshot.
    for name, expected in before.items():
        contents = (source / name).read_bytes()
        if digest(contents) != expected:
            raise ValueError("source changed while copying")
        with (target / name).open("xb") as output:
            output.write(contents)
        if digest((target / name).read_bytes()) != expected:
            raise ValueError("copy hash mismatch")
    (target / ".region-input-copy").write_text(MARKER)
    _, after = source_hashes(root)
    if before != after:
        raise ValueError("source changed after copying")
    evidence = {"source": str(source), "before": before, "after_copy": after, "copied": before}
    (directory / "snapshot.json").write_text(json.dumps(evidence, indent=2) + "\n")
    return target, evidence


def verify_snapshot(directory, evidence, root=ROOT):
    _, after = source_hashes(root)
    copied = {name: digest((Path(directory) / "input" / name).read_bytes()) for name in evidence["before"]}
    if after != evidence["before"] or copied != evidence["copied"]:
        raise ValueError("source or input copy changed during execution")
    evidence.update(after_run=after, copied_after_run=copied)
    (Path(directory) / "snapshot.json").write_text(json.dumps(evidence, indent=2) + "\n")


def live_cases(input_dir):
    rows, counts = [], {}
    for path in sorted(input_dir.glob("*.mcr")):
        _, rx, rz, _ = path.name.split(".")
        rx, rz = int(rx), int(rz)
        header = path.read_bytes()[:4096]
        occupied = [slot for slot in range(1024) if int.from_bytes(header[slot*4:slot*4+4], "big") >> 8]
        empty = [slot for slot in range(1024) if slot not in occupied]
        # Empty samples include corners/edges when absent, plus first and last gaps.
        selected = set(occupied) | set(empty[:4] + empty[-4:])
        selected.update(slot for slot in (0, 31, 32, 991, 992, 1023) if slot in empty)
        for slot in sorted(selected):
            x, z = rx*32+slot%32, rz*32+slot//32
            rows.append(f"live-{rx}-{rz}-{slot}\t{path.name}\t{x}\t{z}\n")
        counts[path.name] = {"present": len(occupied), "absent": len(selected)-len(occupied)}
    if sum(v["present"] for v in counts.values()) != 704:
        raise ValueError("live occupied count differs from contract")
    return "".join(rows).encode("ascii"), counts


def synthetic_cases():
    """Synthetic wire inputs; expected classification follows the safe contract."""
    payload = bytes.fromhex("0a0000030001780102030400")

    def region(compressed, compression=2, count=None):
        record = struct.pack(">I", len(compressed)+1) + bytes([compression]) + compressed
        count = count or (len(record)+4095)//4096
        result = bytearray((2+count)*4096)
        result[:4] = struct.pack(">I", (2 << 8) | count)
        result[4096:4100] = struct.pack(">I", 0xffffffff)
        result[8192:8192+len(record)] = record
        return bytes(result)

    compressed = zlib.compress(payload)
    valid = region(compressed)
    cases = {"zlib": (valid, True), "gzip": (region(gzip.compress(payload, mtime=0), 1), True),
             "zlib-trailing": (region(compressed+b"trailing"), True),
             "gzip-trailing": (region(gzip.compress(payload, mtime=0)+b"trailing", 1), True),
             "gzip-multiple": (region(gzip.compress(payload, mtime=0)*2, 1), True),
             "raw-unsupported": (region(payload, 3), False), "unknown-compression": (region(compressed, 255), False),
             "short-header": (b"\0"*4096, False), "unaligned": (valid[:-1], False),
             "truncated-zlib": (region(compressed[:-2]), False),
             "bad-checksum": (region(compressed[:-1]+bytes([compressed[-1]^1])), False),
             "truncated-gzip": (region(gzip.compress(payload, mtime=0)[:-2], 1), False),
             "empty-region": (bytes(8192), True),
             "exact-sector": (region(compressed + bytes(4091-len(compressed))), True),
             "maximum-record": (region(compressed + bytes(255*4096-5-len(compressed))), True),
             "output-bomb": (region(zlib.compress(b"\0"*(64*1024*1024+1))), False)}
    for name, start, value in (("header-sector", 0, 0x101), ("outside-file", 0, 0xffffff01),
                               ("zero-sectors", 0, 0x200), ("zero-length", 8192, 0),
                               ("oversized-length", 8192, 4093), ("overlap", 4, 0x201)):
        altered = bytearray(valid)
        altered[start:start+4] = struct.pack(">I", value)
        cases[name] = (bytes(altered), False)
    padded = bytearray(valid)
    padded[-16:] = b"P" * 16
    cases["sector-padding"] = (bytes(padded), True)
    return cases


def coordinate_cases():
    base = synthetic_cases()["zlib"][0]
    result = []
    for coord in (-2**31, -33, -32, -31, -1, 0, 31, 32, 33, 2**31-1):
        region, local = divmod(coord, 32)
        slot = local + local*32
        contents = bytearray(base)
        contents[:4] = bytes(4)
        contents[4096:4100] = bytes(4)
        contents[slot*4:slot*4+4] = base[:4]
        contents[4096+slot*4:4100+slot*4] = base[4096:4100]
        result.append((f"coordinate-{coord}", f"r.{region}.{region}.mcr", coord, bytes(contents)))
    return result


def run_oracle(directory, input_dir, data, root=ROOT):
    config = configuration(root)
    jar = (root / config["client_jar"]).resolve(strict=True)
    classes = directory / "classes"
    classes.mkdir()
    subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(root / "tools/oracle/src/RegionOracle.java")],
                   check=True, capture_output=True, timeout=30)
    command = ["java", "-XX:-UsePerfData", "-Xmx256m", "-cp", f"{classes}:{jar}", "RegionOracle", str(jar)]
    result = subprocess.run(command + [str(input_dir), str(directory / "payloads"), str(directory / "native-audit.json")],
                            input=data, capture_output=True, timeout=60)
    (directory / "reference.tsv").write_bytes(result.stdout)
    (directory / "reference.stderr").write_bytes(result.stderr)
    if result.returncode:
        raise ValueError(f"native adapter failed: {result.stderr.decode(errors='replace')[:1000]}")
    return command


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="publish coordinate fixture and audit; never world bytes")
    args = parser.parse_args()
    parent = ROOT / configuration().get("artifacts", "artifacts") / "oracle-region"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    inputs, evidence = snapshot(directory)
    data, counts = live_cases(inputs)
    (directory / "input.tsv").write_bytes(data)
    try:
        command = run_oracle(directory, inputs, data)
        audit = json.loads((directory / "native-audit.json").read_text())
        if audit["present"] != 704 or audit["native_reads"] != len(data.splitlines()):
            raise ValueError("native live count mismatch")
        repeat = subprocess.run(command + [str(inputs), str(directory / "payloads-repeat"), str(directory / "native-audit-repeat.json")],
                                input=data, capture_output=True, timeout=60)
        if repeat.returncode or repeat.stdout != (directory / "reference.tsv").read_bytes():
            raise ValueError("native live output is not reproducible")
        synthetic = []
        for name, (contents, expected_valid) in synthetic_cases().items():
            case_dir = directory / "synthetic" / name
            copy = case_dir / "input"
            copy.mkdir(parents=True)
            (copy / ".region-input-copy").write_text(MARKER)
            (copy / "r.0.0.mcr").write_bytes(contents)
            result = subprocess.run(command + [str(copy), str(case_dir / "payloads"), str(case_dir / "audit.json")],
                                    input=f"{name}\tr.0.0.mcr\t0\t0\n".encode(), capture_output=True, timeout=30)
            (case_dir / "stdout").write_bytes(result.stdout)
            (case_dir / "stderr").write_bytes(result.stderr)
            synthetic.append({"name": name, "input_sha256": digest(contents), "safe_contract_valid": expected_valid,
                              "native_exit": result.returncode, "stdout": result.stdout.decode(errors="replace"),
                              "copy_unchanged": digest((copy / "r.0.0.mcr").read_bytes()) == digest(contents)})
            if not synthetic[-1]["copy_unchanged"]:
                raise ValueError(f"native mutated synthetic copy: {name}")
            if name not in {"oversized-length", "overlap"} and (result.returncode == 0) != expected_valid:
                raise ValueError(f"unexpected native synthetic result: {name}")
        coordinates = []
        for name, filename, coord, contents in coordinate_cases():
            case_dir = directory / "synthetic" / name
            copy = case_dir / "input"
            copy.mkdir(parents=True)
            (copy / ".region-input-copy").write_text(MARKER)
            (copy / filename).write_bytes(contents)
            row = f"{name}\t{filename}\t{coord}\t{coord}\n".encode()
            result = subprocess.run(command + [str(copy), str(case_dir / "payloads"), str(case_dir / "audit.json")],
                                    input=row, capture_output=True, timeout=30)
            (case_dir / "stdout").write_bytes(result.stdout)
            (case_dir / "stderr").write_bytes(result.stderr)
            if result.returncode or b"\tpresent\t4294967295\t2\t12\t" not in result.stdout:
                raise ValueError(f"native coordinate boundary failed: {coord}")
            coordinates.append(row)
        (directory / "synthetic-audit.json").write_text(json.dumps(synthetic, indent=2)+"\n")
        if args.write:
            (FIXTURES / "live.tsv").write_bytes(data)
            (FIXTURES / "coordinates.tsv").write_bytes(b"".join(coordinates))
            (FIXTURES / "audit.json").write_text(json.dumps({"kind": "native original adapter only; Rust parity pending",
                "files": counts, "cases": len(data.splitlines()), "present": 704, "input_sha256": digest(data),
                "native": audit, "synthetic": synthetic, "coordinate_cases": len(coordinates)}, indent=2)+"\n")
    finally:
        verify_snapshot(directory, evidence)
    print(f"PASS: original qj read {audit['present']} occupied and {audit['absent']} absent slots twice; {len(synthetic)} synthetic probes, {len(coordinates)} coordinate boundaries; {directory}")


if __name__ == "__main__":
    main()
