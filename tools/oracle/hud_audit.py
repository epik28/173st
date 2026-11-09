#!/usr/bin/env python3
"""Verify the pinned Beta 1.7.3 GUI resources and GuiIngame layout audit."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import zipfile


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/hud/audit.json"
PINNED = {
    "jar_sha256": "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c",
    "jar_size": 1465375,
    "class": "uq",
    "class_sha256": "b4b61c370315e1ed95b42a24a01112afd7c2d44e387e48adc5e801edd4578f12",
    "resources": {
        "gui/gui.png": ("e223bea0b5004a3b5497607c6d529daf840396526a46eff75c299364cf9dc795", 14210, 256, 256),
        "gui/icons.png": ("8b5a8cfd44b933559c36aecaa722165ad048f1e942f1eeae616554124d6dac43", 2423, 256, 256),
    },
}
EXPECTED_LAYOUT = {
    "atlas_size": [256, 256],
    "hotbar": {
        "frame_screen": ["width/2-91", "height-22"],
        "frame_uv_size": [0, 0, 182, 22],
        "selection_screen": ["width/2-92+selected_slot*20", "height-23"],
        "selection_uv_size": [0, 22, 24, 22],
        "slots": 9,
        "slot_pitch": 20,
    },
    "crosshair": {"screen": ["width/2-7", "height/2-7"], "uv_size": [0, 0, 16, 16]},
    "health_full_20": {
        "hearts": 10,
        "blink_state": "inactive",
        "screen_first": ["width/2-91", "height-32"],
        "horizontal_pitch": 8,
        "background_uv_size": [16, 0, 9, 9],
        "full_heart_uv_size": [52, 0, 9, 9],
    },
}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def load_fixture(path):
    try:
        value = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read valid audit fixture: {error}") from error
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError("unsupported HUD audit fixture schema")
    reference = value.get("reference")
    if not isinstance(reference, dict):
        raise ValueError("fixture reference must be an object")
    if reference.get("jar_sha256") != PINNED["jar_sha256"] or reference.get("jar_size") != PINNED["jar_size"]:
        raise ValueError("fixture does not match the pinned client JAR")
    if reference.get("class") != PINNED["class"] or reference.get("class_sha256") != PINNED["class_sha256"]:
        raise ValueError("fixture class provenance differs from the pinned audit")
    if reference.get("class_super") != "ub" or reference.get("class_source_file_attribute") != "SourceFile":
        raise ValueError("fixture native class attributes are altered")
    if reference.get("class_identification") != (
        "uq contains both /gui/gui.png and /gui/icons.png references and its public "
        "a(FZII)V method performs the scaled HUD draws"
    ):
        raise ValueError("fixture native class identification was altered")
    if reference.get("jar_source") != "PrismLauncher local cache; verified against local Prism metadata":
        raise ValueError("fixture client JAR provenance was altered")
    want_resources = {
        name: {"sha256": digest, "bytes": size, "width": width, "height": height}
        for name, (digest, size, width, height) in PINNED["resources"].items()
    }
    if reference.get("resources") != want_resources:
        raise ValueError("fixture resource evidence differs from pinned hashes/dimensions")
    if value.get("layout") != EXPECTED_LAYOUT:
        raise ValueError("fixture layout evidence differs from the reviewed native layout")
    if reference.get("render_method") != {
        "name": "a",
        "descriptor": "(FZII)V",
        "mapped_name": "GuiIngame.renderGameOverlay(float, boolean, int, int)",
        "mapping_basis": "Beta 1.7.3 MCP name; obfuscated descriptor and class are authoritative",
    }:
        raise ValueError("fixture render method provenance is malformed or changed")
    if value.get("bytecode_evidence") != EXPECTED_EVIDENCE:
        raise ValueError("fixture bytecode evidence was altered")
    return value


EXPECTED_EVIDENCE = {
    "method_descriptor": "descriptor: (FZII)V",
    "scaled_resolution_constructor": "invokespecial #190                // Method qq.\"<init>\":(Lkv;II)V",
    "gui_resource": "ldc           #28                 // String /gui/gui.png",
    "icons_resource": "ldc           #29                 // String /gui/icons.png",
    "hotbar_frame": "261: sipush        182\n     264: bipush        22\n     266: invokevirtual #206                // Method b:(IIIIII)V",
    "hotbar_frame_position": "249: iconst_2\n     250: idiv\n     251: bipush        91\n     253: isub\n     254: iload         7\n     256: bipush        22\n     258: isub",
    "selection_frame": "296: bipush        22\n     298: bipush        24\n     300: bipush        22\n     302: invokevirtual #206                // Method b:(IIIIII)V",
    "selection_frame_position": "272: iconst_2\n     273: idiv\n     274: bipush        91\n     276: isub\n     277: iconst_1\n     278: isub\n     279: aload         11\n     281: getfield      #104                // Field ix.c:I\n     284: bipush        20\n     286: imul\n     287: iadd",
    "crosshair": "bipush        16\n     357: bipush        16\n     359: invokevirtual #206                // Method b:(IIIIII)V",
    "crosshair_position": "343: bipush        7\n     345: isub\n     346: iload         7\n     348: iconst_2\n     349: idiv\n     350: bipush        7\n     352: isub",
    "health_loop": "iload         16\n     480: bipush        10\n     482: if_icmpge     777",
    "health_y": "485: iload         7\n     487: bipush        32\n     489: isub\n     490: istore        17",
    "health_x": "609: iconst_2\n     610: idiv\n     611: bipush        91\n     613: isub\n     614: iload         16\n     616: bipush        8\n     618: imul",
    "health_background": "646: bipush        16\n     648: iload         18\n     650: bipush        9\n     652: imul",
    "health_full_uv": "735: bipush        52\n     737: iconst_0\n     738: bipush        9\n     740: bipush        9",
}


def default_jar():
    local_config = ROOT / "config/local.toml"
    try:
        import tomllib

        config = tomllib.loads(local_config.read_text(encoding="utf-8"))
        candidate = Path(config["paths"]["client_jar"])
    except (OSError, KeyError, ValueError) as error:
        raise ValueError(f"config/local.toml must provide paths.client_jar: {error}") from error
    return candidate if candidate.is_absolute() else ROOT / candidate


def png_dimensions(data, name):
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise ValueError(f"{name} is missing a valid PNG signature/IHDR")
    return struct.unpack(">II", data[16:24])


def verify_archive_contents(archive):
    """Check resource payloads and the native class before the javap audit."""
    try:
        for name, (digest, size, width, height) in PINNED["resources"].items():
            try:
                data = archive.read(name)
            except KeyError as error:
                raise ValueError(f"pinned JAR is missing {name}") from error
            actual_dims = png_dimensions(data, name)
            expected = (digest, size, width, height)
            actual = (sha256(data), len(data), *actual_dims)
            if actual != expected:
                raise ValueError(f"{name} bytes/hash/dimensions differ: {actual}")
        class_name = PINNED["class"] + ".class"
        class_bytes = archive.read(class_name)
    except KeyError as error:
        raise ValueError(f"pinned client JAR is missing {error}") from error
    if sha256(class_bytes) != PINNED["class_sha256"]:
        raise ValueError("uq.class hash differs from the native class provenance")


def verify(jar_path, fixture_path=FIXTURE):
    fixture = load_fixture(fixture_path)
    jar_bytes = Path(jar_path).read_bytes()
    if len(jar_bytes) != PINNED["jar_size"] or sha256(jar_bytes) != PINNED["jar_sha256"]:
        raise ValueError("client JAR hash/size differs from the pinned Beta 1.7.3 client")
    try:
        with zipfile.ZipFile(jar_path) as archive:
            verify_archive_contents(archive)
    except zipfile.BadZipFile as error:
        raise ValueError(f"pinned client JAR cannot provide the expected class: {error}") from error

    javap = shutil.which("javap")
    if not javap:
        raise ValueError("javap is required to verify native method/layout provenance")
    try:
        result = subprocess.run(
            [javap, "-classpath", str(jar_path), "-p", "-s", "-c", "uq"],
            check=False,
            capture_output=True,
            text=True,
            timeout=20,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ValueError(f"javap failed: {error}") from error
    if result.returncode != 0:
        raise ValueError(f"javap rejected pinned class: {result.stderr.strip()}")
    disassembly = result.stdout
    for label, snippet in fixture["bytecode_evidence"].items():
        if snippet not in disassembly:
            raise ValueError(f"native bytecode evidence not found for {label}")
    for required in (
        'Compiled from "SourceFile"',
        "public class uq extends ub",
        "public void a(float, boolean, int, int);",
        "descriptor: (FZII)V",
        "private void b(float, int, int);",
        "String /gui/gui.png",
        "String /gui/icons.png",
    ):
        if required not in disassembly:
            raise ValueError(f"native class/method provenance missing: {required}")
    # Sanity guard that both textures are sampled from their native 256x256 atlases.
    if fixture["layout"]["atlas_size"] != [256, 256]:
        raise ValueError("unsupported native atlas size")
    return fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, default=None, help="pinned Beta 1.7.3 client JAR")
    parser.add_argument("--fixture", type=Path, default=FIXTURE)
    args = parser.parse_args()
    try:
        fixture = verify(args.jar or default_jar(), args.fixture)
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    ref = fixture["reference"]
    print(
        "PASS: pinned client JAR, gui/gui.png + gui/icons.png hashes/dimensions, "
        f"{ref['class']}:{ref['render_method']['descriptor']} bytecode/layout evidence"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
