#!/usr/bin/env python3
"""Build and run the read-only native WORLD-LOAD-001 C01 corpus."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/world"
MARK_WORLD = "WORLD-LOAD-001 copied inputs\n"
MARK_REGION = "REGION-001 copied inputs\n"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
EXPECTED_LEVEL = {"size": 411, "sha256": "2711d16242afd13909575955d4a34502da01f6f666ee79341fe1c1ba96c8a960"}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def config(root=ROOT):
    return tomllib.loads((root / "config/local.toml").read_text())["paths"]


def manifest(root=ROOT):
    return json.loads((root / "tests/fixtures/world/sources.json").read_text())


def source_paths(root=ROOT):
    cfg, doc = config(root), manifest(root)
    world = (root / cfg["prism_instance"] / doc["world_relative_path"]).resolve(strict=True)
    result = {}
    for name, expected in doc["files"].items():
        if Path(name).name != name:
            raise ValueError("unsafe source basename")
        parent = world if name == "level.dat" else world / "region"
        path = parent / name
        if path.is_symlink() or path.resolve(strict=True).parent != parent.resolve(strict=True):
            raise ValueError("unsafe source path")
        raw = path.read_bytes()
        if len(raw) != expected["size"] or digest(raw) != expected["sha256"]:
            raise ValueError(f"source size/hash mismatch: {name}")
        result[name] = path
    return world, result


def snapshot(run_dir, root=ROOT):
    artifacts = (root / config(root).get("artifacts", "artifacts")).resolve()
    run_dir = Path(run_dir).resolve(strict=True)
    if not run_dir.is_relative_to(artifacts) or run_dir == artifacts:
        raise ValueError("snapshot must be inside a unique artifacts run")
    world, sources = source_paths(root)
    copied = run_dir / "input"
    copied.mkdir()
    before = {}
    for name, path in sources.items():
        data = path.read_bytes()
        info = manifest(root)["files"][name]
        if len(data) != info["size"] or digest(data) != info["sha256"]:
            raise ValueError(f"source changed during copy: {name}")
        with (copied / name).open("xb") as f:
            f.write(data)
        if digest((copied / name).read_bytes()) != info["sha256"]:
            raise ValueError(f"copy hash mismatch: {name}")
        before[name] = info["sha256"]
    (copied / ".world-input-copy").write_text(MARK_WORLD)
    (copied / ".region-input-copy").write_text(MARK_REGION)
    evidence = {"source_world": str(world), "source_before": before, "copied_before": dict(before)}
    verify_snapshot(run_dir, evidence, root, publish=False)
    (run_dir / "snapshot.json").write_text(json.dumps(evidence, indent=2) + "\n")
    return copied, evidence


def verify_snapshot(run_dir, evidence, root=ROOT, publish=True):
    _, source_map = source_paths(root)
    after = {name: digest(path.read_bytes()) for name, path in source_map.items()}
    copied = Path(run_dir) / "input"
    copied_after = {name: digest((copied / name).read_bytes()) for name in evidence["source_before"]}
    if after != evidence["source_before"] or copied_after != evidence["copied_before"]:
        raise ValueError("source or input copy changed during execution")
    evidence["source_after"] = after
    evidence["copied_after"] = copied_after
    if publish:
        (Path(run_dir) / "snapshot.json").write_text(json.dumps(evidence, indent=2) + "\n")


def expected_fixture(root=ROOT):
    old = (root / "tests/fixtures/region/live.tsv").read_text().splitlines()
    chunks = []
    for line in old:
        case_id, filename, x, z = line.split("\t")
        chunks.append(f"{case_id}\tchunk\t{filename}\t{x}\t{z}\n")
    return "# WORLD-LOAD-001 native input coordinates; one metadata row plus REGION-001 slots\n" + \
        "live-level\tlevel\tlevel.dat\t-\t-\n" + "".join(chunks)


def class_hash(jar, class_name):
    import zipfile
    with zipfile.ZipFile(jar) as zf:
        return digest(zf.read(class_name.replace('.', '/') + ".class"))


def run_native(run_dir, input_dir, rows, root=ROOT, timeout=90):
    jar = (root / config(root)["client_jar"]).resolve(strict=True)
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured JAR SHA mismatch")
    classes = run_dir / "classes"
    classes.mkdir()
    java_src = root / "tools/oracle/src/WorldOracle.java"
    subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(java_src)], check=True, capture_output=True, timeout=40)
    command = ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", f"{classes}:{jar}", "WorldOracle",
               str(jar), str(input_dir), str(run_dir / "retained"), str(run_dir / "native-audit.json")]
    result = subprocess.run(command, input=rows.encode(), capture_output=True, timeout=timeout)
    (run_dir / "native.stdout.tsv").write_bytes(result.stdout)
    (run_dir / "native.stderr.log").write_bytes(result.stderr)
    if result.returncode:
        raise RuntimeError(f"native oracle failed ({result.returncode}): {result.stderr.decode(errors='replace')[-2000:]}")
    return json.loads((run_dir / "native-audit.json").read_text()), result.stdout.decode()


def run_generated(run_dir, jar, classes):
    generated = run_dir / "generated"
    generated.mkdir()
    command = ["java", "-XX:-UsePerfData", "-Xmx128m", "-cp", f"{classes}:{jar}", "WorldOracle",
               "--self-test", str(jar), str(generated / "retained"), str(generated / "audit.json")]
    result = subprocess.run(command, capture_output=True, timeout=20)
    (generated / "stdout.log").write_bytes(result.stdout)
    (generated / "stderr.log").write_bytes(result.stderr)
    if result.returncode:
        raise RuntimeError(f"generated native self-test failed: {result.stderr.decode(errors='replace')[-1200:]}")
    return json.loads((generated / "audit.json").read_text())


def write_fixtures():
    FIXTURE.mkdir(parents=True, exist_ok=True)
    (FIXTURE / "live.tsv").write_text(expected_fixture())
    (FIXTURE / "provenance.md").write_text("""# WORLD-LOAD-001 C01 native corpus

