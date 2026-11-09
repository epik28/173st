#!/usr/bin/env python3
"""Verify native Beta 1.7.3 hotbar input and default key binding evidence."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/hotbar/audit.json"
PINNED_JAR_SHA256 = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
PINNED_JAR_SIZE = 1465375
PINNED_CLASSES = {
    "net.minecraft.client.Minecraft": "db99df969e5f30d01ad4424a4d63d3b1f4a11832055dd291e65d476cfa4c38b3",
    "ix": "03eb35aef64c71087942c5dc8b2849e9b47e9482c97be825895cfc2149b8e009",
    "kv": "fd62ebcc36e2f2a1428cb41b2f1f8ebd86fee9855ec1b7f7055ba307d6577102",
    "qb": "4c1524aeb93b82f91517c5ed5a3dcce02c1b110a9d65b3db7089cfc8a37cd7aa",
}
EXPECTED_METHODS = {
    "hotbar_event_dispatch": {
        "class": "net.minecraft.client.Minecraft",
        "name": "k",
        "descriptor": "()V",
        "mapped_name": "Minecraft.runTick",
        "mapping_basis": "MCP Beta 1.7.3 name; pinned obfuscated class and descriptor are authoritative",
    },
    "inventory_scroll": {
        "class": "ix",
        "name": "b",
        "descriptor": "(I)V",
        "mapped_name": "Inventory.changeCurrentItem(int)",
        "mapping_basis": "MCP Beta 1.7.3 name; pinned obfuscated class and descriptor are authoritative",
    },
    "default_bindings_constructor": {
        "class": "kv",
        "name": "<init>",
        "descriptor": "(Lnet/minecraft/client/Minecraft;Ljava/io/File;)V",
        "mapped_name": "GameSettings.<init>",
        "mapping_basis": "MCP Beta 1.7.3 name; pinned obfuscated class and descriptor are authoritative",
    },
}
EXPECTED_BEHAVIOR = {
    "number_keys": {
        "input_source": "Keyboard.getEventKey() on a key-down event",
        "physical_codes": list(range(2, 11)),
        "slots": list(range(9)),
        "formula": "slot = key_code - 2",
        "configurable_binding": False,
        "repeat": "key-up events do not select; every key-down event selects its slot",
    },
    "wheel": {
        "event_source": "Mouse.getEventDWheel()",
        "zero_event": "ignored",
        "step_count": "one slot per nonzero mouse event, regardless of delta magnitude",
        "direction": "positive delta decrements selected index; negative delta increments it",
        "wrap": "floor_mod(selected_index - sign(delta), 9)",
        "examples": [
            {"selected": 0, "delta": 120, "result": 8},
            {"selected": 0, "delta": -120, "result": 1},
            {"selected": 8, "delta": -120, "result": 0},
            {"selected": 0, "delta": 240, "result": 8},
        ],
    },
    "default_bindings": {
        "key.forward": {"code": 17, "label": "W"},
        "key.left": {"code": 30, "label": "A"},
        "key.back": {"code": 31, "label": "S"},
        "key.right": {"code": 32, "label": "D"},
        "key.jump": {"code": 57, "label": "Space"},
        "key.inventory": {"code": 18, "label": "E"},
        "key.drop": {"code": 16, "label": "Q"},
        "key.chat": {"code": 20, "label": "T"},
        "key.fog": {"code": 33, "label": "F"},
        "key.sneak": {"code": 42, "label": "Left Shift"},
    },
}
EXPECTED_EVIDENCE = {
    "hotbar_dispatch_method": "public void k();\n    descriptor: ()V",
    "wheel_reads_event_delta": "363: invokestatic #593                // Method org/lwjgl/input/Mouse.getEventDWheel:()I",
    "wheel_ignores_zero": "367: iload_3\n     368: ifeq          421",
    "wheel_calls_inventory_once": "371: aload_0\n     372: getfield #306                // Field h:Ldc;\n     375: getfield #231                // Field dc.c:Lix;\n     378: iload_3\n     379: invokevirtual #435                // Method ix.b:(I)V",
    "keyboard_key_down_gate": "572: invokestatic #586                // Method org/lwjgl/input/Keyboard.getEventKeyState:()Z\n     575: ifeq          553",
    "number_key_loop_bounds": "849: iconst_0\n     850: istore_1\n     851: iload_1\n     852: bipush        9\n     854: if_icmpge     883",
    "number_key_mapping_and_assignment": "857: invokestatic #585                // Method org/lwjgl/input/Keyboard.getEventKey:()I\n     860: iconst_2\n     861: iload_1\n     862: iadd\n     863: if_icmpne     877\n     866: aload_0\n     867: getfield #306                // Field h:Ldc;\n     870: getfield #231                // Field dc.c:Lix;\n     873: iload_1\n     874: putfield #242                // Field ix.c:I",
    "inventory_scroll_sign_and_wrap": "public void b(int);\n    descriptor: (I)V\n    Code:\n       0: iload_1\n       1: ifle          6\n       4: iconst_1\n       5: istore_1\n       6: iload_1\n       7: ifge          12\n      10: iconst_m1\n      11: istore_1\n      12: aload_0\n      13: dup\n      14: getfield #19                 // Field c:I\n      17: iload_1\n      18: isub\n      19: putfield #19                 // Field c:I",
    "inventory_wrap_negative": "22: aload_0\n      23: getfield #19                 // Field c:I\n      26: ifge          43\n      29: aload_0\n      30: dup\n      31: getfield #19                 // Field c:I\n      34: bipush 9\n      36: iadd\n      37: putfield #19                 // Field c:I",
    "inventory_wrap_upper": "43: aload_0\n      44: getfield #19                 // Field c:I\n      47: bipush        9\n      49: if_icmplt     66\n      52: aload_0\n      53: dup\n      54: getfield #19                 // Field c:I\n      57: bipush        9\n      59: isub\n      60: putfield      #19                 // Field c:I",
}
EXPECTED_DEFAULT_BINDING_EVIDENCE = {
    "key.forward": "ldc           #35                 // String key.forward\n      73: bipush        17\n      75: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.left": "ldc           #38                 // String key.left\n      88: bipush        30\n      90: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.back": "ldc           #31                 // String key.back\n     103: bipush        31\n     105: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.right": "ldc           #39                 // String key.right\n     118: bipush        32\n     120: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.jump": "ldc           #37                 // String key.jump\n     133: bipush        57\n     135: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.inventory": "ldc           #36                 // String key.inventory\n     148: bipush        18\n     150: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.drop": "ldc           #33                 // String key.drop\n     163: bipush        16\n     165: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.chat": "ldc           #32                 // String key.chat\n     178: bipush        20\n     180: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.fog": "ldc           #34                 // String key.fog\n     193: bipush        33\n     195: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
    "key.sneak": "ldc           #40                 // String key.sneak\n     208: bipush        42\n     210: invokespecial #199                // Method qb.\"<init>\":(Ljava/lang/String;I)V",
}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def normalized(value):
    """javap's column padding varies with descriptor widths; keep tokens exact."""
    return " ".join(value.split())


