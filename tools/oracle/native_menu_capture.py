#!/usr/bin/env python3
"""Launch the pinned Beta 1.7.3 client in a disposable home and capture GUI screens.

All writes are confined to paths beneath this repository's ignored artifacts/
directory. Existing Prism data is read-only; this script never downloads or
opens a playable world.
"""

import argparse
import ctypes
import ctypes.util
import hashlib
import json
import math
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


def checked_command(command, timeout=15):
    result = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise ValueError(f"command failed ({result.returncode}): {' '.join(map(str, command))}\n{result.stderr.strip()}")
    return result.stdout.strip()


def prepare(runtime, jar_path):
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
    java_version = checked_command([str(JAVA8), "-version"], timeout=10)
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
        match = re.search(r"(0x[0-9a-fA-F]+)\s+\"([^\"]*)\"\s+(\d+)x(\d+)([+-]\d+)([+-]\d+)", line)
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
    raise TimeoutError(f"Minecraft window did not appear in {timeout_seconds}s; see {log_path}")


def actual_window(window):
    output = checked_command(["xwininfo", "-id", window["id"]], timeout=5)
    width = re.search(r"Width:\s+(\d+)", output)
    height = re.search(r"Height:\s+(\d+)", output)
    if not width or not height:
        raise ValueError(f"cannot read actual X window size for {window['id']}")
    return int(width.group(1)), int(height.group(1))


def set_window_size(window, width, height):
    with XSession() as x11:
        x11.lib.XMoveResizeWindow(x11.display, int(window["id"], 16), 0, 0, width, height)
        x11.lib.XFlush(x11.display)
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
        self.lib.XFlush.argtypes = [ctypes.c_void_p]
        self.xtst.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int,
                                                   ctypes.c_int, ctypes.c_ulong]
        self.xtst.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int,
                                                   ctypes.c_ulong]
        self.xtst.XTestFakeMotionEvent.restype = ctypes.c_int
        self.xtst.XTestFakeButtonEvent.restype = ctypes.c_int
        return self

    def click(self, x, y):
        if not self.xtst.XTestFakeMotionEvent(self.display, -1, x, y, 0):
            raise ValueError("XTest could not move pointer")
        if not self.xtst.XTestFakeButtonEvent(self.display, 1, 1, 0):
            raise ValueError("XTest could not press mouse button")
        self.xtst.XTestFakeButtonEvent(self.display, 1, 0, 0)
        self.lib.XFlush(self.display)

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
        "-Xms256m", "-Xmx1024m", "-cp", context["classpath"],
        "net.minecraft.client.Minecraft", "MenuParityC01", "-",
    ]
    environment = os.environ.copy()
    environment.pop("WAYLAND_DISPLAY", None)
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
        if profile["width"] != initial["width"] or profile["height"] != initial["height"]:
            actual = set_window_size(initial, profile["width"], profile["height"])
        else:
            actual = actual_window(initial)
        profile_result["initial_window"] = initial
        profile_result["actual_viewport_window"] = {"width": actual[0], "height": actual[1], "window_id": initial["id"]}
        main_path = output_dir / f"{profile['name']}-main.png"
        capture_window(initial["id"], main_path)
        main_size = png_dimensions(main_path)
        if main_size != (profile["width"], profile["height"]):
            raise ValueError(f"main-menu frame is {main_size}, expected {profile['width']}x{profile['height']}")
        profile_result["main_frame"] = {"path": str(main_path), "sha256": digest(main_path), "bytes": main_path.stat().st_size}

        gui_x = profile["scaled_width"] // 2
        gui_y = profile["scaled_height"] // 4 + 48 + 10
        physical_x = round(gui_x * profile["width"] / profile["scaled_width"])
        physical_y = round(gui_y * profile["height"] / profile["scaled_height"])
        with XSession() as x11:
            x11.click(physical_x, physical_y)
        time.sleep(4.0)
        empty_path = output_dir / f"{profile['name']}-world-list-empty.png"
        capture_window(initial["id"], empty_path)
        if png_dimensions(empty_path) != (profile["width"], profile["height"]):
            raise ValueError("world-list frame dimensions differ from the requested viewport")
        if digest(empty_path) == digest(main_path):
            raise ValueError("Singleplayer click left the frame pixel-identical; world-list transition was not observed")
        profile_result["world_list_empty_frame"] = {"path": str(empty_path), "sha256": digest(empty_path), "bytes": empty_path.stat().st_size}
        profile_result["singleplayer_click"] = {"physical_xy": [physical_x, physical_y], "scaled_gui_xy": [gui_x, gui_y]}
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
    parser.add_argument("--prepare-only", action="store_true", help="extract pinned local natives and check prerequisites without launching a window")
    parser.add_argument("--timeout", type=int, default=45, help="maximum seconds to wait for the game window")
    args = parser.parse_args()
    if args.timeout < 1 or args.timeout > 180:
        parser.error("--timeout must be in 1..180 seconds")
    try:
        runtime = inside_artifacts(args.runtime, "runtime")
        output_dir = inside_artifacts(args.output, "output")
        output_dir.mkdir(parents=True, exist_ok=True)
        context = prepare(runtime, args.jar)
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
            "xvfb_invocation_expected": "env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1400x900x24' python3 tools/oracle/native_menu_capture.py --runtime artifacts/menu-parity-capture/runtime --output artifacts/menu-parity-capture/frames",
            "no_home_environment_override": True,
            "world_policy": "empty saves directory only; never opens or creates a playable world",
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
        manifest["unavailable_save_case"] = "not captured; empty saves only"
        manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"CAPTURED: {len(manifest['captures']) * 2} native frames in {output_dir}")
        print(f"MANIFEST: {manifest_path}")
        return 0
    except (OSError, ValueError, zipfile.BadZipFile, subprocess.TimeoutExpired, TimeoutError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