The 749 rows are coordinates: the original level.dat plus the 748 selected slots from REGION-001. Expected outputs are produced by `tools/oracle/src/WorldOracle.java` using the pinned Beta 1.7.3 client JAR. The adapter calls native `ij.b(DataInput)`, `ei(nu)`, and `qj.a(int,int)`; `native_chunk_loads=0` and entity/tile factories are never called. `live.tsv` contains no expected answers or world bytes. Artifacts, class files, copied saves, and retained raw NBT stay ignored under `artifacts/`.

`sources.json` pins five source files. The runner compares their exact sizes and SHA-256 before copy, after copy, and in `finally` after native execution. See `audit.json` for class/JAR hashes, call counts, generated native semantics checks, and the exact live row digest.
""")


def verify(root=ROOT, write=False):
    rows = expected_fixture(root)
    fixture = root / "tests/fixtures/world/live.tsv"
    if not write and fixture.read_text() != rows:
        raise AssertionError("live coordinate fixture differs from generator")
    parent = root / config(root).get("artifacts", "artifacts") / "oracle-world"
    parent.mkdir(parents=True, exist_ok=True)
    run_dir = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    copied, evidence = snapshot(run_dir, root)
    try:
        audit, stdout = run_native(run_dir, copied, rows, root)
        repeat_dir = run_dir / "repeat"
        repeat_dir.mkdir()
        repeat_audit, repeat_stdout = run_native(repeat_dir, copied, rows, root)
        if repeat_stdout != stdout or repeat_audit != audit:
            raise AssertionError("native live run was not reproducible")
        jar = (root / config(root)["client_jar"]).resolve()
        generated = run_generated(run_dir, jar, run_dir / "classes")
        if digest(jar.read_bytes()) != JAR_SHA:
            raise AssertionError("configured JAR changed during native execution")
    finally:
        verify_snapshot(run_dir, evidence, root)
    lines = stdout.splitlines()
    if len(lines) != 749 or sum(row.split("\t")[1] == "chunk" for row in lines) != 748:
        raise AssertionError("native live output count mismatch")
    if audit.get("live_rows") != 749 or audit.get("native_calls", {}).get("qj_a_int_int") != 748:
        raise AssertionError("native invocation audit mismatch")
    if audit["native_calls"].get("ij_b_DataInput") != 705 or audit["native_calls"].get("ei_constructor") != 1:
        raise AssertionError("live native-call counts must exclude generated tests")
    if audit.get("native_calls", {}).get("native_chunk_loads") != 0:
        raise AssertionError("full native chunk loader unexpectedly invoked")
    if generated["native_calls"] != {"ij_b_DataInput": 11, "ei_constructor": 7, "qj_a_int_int": 0, "native_chunk_loads": 0}:
        raise AssertionError("generated native counts mismatch")
    if audit["classes"] != {name: class_hash((root / config(root)["client_jar"]).resolve(), name)
                            for name in ("ij", "nu", "ei", "qj")}:
        raise AssertionError("class hash audit mismatch")
    native_audit = dict(audit)
    retained = native_audit.pop("retained_files")
    retained_manifest = json.dumps(retained, sort_keys=True, separators=(",", ":")).encode()
    (run_dir / "retained-manifest.json").write_bytes(retained_manifest + b"\n")
    generated_summary = {k: v for k, v in generated.items() if k != "retained_files"}
    report = {"kind": "native original-JAR world data oracle; no Rust parity asserted",
              "live_cases": 749, "input_sha256": digest(rows.encode()), "jar_sha256": JAR_SHA,
              "source_before": evidence["source_before"], "source_after": evidence["source_after"],
              "copied_after": evidence["copied_after"], "native": native_audit, "generated": generated_summary,
              "retained_files": len(retained), "retained_manifest_sha256": digest(retained_manifest),
              "reference_sha256": digest(stdout.encode()), "repeat_reference_sha256": digest(repeat_stdout.encode()),
              "repeatable": True}
    (run_dir / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    if write:
        write_fixtures()
        (root / "tests/fixtures/world/generated.json").write_text(json.dumps(generated_summary, indent=2) + "\n")
        (root / "tests/fixtures/world/audit.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"live_cases": 749, "chunk_present": audit["chunk_present"],
                      "chunk_absent": audit["chunk_absent"], "native_calls": audit["native_calls"],
                      "generated_semantics": generated["generated_semantics"],
                      "generated_native_calls": generated["native_calls"],
                      "reference_sha256": report["reference_sha256"], "repeatable": True,
                      "run_dir": str(run_dir)}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="write coordinate/provenance fixtures and run original oracle")
    args = parser.parse_args()
    verify(write=args.write)


if __name__ == "__main__":
    main()
