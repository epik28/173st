#!/usr/bin/env python3
"""Verify native Beta inventory screen evidence against the pinned client JAR."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/inventory_ui/audit.json"
ORACLE = ROOT / "tools/oracle/src/InventoryUiOracle.java"
PINNED_JAR_SHA256 = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
PINNED_JAR_SIZE = 1465375
PINNED_CLASSES = {
    "net.minecraft.client.Minecraft": "db99df969e5f30d01ad4424a4d63d3b1f4a11832055dd291e65d476cfa4c38b3",
    "kv": "fd62ebcc36e2f2a1428cb41b2f1f8ebd86fee9855ec1b7f7055ba307d6577102",
    "qb": "4c1524aeb93b82f91517c5ed5a3dcce02c1b110a9d65b3db7089cfc8a37cd7aa",
    "ue": "b7ca251056b4c59ade5c419517de8e1a217b6c9651d4521f6fa500c6b1655570",
    "id": "934c3e5664d2113ab02b57ff73560bac66ff65fd5620f3754db69cffbe72792c",
    "da": "55349b76fb6119d1ea841fb3488b19b387a32abae0b2ce94b5e11706cc4b6890",
    "aa": "8de8d32824f6ebc4cec9b0aca886cc0c3569cefeb77e22e8f94ef39d27b86cb3",
    "gp": "1c5ab56b12dc83efc81c2fb058bc28bff1b10e64e4435ba1a31f3e3ce756d7c3",
    "iz": "3e8a07bf8c43729c8367bab454af876e698505620f230e012432c04e3828d6d6",
    "gm": "f9d971a83efa0c6399f1fc4bc0bc06c6a6249908afc6b46f5889a49365c190c7",
    "uu": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "ck": "fe3d757cd56ac42ef8e0c0aaecf5fc7878406fe66db88ecf8061456f50cdd2eb",
    "dm": "c62e04938cf3b6d54ebf1de986fabc6f8fe366e74e3cb70bc1c4a053370c5f89",
    "qd": "5721cdbdbf6f52606e7710df443cb19a573318cd5fd1913aecb40933e9119f56",
    "gs": "52e2d97748d9a306703e018822fece7d0ef9ca8ddf3448a1d0e568f2fbe29202",
}
PINNED_RESOURCES = {
    "gui/inventory.png": ("53dd5bde88588215b1700aee4a7dd70a3049e7e53f914982774ebdc26d6862b0", 2761, 256, 256),
    "gui/items.png": ("545699e0c5f655c1e1876bb9f2fa5d93c4fd514dae3802d1cea43ae0003343f3", 21381, 256, 256),
}
EXPECTED_ICONS = [[1, "ck", 1], [3, "ck", 2], [332, "dm", 14], [267, "qd", 66]]
EXPECTED_EVIDENCE_KEYS = {
    "open_screen", "default_e_binding", "escape_closes_and_grabs_mouse", "panel_dimensions",
    "panel_center_and_uv_draw", "panel_uv_origin", "inventory_slot_grid", "hotbar_slot_row",
    "native_item_atlas_sampling", "native_item_stack_icon_index", "gui_texture", "items_texture",
}
EXPECTED_EVIDENCE_SHA256 = "49be3c2cba432e983c39f5e8b889d6c4e4625f8a6d523521fddfc2e277d4e0d2"
EXPECTED_METHODS = {
    "inventory_key_dispatch": ("net.minecraft.client.Minecraft", "k", "()V"),
    "default_inventory_binding_constructor": ("kv", "<init>", "(Lnet/minecraft/client/Minecraft;Ljava/io/File;)V"),
    "inventory_screen_constructor": ("ue", "<init>", "(Lgs;)V"),
    "inventory_panel_draw": ("ue", "a", "(F)V"),
    "container_dimensions": ("id", "<init>", "(Ldw;)V"),
    "player_container_slots": ("aa", "<init>", "(Lix;Z)V"),
    "slot_icon_draw": ("id", "a", "(Lgp;)V"),
    "stack_item_lookup": ("iz", "a", "()Lgm;"),
}
EXPECTED_LAYOUT = {
    "size": [176, 166],
    "origin": ["(scaled_width-176)/2", "(scaled_height-166)/2"],
    "uv_origin": [0, 0],
    "uv_size": [176, 166],
    "atlas_size": [256, 256],
}
EXPECTED_BEHAVIOR = {
    "open": "On key-down, when no GUI screen is active, Minecraft compares the event key with the configured key.inventory binding and opens ue (GuiInventory); the default binding is E/keycode 18.",
    "close": "While a GUI screen is active, key events go to that screen. Base da.a(char,int) closes on Escape/keycode 1 and restores mouse capture. E is not a close key in the native inventory screen.",
    "key_repeat": "No repeat-specific filter in Minecraft.k(); GUI dispatch prevents a repeated E event from reopening/toggling while a screen is active.",
}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def load_fixture(path=FIXTURE):
    try:
        fixture = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read valid inventory UI fixture: {error}") from error
    if not isinstance(fixture, dict) or fixture.get("schema_version") != 1:
        raise ValueError("unsupported inventory UI fixture schema")
    reference = fixture.get("reference", {})
    if reference.get("jar_sha256") != PINNED_JAR_SHA256 or reference.get("jar_size") != PINNED_JAR_SIZE:
        raise ValueError("fixture does not match the pinned client JAR")
    if reference.get("jar_source") != "PrismLauncher local cache; verified against local Prism metadata":
        raise ValueError("fixture client JAR provenance was altered")
    resources = {name: {"sha256": h, "bytes": n, "width": w, "height": hgt}
                 for name, (h, n, w, hgt) in PINNED_RESOURCES.items()}
    if reference.get("resources") != resources or reference.get("classes") != PINNED_CLASSES:
        raise ValueError("fixture resource or class hashes differ from pinned native evidence")
    if fixture.get("behavior") is None or {k: fixture["behavior"].get(k) for k in EXPECTED_BEHAVIOR} != EXPECTED_BEHAVIOR:
        raise ValueError("fixture key behavior differs from reviewed native behavior")
    panel = fixture["behavior"].get("inventory_panel")
    if panel != EXPECTED_LAYOUT:
        raise ValueError("fixture panel dimensions/origin/UV evidence was altered")
    slots = fixture["behavior"].get("main_slots", {})
    expected_positions = [[8 + 18 * col, 84 + 18 * row] for row in range(3) for col in range(9)]
    expected_positions += [[8 + 18 * col, 142] for col in range(9)]
    if (slots.get("slot_count") != 36 or slots.get("positions") != expected_positions or
            slots.get("slot_size") != [16, 16] or slots.get("pitch") != [18, 18]):
        raise ValueError("fixture main inventory slot geometry was altered")
    mapping = fixture["behavior"].get("item_icons", {})
    wanted_mapping = {str(item): {"native_item_class": cls, "tile_index": tile,
                                  "uv_origin": [(tile % 16) * 16, (tile // 16) * 16]}
                      for item, cls, tile in EXPECTED_ICONS}
    if mapping.get("atlas") != "/gui/items.png" or mapping.get("tile_size") != [16, 16] or mapping.get("mapping") != wanted_mapping:
        raise ValueError("fixture supported item icon mapping was altered")
    methods = reference.get("native_methods", {})
    if set(methods) != set(EXPECTED_METHODS):
        raise ValueError("fixture native method inventory is incomplete or altered")
    for label, (class_name, name, descriptor) in EXPECTED_METHODS.items():
        method = methods[label]
        if (method.get("class"), method.get("name"), method.get("descriptor")) != (class_name, name, descriptor):
            raise ValueError(f"fixture native method provenance was altered: {label}")
    if fixture.get("runtime_oracle", {}).get("expected_icon_rows") != EXPECTED_ICONS:
        raise ValueError("fixture runtime icon oracle rows were altered")
    evidence = fixture.get("bytecode_evidence")
    if not isinstance(evidence, dict) or set(evidence) != EXPECTED_EVIDENCE_KEYS:
        raise ValueError("fixture bytecode evidence is incomplete or has unknown claims")
    evidence_hash = sha256(json.dumps(evidence, sort_keys=True, separators=(",", ":")).encode())
    if evidence_hash != EXPECTED_EVIDENCE_SHA256:
        raise ValueError("fixture bytecode evidence was altered")
    return fixture


def default_jar():
    try:
        config = tomllib.loads((ROOT / "config/local.toml").read_text(encoding="utf-8"))
        candidate = Path(config["paths"]["client_jar"])
    except (OSError, KeyError, ValueError) as error:
        raise ValueError(f"config/local.toml must provide paths.client_jar: {error}") from error
    return candidate if candidate.is_absolute() else ROOT / candidate


def verify(jar_path, fixture_path=FIXTURE, work_dir=None):
    fixture = load_fixture(fixture_path)
    jar = Path(jar_path)
    jar_bytes = jar.read_bytes()
    if len(jar_bytes) != PINNED_JAR_SIZE or sha256(jar_bytes) != PINNED_JAR_SHA256:
        raise ValueError("client JAR hash/size differs from the pinned Beta 1.7.3 client")
    try:
        with zipfile.ZipFile(jar) as archive:
            for name, expected in PINNED_RESOURCES.items():
                data = archive.read(name)
                if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
                    raise ValueError(f"{name} has invalid PNG signature/IHDR")
                actual = (sha256(data), len(data), *struct.unpack(">II", data[16:24]))
                if actual != expected:
                    raise ValueError(f"{name} resource hash/size/dimensions differ: {actual}")
            for class_name, digest in PINNED_CLASSES.items():
                if sha256(archive.read(class_name.replace(".", "/") + ".class")) != digest:
                    raise ValueError(f"native class hash differs for {class_name}")
    except (zipfile.BadZipFile, KeyError) as error:
        raise ValueError(f"pinned client JAR is missing a required resource/class: {error}") from error

    javap = shutil.which("javap")
    javac = shutil.which("javac")
    java = shutil.which("java")
    if not javap or not javac or not java:
        raise ValueError("javap, javac, and java are required for the native inventory audit")
    # Each class is disassembled from the hash-pinned archive; use the fixture's
    # short excerpts as mutation-sensitive claims, with signatures checked too.
    classes = sorted({entry[0] for entry in EXPECTED_METHODS.values()} | {"aa", "kv", "qb", "ue", "id", "da"})
    outputs = {}
    for class_name in classes:
        result = subprocess.run([javap, "-classpath", str(jar), "-p", "-s", "-c", class_name],
                                capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise ValueError(f"javap rejected native class {class_name}: {result.stderr.strip()}")
        outputs[class_name] = result.stdout
    for label, excerpt in fixture["bytecode_evidence"].items():
        class_for_evidence = {
            "open_screen": "net.minecraft.client.Minecraft", "default_e_binding": "kv",
            "escape_closes_and_grabs_mouse": "da", "panel_dimensions": "id",
            "panel_center_and_uv_draw": "ue", "panel_uv_origin": "ue",
            "inventory_slot_grid": "aa", "hotbar_slot_row": "aa",
            "native_item_atlas_sampling": "id", "native_item_stack_icon_index": "id",
            "gui_texture": "ue", "items_texture": "id",
        }.get(label)
        if not class_for_evidence or " ".join(excerpt.split()) not in " ".join(outputs[class_for_evidence].split()):
            raise ValueError(f"native bytecode evidence not found or unknown: {label}")
    for label, (class_name, name, descriptor) in EXPECTED_METHODS.items():
        output = outputs[class_name]
        if f"descriptor: {descriptor}" not in output:
            raise ValueError(f"native method descriptor missing: {label} {descriptor}")

    if work_dir is None:
        import tempfile
        tmp = tempfile.TemporaryDirectory(prefix="inventory-ui-audit-")
        work = Path(tmp.name)
    else:
        tmp = None
        work = Path(work_dir)
        work.mkdir(parents=True, exist_ok=True)
    try:
        classes_dir = work / "classes"
        classes_dir.mkdir(exist_ok=True)
        compiled = subprocess.run([javac, "-cp", str(jar), "-d", str(classes_dir), str(ORACLE)],
                                  capture_output=True, text=True, timeout=30)
        if compiled.returncode:
            raise ValueError(f"inventory UI oracle compilation failed: {compiled.stderr.strip()}")
        result = subprocess.run([java, "-cp", f"{classes_dir}:{jar}", "InventoryUiOracle", str(jar)],
                                capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise ValueError(f"native inventory icon oracle failed: {result.stderr.strip()}")
        rows = [line.split("\t") for line in result.stdout.splitlines()]
        if len(rows) != 6 or rows[0] != ["jar_sha256", PINNED_JAR_SHA256] or Path(rows[1][1]).resolve() != jar.resolve():
            raise ValueError("native inventory oracle did not load classes from the pinned client JAR")
        actual_icons = [[int(row[1]), row[2], int(row[3])] for row in rows[2:]]
        if actual_icons != EXPECTED_ICONS or actual_icons != fixture["runtime_oracle"]["expected_icon_rows"]:
            raise ValueError(f"native supported item icon mapping differs: {actual_icons}")
    finally:
        if tmp is not None:
            tmp.cleanup()
    return fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, default=None, help="pinned Beta 1.7.3 client JAR")
    parser.add_argument("--fixture", type=Path, default=FIXTURE)
    parser.add_argument("--work-dir", type=Path, default=None)
    args = parser.parse_args()
    try:
        fixture = verify(args.jar or default_jar(), args.fixture, args.work_dir)
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("PASS: pinned inventory resources, GuiInventory key/panel/36-slot bytecode, and native item icon indices")
    print(f"     panel={fixture['behavior']['inventory_panel']['size']}; slots=36; IDs=1,3,332,267")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
