#!/usr/bin/env python3
"""Launch the pinned Beta 1.7.3 client in a disposable home and capture GUI screens.

All writes are confined to paths beneath this repository's ignored artifacts/
directory. Existing Prism data is read-only; this script never downloads or
opens a playable world.
"""

import argparse
import ctypes
import ctypes.util
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import time
import zipfile


ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "artifacts"
PRISM = Path("/home/andrew/.local/share/PrismLauncher")
DEFAULT_JAR = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
JAVA8 = PRISM / "java/jre-legacy/bin/java"
LIBRARIES = [
    PRISM / "libraries/org/lwjgl/lwjgl/lwjgl/2.9.4-nightly-20150209/lwjgl-2.9.4-nightly-20150209.jar",
    PRISM / "libraries/org/lwjgl/lwjgl/lwjgl_util/2.9.4-nightly-20150209/lwjgl_util-2.9.4-nightly-20150209.jar",
    PRISM / "libraries/net/java/jinput/jinput/2.0.5/jinput-2.0.5.jar",
    PRISM / "libraries/net/java/jutils/jutils/1.0.0/jutils-1.0.0.jar",
]
NATIVE_JARS = [
    PRISM / "libraries/org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209-natives-linux.jar",
    PRISM / "libraries/net/java/jinput/jinput-platform/2.0.5/jinput-platform-2.0.5-natives-linux.jar",
]
PINNED_CLIENT_SHA256 = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
SYNTHETIC_SAVE_FIXTURE = ROOT / "tests/fixtures/menu_parity/synthetic_saves.json"
SYNTHETIC_SAVE_FIXTURE_SHA256 = "09a415e91dba34a43ab385fc4e9eeead0a7215ef7fbe75728b1d971ef4e21496"
EXPECTED_NATIVE_NAMES = {
    "liblwjgl.so", "liblwjgl64.so", "libopenal.so", "libopenal64.so",
    "libjinput-linux.so", "libjinput-linux64.so",
}
PROFILES = [
    {"name": "854x480-gui1", "width": 854, "height": 480, "gui_scale": 1, "scale_factor": 1,
     "scaled_width": 854, "scaled_height": 480},
    {"name": "1280x720-gui3", "width": 1280, "height": 720, "gui_scale": 3, "scale_factor": 3,
     "scaled_width": 427, "scaled_height": 240},
]


def digest(path):
    value = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def inside_artifacts(path, label):
    resolved = Path(path).resolve()
    if not resolved.is_relative_to(ARTIFACTS.resolve()):
        raise ValueError(f"{label} must be inside {ARTIFACTS}")
    return resolved


def checked_command(command, timeout=15, allow_stderr=False):
    result = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise ValueError(f"command failed ({result.returncode}): {' '.join(map(str, command))}\n{result.stderr.strip()}")
    output = result.stdout.strip()
    if allow_stderr:
        output = "\n".join(part for part in (output, result.stderr.strip()) if part)
    return output


def tag(tag_type, name, payload):
    encoded = name.encode("utf-8")
    return bytes([tag_type]) + struct.pack(">H", len(encoded)) + encoded + payload


def nbt_level_dat(metadata):
    children = bytearray()
    for name in ("RandomSeed", "LastPlayed", "SizeOnDisk"):
        children.extend(tag(4, name, struct.pack(">q", metadata[name])))
    for name in ("SpawnX", "SpawnY", "SpawnZ", "version"):
        children.extend(tag(3, name, struct.pack(">i", metadata[name])))
    encoded_name = metadata["LevelName"].encode("utf-8")
    children.extend(tag(8, "LevelName", struct.pack(">H", len(encoded_name)) + encoded_name))
    children.append(0)  # End Data compound.
    raw = b"\x0a\x00\x00" + tag(10, "Data", bytes(children)) + b"\x00"
    return gzip.compress(raw, mtime=0)


