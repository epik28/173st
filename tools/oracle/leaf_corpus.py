#!/usr/bin/env python3
"""Verify the committed native LEAF-001 C01 capture; live mode calls the pinned client."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/leaf"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
INPUT = ("suite\tLEAF-001-C01\tdefault=constructor\tfancy=0,1\tmetadata=0..15\tfaces=0..5\t"
         "neighbors=0,1,18,20\tgrass_rgb=3e4ccccd,3ecccccd,3f19999a\n").encode("ascii")
ORACLE = ROOT / "tools/oracle/src/LeafOracle.java"
DEFAULT_JAR = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def parse_native(data):
    text = data.decode("ascii")
    rows = [line.split("\t") for line in text.splitlines() if line and not line.startswith("#")]
    leaves = [r for r in rows if r[0] == "leaf"]
    visible = [r for r in rows if r[0] == "visible"]
    collisions = [r for r in rows if r[0] == "collision"]
    defaults = [r for r in rows if r[0] == "default"]
    grass = [r for r in rows if r[0] == "grass"]
    vertices = [r for r in rows if r[0] == "vertex"]
    if len(leaves) != 192 or len(visible) != 48 or len(collisions) != 1 or defaults != [["default", "1", "52"]]:
        raise ValueError("leaf fixture does not cover both modes, all metadata/faces, neighbors, and collision")
    if len(grass) != 2 or len(vertices) != 96:
        raise ValueError("grass capture must contain both overlay modes and all packed vertices")
    leaf_keys = {(int(r[1]), int(r[2]), int(r[3])) for r in leaves}
    vis_keys = {(int(r[1]), int(r[2]), int(r[3])) for r in visible}
    if leaf_keys != {(mode, meta, face) for mode in (0, 1) for meta in range(16) for face in range(6)}:
        raise ValueError("leaf metadata/face coverage is incomplete")
    if vis_keys != {(mode, n, face) for mode in (0, 1) for n in (0, 1, 18, 20) for face in range(6)}:
        raise ValueError("leaf neighbor visibility coverage is incomplete")
    expected_calls = "#calls\tleaf_api=192\tsetters=2\tvisibility=48\topacity=33\ttile=193\tgrass_renderer=2\tvertices=96\tcollision=1\tgl_draw=0\tgl_flush=0"
    if expected_calls not in text:
        raise ValueError("native invocation audit counts differ")
    if "#input\t" + INPUT.decode().strip() not in text:
        raise ValueError("native input record differs from frozen case")
    if "#expected-render-state\tMinecraft constructor bytecode: GL11.glDepthFunc(515=GL_LEQUAL); GL11.glAlphaFunc(516=GL_GREATER,0.1f)" not in text:
        raise ValueError("native render-state evidence missing")
    # Independently assert the actual observed results that select this API contract.
    indexed = {(int(r[1]), int(r[2]), int(r[3])): (int(r[4]), int(r[5])) for r in leaves}
    for mode, opacity, base, pine in ((0, 1, 53, 133), (1, 0, 52, 132)):
        for meta in range(16):
            expected_tile = pine if (meta & 3) == 1 else base
            for face in range(6):
                if indexed[(mode, meta, face)] != (opacity, expected_tile):
                    raise ValueError("leaf native opacity/tile mapping mismatch")
    vindex = {(int(r[1]), int(r[2]), int(r[3])): int(r[4]) for r in visible}
    for face in range(6):
        if [vindex[(0, n, face)] for n in (0, 1, 18, 20)] != [1, 0, 0, 1]:
            raise ValueError("fast leaf visibility differs from native fixture")
        if [vindex[(1, n, face)] for n in (0, 1, 18, 20)] != [1, 0, 1, 1]:
            raise ValueError("fancy leaf visibility differs from native fixture")
    collision = collisions[0]
    if collision[1:] != ["OK", "0x0.0p0", "0x1.0p6", "0x0.0p0", "0x1.0p0", "0x1.04p6", "0x1.0p0"]:
        raise ValueError("original null-world collision query did not return unit cube at y=64")
    grass_by_mode = {int(r[1]): int(r[3]) for r in grass}
    if grass_by_mode != {0: 36, 1: 60}:
        raise ValueError("flat grass native packed vertex counts differ")
    return text, rows


def fixture_check():
    data = (FIXTURES / "native.tsv").read_bytes()
    _, rows = parse_native(data)
    if (FIXTURES / "input.tsv").read_bytes() != INPUT:
        raise ValueError("committed frozen C01 native input differs")
    audit = json.loads((FIXTURES / "audit.json").read_text(encoding="utf-8"))
    if audit.get("native_sha256") != digest(data) or audit.get("input_sha256") != digest(INPUT):
        raise ValueError("committed audit hashes do not bind C01 input and output")
    if audit.get("jar_sha256") != JAR_SHA or audit.get("calls") != {
        "leaf_api": 192, "setters": 2, "visibility": 48, "opacity": 33,
        "tile": 193, "grass_renderer": 2, "vertices": 96, "collision": 1,
        "gl_draw": 0, "gl_flush": 0,
    }:
        raise ValueError("committed native provenance or call counts differ")
    if audit.get("sources") != {name: JAR_SHA for name in ("bk", "nr", "uu", "cv", "nw", "minecraft")}:
        raise ValueError("original classes are not all bound to the pinned client-JAR code source")
    if "#setup\txp is a dynamic proxy supplying neighbor IDs/opacity and brightness 1; no world/chunk load; no GL draw/flush" not in data.decode("ascii"):
        raise ValueError("native synthetic-world setup is not documented")
    return {"leaf_rows": 192, "visibility_rows": 48, "grass_vertices": 96,
            "native_sha256": audit["native_sha256"], "repeatable": audit.get("repeatable") is True}


def configured_jar():
    configured = os.environ.get("MINECRAFT_BETA_173_JAR")
    if configured:
        return Path(configured).expanduser().resolve(strict=True)
    config = ROOT / "config/local.toml"
    for line in config.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("client_jar") and "=" in line:
            value = line.split("=", 1)[1].strip().strip('"')
            return (ROOT / value).resolve(strict=True)
    return DEFAULT_JAR.resolve(strict=True)


def one_live(jar, temp):
    classes = temp / "classes"
    classes.mkdir()
    compiled = subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(ORACLE)],
                              capture_output=True, timeout=40)
    if compiled.returncode:
        raise RuntimeError("javac failed: " + compiled.stderr.decode(errors="replace")[-3000:])
    result = subprocess.run(["java", "-XX:-UsePerfData", "-Xmx512m", "-cp", os.pathsep.join((str(classes), str(jar)),),
                             "LeafOracle", str(jar)], input=INPUT, capture_output=True, timeout=60)
    if result.returncode:
        raise RuntimeError("LeafOracle failed: " + result.stderr.decode(errors="replace")[-3000:])
    parse_native(result.stdout)
    return result.stdout


def verify_live(write=False):
    if not shutil.which("javac") or not shutil.which("java") or not shutil.which("javap"):
        raise RuntimeError("live leaf capture requires javac, java, and javap")
    jar = configured_jar()
    if digest(jar.read_bytes()) != JAR_SHA:
        raise ValueError("configured original client JAR hash mismatch")
    with tempfile.TemporaryDirectory(prefix="leaf-native-") as temp_name:
        temp = Path(temp_name)
        (temp / "first").mkdir()
        (temp / "second").mkdir()
        first = one_live(jar, temp / "first")
        second = one_live(jar, temp / "second")
        if first != second:
            raise AssertionError("two original-client leaf/grass captures differ")
        decomp = subprocess.run(["javap", "-classpath", str(jar), "-p", "-c", "net.minecraft.client.Minecraft"],
                                capture_output=True, check=True, timeout=30).stdout.decode("utf-8", errors="strict")
        bytecode_lines = decomp.splitlines()
        depth_ok = any("sipush" in line and line.rstrip().endswith("515") and i + 1 < len(bytecode_lines)
                        and "GL11.glDepthFunc" in bytecode_lines[i + 1]
                        for i, line in enumerate(bytecode_lines))
        if not depth_ok:
            raise ValueError("original Minecraft constructor does not set GL_LEQUAL (515)")
        alpha_ok = any("sipush" in line and line.rstrip().endswith("516") and i + 2 < len(bytecode_lines)
                       and "float 0.1f" in bytecode_lines[i + 1]
                       and "GL11.glAlphaFunc" in bytecode_lines[i + 2]
                       for i, line in enumerate(bytecode_lines))
        if not alpha_ok:
            raise ValueError("original Minecraft constructor does not set GL_GREATER/0.1 alpha test")
    audit = {
        "jar_sha256": JAR_SHA,
        "native_sha256": digest(first),
        "input_sha256": digest(INPUT),
        "classes": {key: value for key, value in
                    (field.split("=", 1) for field in first.decode().splitlines()[0].split("\t")[2:])},
        "sources": {key: value for key, value in
                    (field.split("=", 1) for field in first.decode().splitlines()[1].split("\t")[1:])},
        "bootstrap": {key: value for key, value in
                      (field.split("=", 1) for field in first.decode().splitlines()[2].split("\t")[1:])},
        "world_calls": {parts[1]: int(parts[2]) for parts in
                        (line.split("\t") for line in first.decode().splitlines()) if parts[0] == "#world_call"},
        "calls": {"leaf_api": 192, "setters": 2, "visibility": 48, "opacity": 33,
                  "tile": 193, "grass_renderer": 2, "vertices": 96, "collision": 1,
                  "gl_draw": 0, "gl_flush": 0},
        "repeatable": True,
        "render_state_javap_sha256": digest(decomp.encode()),
        "gl_depth_func": "GL_LEQUAL (515)",
        "gl_alpha_func": "GL_GREATER (516), cutoff 0.1f",
        "collision_query": "original uu.f(null,0,64,0), returned full unit cube",
        "grass_rgb_float_bits": ["3e4ccccd", "3ecccccd", "3f19999a"],
    }
    if write:
        FIXTURES.mkdir(parents=True, exist_ok=True)
        (FIXTURES / "input.tsv").write_bytes(INPUT)
        (FIXTURES / "native.tsv").write_bytes(first)
        (FIXTURES / "audit.json").write_text(json.dumps(audit, indent=2) + "\n", encoding="utf-8")
    elif first != (FIXTURES / "native.tsv").read_bytes():
        raise AssertionError("live native capture differs from committed fixture")
    return audit


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", action="store_true", help="opt in to original JAR calls and javap state verification")
    parser.add_argument("--write", action="store_true", help="write fixture after deterministic live recapture")
    args = parser.parse_args()
    if args.write and not args.live:
        parser.error("--write requires --live")
    print(json.dumps(verify_live(write=args.write) if args.live else fixture_check(), indent=2))
