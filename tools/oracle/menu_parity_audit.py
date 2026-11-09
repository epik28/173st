#!/usr/bin/env python3
"""Verify the pinned Beta 1.7.3 menu, world-list, font, and GUI-scale audit."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures/menu_parity"
FIXTURE = FIXTURES / "audit.json"
INPUT = FIXTURES / "scaled_resolution.tsv"
EXPECTED_NATIVE = FIXTURES / "native.tsv"
ORACLE = ROOT / "tools/oracle/src/MenuParityOracle.java"
PINNED_JAR = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
PINNED_JAR_SIZE = 1465375
EXPECTED_NATIVE_SHA256 = "b403287e07085e8b7b04e76ccec3706e15c7d198571ad628f745d1e858bb370d"
PINNED_CLASSES = {
    "net.minecraft.client.Minecraft": "db99df969e5f30d01ad4424a4d63d3b1f4a11832055dd291e65d476cfa4c38b3",
    "fu": "baa1bc76a2451de306229c87a2964b48abf09c0b16353c25f1bb4cdcff79cb2c",
    "rq": "7f3fdc2729f5895cf414601075fa847d61bb7a719f73d60a39c69bf97a16c90f",
    "mg": "0b02e04a20ed97e80a01c8733d13c58937d193a328adaaa568a5a5e53958b4fe",
    "lg": "547d1b6de1394ed7613effd9ba35ce5e5d89dafe48baea43820df004ab59caff",
    "da": "55349b76fb6119d1ea841fb3488b19b387a32abae0b2ce94b5e11706cc4b6890",
    "ke": "579a6c6f21a32752f3debc8d17f66abba9a7581f288f0e06603930ff7efef8d9",
    "qq": "10c63e03905247af1fdc0fd6522e0069948a7735e87a2f8a5d0db65d3cbf03f1",
    "kv": "fd62ebcc36e2f2a1428cb41b2f1f8ebd86fee9855ec1b7f7055ba307d6577102",
    "sj": "c1bec862504ca9b390810ac6eb4e3401032614bf24560757ffb11533ddb66ad3",
    "nh": "31a2a0650ae239f216faf7c26b45c94b3db07964b453a40baea6ee408ed3e9a4",
    "ub": "b06f1cd573e3564c3b6405828a6654218bb9807c694f5a45acaca70f59efb670",
    "vb": "ff8c4833109f9bd4439ebc4ddb718ad303fec9074a2f66a744f06de754d169a9",
}
PINNED_RESOURCES = {
    "gui/background.png": ("416becee17bd2b22999e439e1560aff906f5fc876f47ba53e1001c548a320185", 1011, 16, 16),
    "gui/gui.png": ("e223bea0b5004a3b5497607c6d529daf840396526a46eff75c299364cf9dc795", 14210, 256, 256),
    "font/default.png": ("6ded9b9e06a61beaa61bf8542c46b5d690d45fdaf42e6539afcbc02e4c7a6408", 3667, 128, 128),
    "title/mclogo.png": ("1ec37872aa65aa3f73b327db488472f6a301269b1a458e0d93108b652a487a82", 5410, 256, 256),
    "lang/en_US.lang": ("15309f7acb72e9d08b81cf8ef0bfda31e127112cd506eea2e268a4348efeb68a", 15969, 0, 0),
}
EXPECTED_LABELS = {
    "menu.singleplayer": "Singleplayer",
    "menu.multiplayer": "Multiplayer",
    "menu.mods": "Mods and Texture Packs",
    "menu.options": "Options...",
    "menu.quit": "Quit Game",
    "selectWorld.title": "Select World",
    "selectWorld.select": "Play Selected World",
    "selectWorld.rename": "Rename",
    "selectWorld.delete": "Delete",
    "selectWorld.create": "Create New World",
    "gui.cancel": "Cancel",
    "selectWorld.world": "World",
    "selectWorld.conversion": "Must be converted!",
}
EXPECTED_LAYOUT = {
    "gui_scale_values": {"auto": 0, "small": 1, "normal": 2, "large": 3},
    "scaled_resolution": {
        "minimum_scaled_size": [320, 240],
        "maximum_scale_factor": 1000,
        "pixel_dimensions": "ceil(display_dimension / scale_factor)",
        "logical_dimensions": "display_dimension / scale_factor as double",
    },
    "main_menu": {
        "background": "/gui/background.png tiled at 32px by da",
        "logo": {"resource": "/title/mclogo.png", "screen": ["width/2-137", 30], "draw_parts": [[0, 0, 155, 44], [155, 45, 155, 44]]},
        "primary_button": {"x": "width/2-100", "y": "height/4+48", "size": [200, 20], "rows": {"singleplayer": 0, "multiplayer": 24, "mods": 48}},
        "conditional_bottom_buttons": {"minecraft.n_false": {"y": "height/4+132", "left_x": "width/2-100", "right_x": "width/2+2", "size": [98, 20], "keys": ["menu.options", "menu.quit"]}, "minecraft.n_true": {"y": "height/4+120", "x": "width/2-100", "size": [200, 20], "keys": ["menu.options"]}},
    },
    "world_select": {
        "title": {"anchor": ["width/2", 20], "color": 16777215},
        "list": {"left": 32, "top": 32, "bottom": "height-64", "row_height": 36, "short_content_alignment": "lg.d(): max_scroll=content_height-(bottom-top-4); if negative divide by 2 with Java integer truncation; initial_scroll=min(0,max_scroll); first_row_top=top+4-initial_scroll", "text_y": ["row_y+1", "row_y+12"], "primary_color": 16777215, "secondary_color": 8421504,
                 "selection_outline": {"condition": "list enabled and selected index equals row index", "outer_x": ["width/2-110", "width/2+110"], "outer_y_edges": ["row_y-2", "row_y+row_height-2"], "outer_color": 8421504, "inner_x": ["width/2-109", "width/2+109"], "inner_y_edges": ["row_y-1", "row_y+row_height-3"], "inner_color": 16777215}},
        "buttons": [
            {"key": "selectWorld.select", "id": 1, "x": "width/2-154", "y": "height-52", "size": [150, 20]},
            {"key": "selectWorld.rename", "id": 6, "x": "width/2-154", "y": "height-28", "size": [70, 20]},
            {"key": "selectWorld.delete", "id": 2, "x": "width/2-74", "y": "height-28", "size": [70, 20]},
            {"key": "selectWorld.create", "id": 3, "x": "width/2+4", "y": "height-52", "size": [150, 20]},
            {"key": "gui.cancel", "id": 0, "x": "width/2+4", "y": "height-28", "size": [150, 20]},
        ],
        "selected_world_actions_enabled": ["selectWorld.select", "selectWorld.rename", "selectWorld.delete"],
        "input": {"list_double_click": "same row within 250ms calls select/load by world index", "play_button": "id 1 calls select/load only for selected row", "enter_key": "rq has no key override; Enter does not select/load a world", "escape": "inherited da Escape calls Minecraft.a(null), which selects fu main menu when world is null, then invokes Minecraft.g() mouse handling", "cancel": "id 0 restores parent screen"},
    },
    "button": {
        "atlas": "/gui/gui.png (256x256)",
        "default_size": [200, 20],
        "draw": "two horizontal halves; UV x 0 and 200-width/2; UV y 46+state*20",
        "state_values": {"disabled": 0, "normal": 1, "hover": 2},
        "state_text_color": {"disabled": -6250336, "normal": 14737632, "hover": 16777120},
        "disabled_button_not_hit_tested": True,
    },
    "font": {
        "resource": "/font/default.png",
        "atlas": [128, 128],
        "cell": [8, 8],
        "glyph_count": 256,
        "row_height": 8,
        "glyph_metric": "scan right-to-left from column 7 for any nonzero alpha in 8 rows; width=rightmost+2; space rightmost forced to 2",
        "string_width": "sum metrics for characters in fp.a; skip section sign and its following char; unsupported chars contribute zero",
    },
}
EXPECTED_INPUT = [
    "scale\t854\t480\t0", "scale\t854\t480\t1", "scale\t854\t480\t2", "scale\t854\t480\t3",
    "scale\t1280\t720\t0", "scale\t1280\t720\t1", "scale\t1280\t720\t2", "scale\t1280\t720\t3",
    "scale\t1280\t800\t2",
]


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def default_jar():
    local_config = ROOT / "config/local.toml"
    if not local_config.exists():
        return ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
    try:
        config = tomllib.loads(local_config.read_text(encoding="utf-8"))
        candidate = Path(config["paths"]["client_jar"])
    except (OSError, KeyError, ValueError) as error:
        raise ValueError(f"config/local.toml must provide paths.client_jar: {error}") from error
    return candidate if candidate.is_absolute() else ROOT / candidate


def load_fixture(path=FIXTURE):
    try:
        fixture = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read menu audit fixture: {error}") from error
    if not isinstance(fixture, dict) or fixture.get("schema_version") != 1:
        raise ValueError("unsupported menu audit fixture schema")
    if fixture.get("layout") != EXPECTED_LAYOUT:
        raise ValueError("frozen menu/font/button/scaling API differs from reviewed native constants")
    if fixture.get("labels") != EXPECTED_LABELS:
        raise ValueError("en_US menu labels differ from the pinned translation strings")
    reference = fixture.get("reference", {})
    if reference.get("jar_sha256") != PINNED_JAR or reference.get("jar_size") != PINNED_JAR_SIZE:
        raise ValueError("fixture is not bound to the pinned Beta 1.7.3 client JAR")
    if reference.get("classes") != PINNED_CLASSES:
        raise ValueError("fixture native GUI class provenance differs")
    wanted_resources = {name: {"sha256": h, "bytes": n, **({"dimensions": [w, hgt]} if w else {})}
                        for name, (h, n, w, hgt) in PINNED_RESOURCES.items()}
    if reference.get("resources") != wanted_resources:
        raise ValueError("fixture resource hashes/dimensions differ from pinned native evidence")
    if reference.get("jar_source") != "PrismLauncher local cache; verified against local Prism metadata":
        raise ValueError("fixture JAR provenance statement changed")
    if fixture.get("native_input") != EXPECTED_INPUT:
        raise ValueError("resolution corpus is incomplete or altered")
    if fixture.get("native_output_sha256") != EXPECTED_NATIVE_SHA256 or sha256(EXPECTED_NATIVE.read_bytes()) != EXPECTED_NATIVE_SHA256:
        raise ValueError("native oracle output hash differs from the saved result fixture")
    if len(EXPECTED_NATIVE.read_text(encoding="utf-8").splitlines()) != 265:
        raise ValueError("native oracle fixture must contain 9 resolution rows and 256 glyph rows")
    evidence = fixture.get("evidence")
    required_classes = {"net.minecraft.client.Minecraft", "fu", "rq", "mg", "lg", "da", "ke", "qq", "kv", "sj", "nh"}
    if not isinstance(evidence, dict) or set(evidence) != required_classes:
        raise ValueError("native bytecode evidence classes are incomplete or altered")
    if evidence != EVIDENCE:
        raise ValueError("saved bytecode excerpts differ from the reviewed native audit requirements")
    return fixture


EVIDENCE = {
    "net.minecraft.client.Minecraft": ["/font/default.png", "class sj", "class qq"],
    "fu": ["/title/mclogo.png", "class rq", "menu.singleplayer", "menu.multiplayer", "menu.mods", "menu.options", "menu.quit", "sipush        274", "sipush        155", "bipush        44", "Minecraft.a:(Lda;)V"],
    "rq": ["selectWorld.title", "selectWorld.select", "selectWorld.rename", "selectWorld.delete", "selectWorld.deleteQuestion", "selectWorld.create", "gui.cancel", "sipush        154", "sipush        150", "bipush        20", "iconst_m1", "Field ke.g:Z", "Field i:Ljava/lang/String;", "Minecraft.a:(Lda;)V"],
    "mg": ["bipush        32", "bipush        36", "bipush        64", "bipush        12", "ldc           #2                  // int 16777215", "ldc           #1                  // int 8421504", "rq.e:(I)V"],
    "lg": ["bipush        110", "Field j:I", "Field h:I", "Field i:I", "Method a:()I", "Method c_:(I)Z", "Field m:F", "long 250l", "int 8421504", "iconst_4", "iconst_2", "glDisable:(I)V", "glEnable:(I)V"],
    "da": ["/gui/background.png", "iconst_1", "Keyboard.getEventKey:()I", "Mouse.getEventX:()I", "Minecraft.a:(Lda;)V", "Minecraft.g:()V"],
    "ke": ["/gui/gui.png", "bipush        46", "bipush        20", "int -6250336", "int 14737632", "int 16777120"],
    "qq": ["Field kv.I:I", "sipush        1000", "sipush        320", "sipush        240", "java/lang/Math.ceil:(D)D"],
    "kv": ["guiScale", "options.guiScale.auto", "options.guiScale.small", "options.guiScale.normal", "options.guiScale.large", "Field I:I", "Field ht.m:Lht;", "iand"],
    "sj": ["getResourceAsStream", "sipush        256", "bipush        16", "bipush        8", "sipush        255", "sipush        167", "Field fp.a:Ljava/lang/String;"],
    "nh": ["/lang/en_US.lang", "/lang/stats_US.lang", "Properties.getProperty"],
}


def verify(jar_path, fixture_path=FIXTURE, work_dir=None):
    fixture = load_fixture(fixture_path)
    jar = Path(jar_path)
    jar_bytes = jar.read_bytes()
    if len(jar_bytes) != PINNED_JAR_SIZE or sha256(jar_bytes) != PINNED_JAR:
        raise ValueError("client JAR hash/size differs from pinned Beta 1.7.3 client")
    try:
        with zipfile.ZipFile(jar) as archive:
            for name, (digest, size, width, height) in PINNED_RESOURCES.items():
                data = archive.read(name)
                actual = (sha256(data), len(data))
                if actual != (digest, size):
                    raise ValueError(f"{name} bytes/hash differ: {actual}")
                if width:
                    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
                        raise ValueError(f"{name} has invalid PNG signature/IHDR")
                    if tuple(int.from_bytes(data[offset:offset + 4], "big") for offset in (16, 20)) != (width, height):
                        raise ValueError(f"{name} dimensions differ")
            locale = archive.read("lang/en_US.lang").decode("utf-8")
            locale_values = {}
            for line in locale.splitlines():
                if not line or line.startswith(("#", "!")) or "=" not in line:
                    continue
                key, value = line.split("=", 1)
                locale_values[key] = value
            if any(locale_values.get(key) != value for key, value in EXPECTED_LABELS.items()):
                raise ValueError("pinned en_US GUI translations differ from the frozen menu labels")
            for name, digest in PINNED_CLASSES.items():
                entry = name.replace("net.minecraft.client.", "net/minecraft/client/") + ".class"
                if sha256(archive.read(entry)) != digest:
                    raise ValueError(f"native class hash differs: {name}")
    except (zipfile.BadZipFile, KeyError) as error:
        raise ValueError(f"pinned client archive lacks required GUI evidence: {error}") from error

    javap = shutil.which("javap")
    javac = shutil.which("javac")
    java = shutil.which("java")
    if not javap or not javac or not java:
        raise ValueError("javap, javac, and java are required for the native GUI audit")
    outputs = {}
    for class_name in EVIDENCE:
        result = subprocess.run([javap, "-classpath", str(jar), "-p", "-s", "-c", class_name],
                                capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise ValueError(f"javap rejected {class_name}: {result.stderr.strip()}")
        outputs[class_name] = result.stdout
    for class_name, snippets in EVIDENCE.items():
        normalized = " ".join(outputs[class_name].split())
        for snippet in snippets:
            if " ".join(snippet.split()) not in normalized:
                raise ValueError(f"native bytecode evidence missing for {class_name}: {snippet}")

    if work_dir is None:
        temporary = tempfile.TemporaryDirectory(prefix="menu-parity-audit-")
        work = Path(temporary.name)
    else:
        temporary = None
        work = Path(work_dir)
        work.mkdir(parents=True, exist_ok=True)
    try:
        classes = work / "classes"
        classes.mkdir(exist_ok=True)
        compile_result = subprocess.run([javac, "-cp", str(jar), "-d", str(classes), str(ORACLE)],
                                        capture_output=True, text=True, timeout=30)
        if compile_result.returncode:
            raise ValueError(f"native menu oracle compile failed: {compile_result.stderr.strip()}")
        input_bytes = INPUT.read_bytes()
        outputs = []
        for _ in range(2):
            process = subprocess.run([java, "-cp", f"{classes}:{jar}", "MenuParityOracle"], input=input_bytes,
                                     capture_output=True, timeout=30)
            if process.returncode:
                raise ValueError(f"native menu oracle failed: {process.stderr.decode(errors='replace').strip()}")
            outputs.append(process.stdout)
        expected = EXPECTED_NATIVE.read_bytes()
        if outputs[0] != outputs[1]:
            raise ValueError("two independent native menu oracle executions differ")
        if outputs[0] != expected:
            raise ValueError("native ScaledResolution or glyph metrics differ from saved C01 oracle fixture")
    finally:
        if temporary is not None:
            temporary.cleanup()
    return fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, default=None)
    parser.add_argument("--fixture", type=Path, default=FIXTURE)
    parser.add_argument("--work-dir", type=Path, default=None)
    args = parser.parse_args()
    try:
        fixture = verify(args.jar or default_jar(), args.fixture, args.work_dir)
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("PASS: pinned Beta 1.7.3 GUI bytecode/resources, 9 native scaled-resolution cases, and 256 derived font glyph metrics")
    print(f"     jar={fixture['reference']['jar_sha256']}; visual/native window capture=not performed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