def create_synthetic_saves(saves_dir):
    fixture_bytes = SYNTHETIC_SAVE_FIXTURE.read_bytes()
    if hashlib.sha256(fixture_bytes).hexdigest() != SYNTHETIC_SAVE_FIXTURE_SHA256:
        raise ValueError("synthetic save fixture hash differs from the reviewed native contract")
    fixture = json.loads(fixture_bytes.decode("utf-8"))
    if fixture.get("schema_version") != 1:
        raise ValueError("unsupported synthetic save fixture schema")
    files = {}
    for case in fixture["cases"]:
        directory = saves_dir / case["directory"]
        directory.mkdir(parents=True, exist_ok=True)
        primary = directory / case["file"]
        payload = b"corrupt synthetic metadata; not a world" if case.get("corrupt") else nbt_level_dat(case["metadata"])
        write_disposable_file(primary, payload)
        files[str(primary)] = hashlib.sha256(payload).hexdigest()
        backup = case.get("backup")
        if backup:
            backup_path = directory / backup["file"]
            backup_payload = nbt_level_dat(backup["metadata"])
            write_disposable_file(backup_path, backup_payload)
            files[str(backup_path)] = hashlib.sha256(backup_payload).hexdigest()
    expected_dirs = {case["directory"] for case in fixture["cases"]}
    if {path.name for path in saves_dir.iterdir()} != expected_dirs:
        raise ValueError("synthetic disposable saves root contains unexpected entries")
    for case in fixture["cases"]:
        expected_files = {case["file"]}
        if case.get("backup"):
            expected_files.add(case["backup"]["file"])
        actual_files = {path.name for path in (saves_dir / case["directory"]).iterdir()}
        if actual_files != expected_files:
            raise ValueError(f"synthetic disposable world has unexpected contents: {case['directory']}")
    return fixture, files


def write_disposable_file(path, payload):
    if path.exists():
        if path.read_bytes() != payload:
            raise ValueError(f"disposable synthetic save path already contains different bytes: {path}")
    else:
        path.write_bytes(payload)


def prepare(runtime, jar_path, save_set):
    runtime = inside_artifacts(runtime, "runtime")
    jar_path = Path(jar_path).resolve()
    if digest(jar_path) != PINNED_CLIENT_SHA256:
        raise ValueError("client JAR differs from the pinned Beta 1.7.3 artifact")
    if not JAVA8.is_file() or not os.access(JAVA8, os.X_OK):
        raise ValueError(f"Prism Java 8 executable is missing: {JAVA8}")
    for path in LIBRARIES + NATIVE_JARS:
        if not path.is_file():
            raise ValueError(f"required local Prism library is missing: {path}")
    for command in ("xwininfo", "import"):
        if not shutil.which(command):
            raise ValueError(f"required local capture utility is missing: {command}")
    if not ctypes.util.find_library("X11") or not ctypes.util.find_library("Xtst"):
        raise ValueError("local X11 and Xtst libraries are required for input/window automation")

    runtime.mkdir(parents=True, exist_ok=True)
    native_dir = runtime / "natives"
    native_dir.mkdir(exist_ok=True)
    found_native_names = set()
    native_hashes = {}
    for native_jar in NATIVE_JARS:
        with zipfile.ZipFile(native_jar) as archive:
            for name in archive.namelist():
                leaf = Path(name).name
                if not leaf.endswith(".so"):
                    continue
                if leaf not in EXPECTED_NATIVE_NAMES:
                    raise ValueError(f"unexpected native library in pinned Prism archive: {leaf}")
                found_native_names.add(leaf)
                target = native_dir / leaf
                payload = archive.read(name)
                if target.exists():
                    if target.read_bytes() != payload:
                        raise ValueError(f"disposable native already exists with different bytes: {target}")
                else:
                    target.write_bytes(payload)
                    target.chmod(0o700)
                native_hashes[leaf] = hashlib.sha256(payload).hexdigest()
    if found_native_names != EXPECTED_NATIVE_NAMES:
        raise ValueError(f"native archive contents differ: {sorted(found_native_names)}")

    game_home = runtime / "home"
    minecraft_home = game_home / ".minecraft"
    game_cwd = runtime / "cwd"
    (minecraft_home / "saves").mkdir(parents=True, exist_ok=True)
    game_cwd.mkdir(exist_ok=True)
    synthetic_fixture, synthetic_files = (None, {})
    if save_set == "synthetic":
        synthetic_fixture, synthetic_files = create_synthetic_saves(minecraft_home / "saves")
    elif any((minecraft_home / "saves").iterdir()):
        raise ValueError("empty save set requested but disposable saves directory is not empty")
    java_version = checked_command([str(JAVA8), "-version"], timeout=10, allow_stderr=True)
    classpath = os.pathsep.join(map(str, [jar_path] + LIBRARIES))
    return {
        "runtime": runtime,
        "jar": jar_path,
        "native_dir": native_dir,
        "game_home": game_home,
        "minecraft_home": minecraft_home,
        "game_cwd": game_cwd,
        "classpath": classpath,
        "java_version": java_version,
        "save_set": save_set,
        "synthetic_save_fixture": synthetic_fixture,
        "synthetic_save_files_sha256": synthetic_files,
        "client_sha256": PINNED_CLIENT_SHA256,
        "library_sha256": {str(path): digest(path) for path in LIBRARIES + NATIVE_JARS},
        "native_sha256": native_hashes,
    }