def load_fixture(path=FIXTURE):
    try:
        fixture = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read valid hotbar audit fixture: {error}") from error
    if not isinstance(fixture, dict) or fixture.get("schema_version") != 1:
        raise ValueError("unsupported hotbar audit fixture schema")
    reference = fixture.get("reference")
    if not isinstance(reference, dict):
        raise ValueError("fixture reference must be an object")
    if reference.get("jar_sha256") != PINNED_JAR_SHA256 or reference.get("jar_size") != PINNED_JAR_SIZE:
        raise ValueError("fixture does not match the pinned client JAR")
    if reference.get("jar_source") != "PrismLauncher local cache; verified against local Prism metadata":
        raise ValueError("fixture client JAR provenance was altered")
    if reference.get("classes") != PINNED_CLASSES:
        raise ValueError("fixture class hashes differ from pinned native classes")
    if reference.get("native_methods") != EXPECTED_METHODS:
        raise ValueError("fixture native method provenance is malformed or changed")
    if fixture.get("behavior") != EXPECTED_BEHAVIOR:
        raise ValueError("fixture behavior evidence differs from the reviewed native behavior")
    fixture_evidence = fixture.get("bytecode_evidence")
    if not isinstance(fixture_evidence, dict) or fixture_evidence.keys() != EXPECTED_EVIDENCE.keys() or any(
        normalized(fixture_evidence[key]) != normalized(value) for key, value in EXPECTED_EVIDENCE.items()
    ):
        raise ValueError("fixture bytecode evidence was altered")
    binding_evidence = fixture.get("default_binding_bytecode")
    if not isinstance(binding_evidence, dict) or binding_evidence.keys() != EXPECTED_DEFAULT_BINDING_EVIDENCE.keys() or any(
        normalized(binding_evidence[key]) != normalized(value)
        for key, value in EXPECTED_DEFAULT_BINDING_EVIDENCE.items()
    ):
        raise ValueError("fixture default binding bytecode evidence was altered")
    return fixture