def window_candidates():
    output = subprocess.run(["xwininfo", "-root", "-tree"], capture_output=True, text=True, timeout=10)
    if output.returncode:
        return []
    candidates = []
    for line in output.stdout.splitlines():
        if "Minecraft" not in line:
            continue
        match = re.search(r"(0x[0-9a-fA-F]+)\s+\"([^\"]*)\"\s*:?(?:\s+\([^)]*\))*\s+(\d+)x(\d+)([+-]\d+)([+-]\d+)", line)
        if not match:
            continue
        window, title, width, height, x, y = match.groups()
        candidates.append({"id": window, "title": title, "x": int(x), "y": int(y),
                           "width": int(width), "height": int(height)})
    candidates.sort(key=lambda item: item["width"] * item["height"], reverse=True)
    return candidates


def await_window(process, timeout_seconds, log_path):
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise ValueError(f"Minecraft exited during startup ({process.returncode}); see {log_path}")
        candidates = window_candidates()
        if candidates:
            return candidates[0]
        time.sleep(0.25)
    tree = subprocess.run(["xwininfo", "-root", "-tree"], capture_output=True, text=True, timeout=10)
    diagnostic_path = log_path.with_suffix(".xwininfo-tree.txt")
    diagnostic_path.write_text(tree.stdout + ("\n[stderr]\n" + tree.stderr if tree.stderr else ""), encoding="utf-8")
    raise TimeoutError(f"Minecraft window did not appear in {timeout_seconds}s; see {log_path} and {diagnostic_path}\n{tree.stdout.strip()}")


def actual_window(window):
    output = checked_command(["xwininfo", "-id", window["id"]], timeout=5)
    width = re.search(r"Width:\s+(\d+)", output)
    height = re.search(r"Height:\s+(\d+)", output)
    if not width or not height:
        raise ValueError(f"cannot read actual X window size for {window['id']}")
    return int(width.group(1)), int(height.group(1))


def gui_to_physical(profile, gui_x, gui_y):
    return (round(gui_x * profile["width"] / profile["scaled_width"]),
            round(gui_y * profile["height"] / profile["scaled_height"]))


def native_first_row_top(scaled_height, row_count, row_height=36):
    """Derive GuiSlot's initial row y from pinned lg.d() short-list centering."""
    top = 32
    bottom = scaled_height - 64
    available = bottom - top - 4
    max_scroll = row_count * row_height - available
    if max_scroll < 0:
        max_scroll = -(abs(max_scroll) // 2)  # Java integer division truncates toward zero.
    initial_scroll = min(0, max_scroll)
    return top + 4 - initial_scroll


def set_window_size(window, width, height):
    window_id = int(window["id"], 16)
    with XSession() as x11:
        x11.lib.XMoveResizeWindow(x11.display, window_id, 0, 0, width, height)
        x11.lib.XRaiseWindow(x11.display, window_id)
        x11.lib.XSetInputFocus(x11.display, window_id, 2, 0)
        x11.lib.XFlush(x11.display)
        x11.lib.XSync(x11.display, 0)
        time.sleep(2.0)
        actual = actual_window(window)
        if actual != (width, height):
            raise ValueError(f"resized native window is {actual[0]}x{actual[1]}, wanted {width}x{height}")
        return actual


class XSession:
    """Small direct Xlib/XTest wrapper; does not launch or configure an X server."""
    def __init__(self):
        self.lib = ctypes.CDLL(ctypes.util.find_library("X11"))
        self.xtst = ctypes.CDLL(ctypes.util.find_library("Xtst"))
        self.display = None

    def __enter__(self):
        self.lib.XOpenDisplay.restype = ctypes.c_void_p
        self.display = self.lib.XOpenDisplay(None)
        if not self.display:
            raise ValueError("cannot connect to DISPLAY")
        self.lib.XMoveResizeWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int,
                                               ctypes.c_int, ctypes.c_uint, ctypes.c_uint]
        self.lib.XRaiseWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
        self.lib.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
        self.lib.XFlush.argtypes = [ctypes.c_void_p]
        self.lib.XSync.argtypes = [ctypes.c_void_p, ctypes.c_int]
        self.xtst.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int,
                                                   ctypes.c_int, ctypes.c_ulong]
        self.xtst.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int,
                                                   ctypes.c_ulong]
        self.xtst.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int,
                                                ctypes.c_ulong]
        self.xtst.XTestFakeMotionEvent.restype = ctypes.c_int
        self.xtst.XTestFakeButtonEvent.restype = ctypes.c_int
        self.xtst.XTestFakeKeyEvent.restype = ctypes.c_int
        self.lib.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
        self.lib.XKeysymToKeycode.restype = ctypes.c_uint
        return self

    def focus_and_click(self, window_id, x, y):
        self.lib.XRaiseWindow(self.display, window_id)
        self.lib.XSetInputFocus(self.display, window_id, 2, 0)
        self.lib.XSync(self.display, 0)
        time.sleep(0.5)
        if not self.xtst.XTestFakeMotionEvent(self.display, -1, x, y, 0):
            raise ValueError("XTest could not move pointer")
        self.lib.XFlush(self.display)
        time.sleep(0.15)
        if not self.xtst.XTestFakeButtonEvent(self.display, 1, 1, 0):
            raise ValueError("XTest could not press mouse button")
        self.lib.XFlush(self.display)
        time.sleep(0.15)
        self.xtst.XTestFakeButtonEvent(self.display, 1, 0, 0)
        self.lib.XFlush(self.display)
        self.lib.XSync(self.display, 0)

    def focus_and_press_key(self, window_id, keysym):
        self.lib.XRaiseWindow(self.display, window_id)
        self.lib.XSetInputFocus(self.display, window_id, 2, 0)
        self.lib.XSync(self.display, 0)
        time.sleep(0.25)
        keycode = self.lib.XKeysymToKeycode(self.display, keysym)
        if not keycode:
            raise ValueError(f"X11 has no keycode for keysym 0x{keysym:x}")
        if not self.xtst.XTestFakeKeyEvent(self.display, keycode, 1, 0):
            raise ValueError("XTest could not press key")
        self.lib.XFlush(self.display)
        time.sleep(0.12)
        self.xtst.XTestFakeKeyEvent(self.display, keycode, 0, 0)
        self.lib.XFlush(self.display)
        self.lib.XSync(self.display, 0)

    def __exit__(self, *_exc):
        if self.display:
            self.lib.XCloseDisplay.argtypes = [ctypes.c_void_p]
            self.lib.XCloseDisplay(self.display)