def default_jar():
    try:
        config = tomllib.loads((ROOT / "config/local.toml").read_text(encoding="utf-8"))
        path = Path(config["paths"]["client_jar"])
    except (OSError, KeyError, ValueError) as error:
        raise ValueError(f"config/local.toml must provide paths.client_jar: {error}") from error
    return path if path.is_absolute() else ROOT / path


def verify(jar_path, fixture_path=FIXTURE):
    fixture = load_fixture(fixture_path)
    jar_bytes = Path(jar_path).read_bytes()
    if len(jar_bytes) != PINNED_JAR_SIZE or sha256(jar_bytes) != PINNED_JAR_SHA256:
        raise ValueError("client JAR hash/size differs from the pinned Beta 1.7.3 client")
    try:
        with zipfile.ZipFile(jar_path) as archive:
            for class_name, expected_hash in PINNED_CLASSES.items():
                entry = class_name.replace(".", "/") + ".class"
                try:
                    class_bytes = archive.read(entry)
                except KeyError as error:
                    raise ValueError(f"pinned client JAR is missing class {entry}") from error
                if sha256(class_bytes) != expected_hash:
                    raise ValueError(f"native class hash differs for {class_name}")
    except zipfile.BadZipFile as error:
        raise ValueError(f"pinned client JAR is not a valid archive: {error}") from error

    javap = shutil.which("javap")
    if not javap:
        raise ValueError("javap is required to verify native input method provenance")
    outputs = {}
    for class_name in PINNED_CLASSES:
        result = subprocess.run(
            [javap, "-classpath", str(jar_path), "-p", "-s", "-c", class_name],
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode:
            raise ValueError(f"javap rejected pinned class {class_name}: {result.stderr.strip()}")
        outputs[class_name] = result.stdout

    evidence_to_class = {name: "net.minecraft.client.Minecraft" for name in EXPECTED_EVIDENCE}
    evidence_to_class.update({
        "inventory_scroll_sign_and_wrap": "ix",
        "inventory_wrap_negative": "ix",
        "inventory_wrap_upper": "ix",
    })
    for label, snippet in EXPECTED_EVIDENCE.items():
        if normalized(snippet) not in normalized(outputs[evidence_to_class[label]]):
            raise ValueError(f"native bytecode evidence not found for {label}")
    kv = outputs["kv"]
    for binding, snippet in EXPECTED_DEFAULT_BINDING_EVIDENCE.items():
        if normalized(snippet) not in normalized(kv):
            raise ValueError(f"native default binding evidence not found for {binding}")
    for token in ("public void k();", "descriptor: ()V", "public void b(int);", "descriptor: (I)V"):
        if token not in outputs["net.minecraft.client.Minecraft"] + outputs["ix"]:
            raise ValueError(f"native method descriptor missing: {token}")
    if "public kv(net.minecraft.client.Minecraft, java.io.File);" not in kv or (
        "descriptor: (Lnet/minecraft/client/Minecraft;Ljava/io/File;)V" not in kv
    ):
        raise ValueError("native default key binding constructor descriptor missing")
    return fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, default=None, help="pinned Beta 1.7.3 client JAR")
    parser.add_argument("--fixture", type=Path, default=FIXTURE)
    args = parser.parse_args()
    try:
        verify(args.jar or default_jar(), args.fixture)
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: pinned Beta 1.7.3 JAR/classes; hardcoded number keys 1–9; "
        "one sign-directed wrapped hotbar step per nonzero wheel event; default bindings"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