def write_options(home, gui_scale):
    options_path = home / ".minecraft/options.txt"
    # Deliberately minimal: Beta 1.7.3 applies constructor defaults for every
    # omitted setting; this overrides only the requested native GUI scale.
    options_path.parent.mkdir(parents=True, exist_ok=True)
    options_path.write_text(f"guiScale:{gui_scale}\n", encoding="ascii")
    return options_path


def launch_profile(context, profile, output_dir, timeout_seconds):
    home = context["game_home"]
    options_path = write_options(home, profile["gui_scale"])
    log_path = output_dir / f"{profile['name']}.log"
    argv = [
        str(JAVA8),
        f"-Duser.home={home}",
        f"-Djava.library.path={context['native_dir']}",
        f"-Dorg.lwjgl.librarypath={context['native_dir']}",
        f"-Dnet.java.games.input.librarypath={context['native_dir']}",
        "-Dhttp.proxyHost=127.0.0.1", "-Dhttp.proxyPort=9",
        "-Dhttps.proxyHost=127.0.0.1", "-Dhttps.proxyPort=9",
        "-Djava.net.useSystemProxies=false",
        "-Xms256m", "-Xmx1024m", "-cp", context["classpath"],
        "net.minecraft.client.Minecraft", "MenuParityC01", "-",
    ]
    environment = os.environ.copy()
    environment.pop("WAYLAND_DISPLAY", None)
    environment["ALSOFT_DRIVERS"] = "null"
    process = None
    log = log_path.open("wb")
    started = time.time()
    profile_result = {
        "profile": profile,
        "argv": argv,
        "user_home_property": str(home),
        "game_directory_observed_from_native_code": str(home / ".minecraft"),
        "options_path": str(options_path),
        "options_sha256_before_launch": digest(options_path),
        "log_path": str(log_path),
        "timeout_seconds": timeout_seconds,
        "cleanup": {"terminate_sent": False, "killed": False, "exit_code": None},
    }
    try:
        process = subprocess.Popen(argv, cwd=context["game_cwd"], env=environment,
                                   stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        initial = await_window(process, timeout_seconds, log_path)
        # Give the display and first screen frame time to initialize. This is
        # not a claim of a fixed simulation timestamp; Splash text is random.
        time.sleep(6.0)
        # Place the client at the root origin for both profiles so XTest root
        # coordinates map directly to framebuffer coordinates.
        actual = set_window_size(initial, profile["width"], profile["height"])
        profile_result["initial_window"] = initial
        profile_result["actual_viewport_window"] = {"width": actual[0], "height": actual[1], "window_id": initial["id"]}
        main_path = output_dir / f"{profile['name']}-main.png"
        profile_result["main_frame"] = capture_record(initial["id"], main_path, profile)

        gui_x = profile["scaled_width"] // 2
        gui_y = profile["scaled_height"] // 4 + 48 + 10
        physical_x, physical_y = gui_to_physical(profile, gui_x, gui_y)
        with XSession() as x11:
            x11.focus_and_click(int(initial["id"], 16), physical_x, physical_y)
        time.sleep(4.0)
        list_suffix = "world-list-empty" if context["save_set"] == "empty" else "world-list-initial"
        list_path = output_dir / f"{profile['name']}-{list_suffix}.png"
        profile_result["world_list_initial_frame"] = capture_record(initial["id"], list_path, profile)
        profile_result["singleplayer_click"] = {
            "physical_xy": [physical_x, physical_y], "scaled_gui_xy": [gui_x, gui_y],
            "focus_raised": True, "focus_set": True, "button_down_ms": 150, "button_up_flushed": True,
            "transition_verified": False,
            "note": "Native screen transition requires visual review of the captured PNG."}

        if context["save_set"] == "synthetic":
            list_x = profile["scaled_width"] // 2
            row_count = sum(1 for case in context["synthetic_save_fixture"]["cases"]
                            if case["expected_native_row"])
            list_row_top = native_first_row_top(profile["scaled_height"], row_count)
            list_y = list_row_top + 18
            select_xy = gui_to_physical(profile, list_x, list_y)
            with XSession() as x11:
                x11.focus_and_click(int(initial["id"], 16), *select_xy)
            time.sleep(1.5)
            selected_path = output_dir / f"{profile['name']}-world-list-selected-row.png"
            profile_result["world_list_selected_frame"] = capture_record(initial["id"], selected_path, profile)
            profile_result["world_row_click"] = {"row_index": 0, "row_count": row_count,
                                                  "native_first_row_top_scaled": list_row_top,
                                                  "scaled_gui_xy": [list_x, list_y],
                                                  "physical_xy": list_xy,
                                                  "focus_raised": True, "focus_set": True,
                                                  "button_down_ms": 150, "button_up_flushed": True}

            with XSession() as x11:
                x11.focus_and_press_key(int(initial["id"], 16), 0xff0d)  # XK_Return
            time.sleep(1.5)
            enter_path = output_dir / f"{profile['name']}-world-list-enter-key.png"
            profile_result["world_list_enter_frame"] = capture_record(initial["id"], enter_path, profile)
            profile_result["enter_key_event"] = {"keysym": "Return", "held_ms": 120,
                                                  "expected_native_behavior": "rq has no Enter handler; remains on list"}

            cancel_x = profile["scaled_width"] // 2 + 4 + 75
            cancel_y = profile["scaled_height"] - 28 + 10
            cancel_xy = gui_to_physical(profile, cancel_x, cancel_y)
            with XSession() as x11:
                x11.focus_and_click(int(initial["id"], 16), *cancel_xy)
            time.sleep(1.5)
            cancel_path = output_dir / f"{profile['name']}-cancel-to-main.png"
            profile_result["cancel_frame"] = capture_record(initial["id"], cancel_path, profile)

            with XSession() as x11:
                x11.focus_and_click(int(initial["id"], 16), physical_x, physical_y)
            time.sleep(2.0)
            with XSession() as x11:
                x11.focus_and_press_key(int(initial["id"], 16), 0xff1b)  # XK_Escape
            time.sleep(1.5)
            escape_path = output_dir / f"{profile['name']}-escape-to-main.png"
            profile_result["escape_frame"] = capture_record(initial["id"], escape_path, profile)
            profile_result["escape_key_event"] = {"keysym": "Escape", "held_ms": 120,
                                                   "expected_native_behavior": "inherited da closes screen and returns to parent"}
            profile_result["play_button_clicked"] = False
        profile_result["elapsed_seconds_before_main_capture"] = round(time.time() - started, 3)
        return profile_result
    finally:
        if process is not None and process.poll() is None:
            profile_result["cleanup"]["terminate_sent"] = True
            process.terminate()
            try:
                profile_result["cleanup"]["exit_code"] = process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                profile_result["cleanup"]["killed"] = True
                process.kill()
                profile_result["cleanup"]["exit_code"] = process.wait(timeout=5)
        elif process is not None:
            profile_result["cleanup"]["exit_code"] = process.returncode
        log.close()
        profile_result["elapsed_seconds_total"] = round(time.time() - started, 3)


def capture_window(window_id, destination):
    result = subprocess.run(["import", "-window", str(window_id), str(destination)],
                            capture_output=True, text=True, timeout=20)
    if result.returncode:
        raise ValueError(f"ImageMagick import failed for X window {window_id}: {result.stderr.strip()}")
    with destination.open("rb") as stream:
        header = stream.read(24)
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"capture is not a PNG image: {destination}")


def capture_record(window_id, destination, profile):
    capture_window(window_id, destination)
    dimensions = png_dimensions(destination)
    expected = (profile["width"], profile["height"])
    if dimensions != expected:
        raise ValueError(f"capture dimensions are {dimensions}, expected {expected}")
    return {"path": str(destination), "sha256": digest(destination), "bytes": destination.stat().st_size,
            "dimensions": list(dimensions)}


def png_dimensions(path):
    with Path(path).open("rb") as stream:
        header = stream.read(24)
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n" or header[12:16] != b"IHDR":
        raise ValueError(f"capture is not a valid PNG image: {path}")
    return struct.unpack(">II", header[16:24])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", type=Path, default=ARTIFACTS / "menu-parity-capture/runtime")
    parser.add_argument("--output", type=Path, default=ARTIFACTS / "menu-parity-capture/frames")
    parser.add_argument("--jar", type=Path, default=DEFAULT_JAR)
    parser.add_argument("--save-set", choices=("empty", "synthetic"), default="empty",
                        help="capture with no saves or deterministic metadata-only test directories")
    parser.add_argument("--prepare-only", action="store_true", help="extract pinned local natives and check prerequisites without launching a window")
    parser.add_argument("--timeout", type=int, default=45, help="maximum seconds to wait for the game window")
    args = parser.parse_args()
    if args.timeout < 1 or args.timeout > 180:
        parser.error("--timeout must be in 1..180 seconds")
    try:
        runtime = inside_artifacts(args.runtime, "runtime")
        output_dir = inside_artifacts(args.output, "output")
        output_dir.mkdir(parents=True, exist_ok=True)
        context = prepare(runtime, args.jar, args.save_set)
        manifest = {
            "schema_version": 1,
            "client": {"path": str(context["jar"]), "sha256": context["client_sha256"]},
            "java8": {"path": str(JAVA8), "version": context["java_version"]},
            "libraries_sha256": context["library_sha256"],
            "extracted_native_sha256": context["native_sha256"],
            "runtime_home": str(context["game_home"]),
            "runtime_game_dir": str(context["minecraft_home"]),
            "runtime_options_policy": "minimal guiScale-only options file under disposable user.home/.minecraft; no writes to Prism instance or user home",
            "x_display": os.environ.get("DISPLAY"),
            "xvfb_invocation_expected": ("env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1400x900x24' python3 tools/oracle/native_menu_capture.py "
                                          f"--runtime {args.runtime} --output {args.output} --save-set {args.save_set}"),
            "no_home_environment_override": True,
            "network_policy": "standard JVM HTTP and HTTPS proxies pointed at 127.0.0.1:9; java.net.useSystemProxies=false; no download or installation is performed",
            "audio_policy": "ALSOFT_DRIVERS=null selects OpenAL Soft's null backend to avoid host audio devices",
            "world_policy": ("empty saves directory; never opens or creates a playable world" if args.save_set == "empty"
                             else "synthetic metadata-only saves; never opens or creates a playable world"),
            "save_set": context["save_set"],
            "synthetic_save_fixture": context["synthetic_save_fixture"],
            "synthetic_save_fixture_sha256": SYNTHETIC_SAVE_FIXTURE_SHA256 if args.save_set == "synthetic" else None,
            "synthetic_save_files_sha256": context["synthetic_save_files_sha256"],
            "captures": [],
            "capture_complete": False,
        }
        manifest_path = output_dir / "manifest.json"
        if args.prepare_only:
            manifest["prepare_only"] = True
            manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            print(f"PREPARED: local offline native runtime in {runtime}; no client was launched")
            print(f"MANIFEST: {manifest_path}")
            return 0
        if not manifest["x_display"]:
            raise ValueError("DISPLAY is unset; run the tool under xvfb-run as documented")
        manifest["argv_and_profile_timeout_seconds"] = args.timeout
        for profile in PROFILES:
            result = launch_profile(context, profile, output_dir, args.timeout)
            manifest["captures"].append(result)
            manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        manifest["capture_complete"] = True
        manifest["unavailable_save_case"] = "not captured; native GUI has no equivalent to Rust's unavailable-save safety policy"
        manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"CAPTURED: {sum(1 for row in manifest['captures'] for key, value in row.items() if key.endswith('_frame') and isinstance(value, dict))} native frames in {output_dir}")
        print(f"MANIFEST: {manifest_path}")
        return 0
    except (OSError, ValueError, zipfile.BadZipFile, subprocess.TimeoutExpired, TimeoutError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
