#!/usr/bin/env python3
"""Opt-in viewer flow check inside a caller-owned isolated X11 display.

Run under xvfb-run. Default: main menu -> list -> cancel -> quit. With
--enter-world: explicitly select the first enabled world, open its disposable
snapshot and capture the game. This checks Rust UI integration, not native
pixel parity. Source-save validation remains in the viewer.
"""

from __future__ import annotations

import argparse
import ctypes as c
from ctypes.util import find_library
import json
import os
from pathlib import Path
import subprocess
import time


class Display:
    def __init__(self):
        self.x = c.CDLL(find_library("X11") or "libX11.so.6")
        self.test = c.CDLL(find_library("Xtst") or "libXtst.so.6")
        signatures = {
            "XOpenDisplay": ([c.c_char_p], c.c_void_p),
            "XDefaultRootWindow": ([c.c_void_p], c.c_ulong),
            "XQueryTree": ([c.c_void_p, c.c_ulong, c.POINTER(c.c_ulong),
                            c.POINTER(c.c_ulong), c.POINTER(c.POINTER(c.c_ulong)),
                            c.POINTER(c.c_uint)], c.c_int),
            "XFetchName": ([c.c_void_p, c.c_ulong, c.POINTER(c.c_char_p)], c.c_int),
            "XFree": ([c.c_void_p], c.c_int),
            "XStringToKeysym": ([c.c_char_p], c.c_ulong),
            "XKeysymToKeycode": ([c.c_void_p, c.c_ulong], c.c_uint),
            "XSetInputFocus": ([c.c_void_p, c.c_ulong, c.c_int, c.c_ulong], c.c_int),
            "XSync": ([c.c_void_p, c.c_int], c.c_int),
            "XCloseDisplay": ([c.c_void_p], c.c_int),
        }
        for name, (args, result) in signatures.items():
            function = getattr(self.x, name)
            function.argtypes, function.restype = args, result
        self.test.XTestFakeKeyEvent.argtypes = [c.c_void_p, c.c_uint, c.c_int, c.c_ulong]
        self.test.XTestFakeKeyEvent.restype = c.c_int
        self.display = self.x.XOpenDisplay(None)
        if not self.display:
            raise RuntimeError("cannot open the caller's X11 display")

    def windows(self):
        root = self.x.XDefaultRootWindow(self.display)
        root_out, parent, count = c.c_ulong(), c.c_ulong(), c.c_uint()
        children = c.POINTER(c.c_ulong)()
        if not self.x.XQueryTree(self.display, root, c.byref(root_out),
                                 c.byref(parent), c.byref(children), c.byref(count)):
            raise RuntimeError("XQueryTree failed")
        result = []
        try:
            for index in range(count.value):
                name = c.c_char_p()
                window = children[index]
                if self.x.XFetchName(self.display, window, c.byref(name)) and name:
                    result.append((window, name.value.decode("utf-8", errors="replace")))
                    self.x.XFree(c.cast(name, c.c_void_p))
        finally:
            if children:
                self.x.XFree(children)
        return result

    def key(self, window, name):
        self.x.XSetInputFocus(self.display, window, 2, 0)
        key = self.x.XKeysymToKeycode(self.display, self.x.XStringToKeysym(name.encode()))
        if not key:
            raise RuntimeError(f"no keycode for {name}")
        for pressed in (1, 0):
            if not self.test.XTestFakeKeyEvent(self.display, key, pressed, 0):
                raise RuntimeError("XTestFakeKeyEvent failed")
        self.x.XSync(self.display, 0)

    def close(self):
        self.x.XCloseDisplay(self.display)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/beta-viewer"))
    parser.add_argument("--output", type=Path, default=Path("artifacts/viewer-smoke"))
    parser.add_argument("--enter-world", action="store_true")
    args = parser.parse_args()
    if os.environ.get("WAYLAND_DISPLAY") or not os.environ.get("DISPLAY"):
        parser.error("use an isolated X11 display; unset WAYLAND_DISPLAY")
    args.output.mkdir(parents=True, exist_ok=True)
    display = Display()
    report = {"scope": "Rust UI flow only; no native pixel parity", "captures": []}
    process = None
    try:
        with (args.output / "viewer.log").open("w") as log:
            process = subprocess.Popen([str(args.binary.resolve())], stdout=log, stderr=log)

            def wait_for(predicate):
                deadline = time.monotonic() + 25
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise RuntimeError(f"viewer exited {process.returncode}; see viewer.log")
                    for window, title in display.windows():
                        if predicate(title):
                            return window
                    time.sleep(0.05)
                raise TimeoutError("expected viewer window did not appear")

            def capture(window, name):
                time.sleep(0.4)
                path = args.output / f"{name}.png"
                subprocess.run(["import", "-window", str(window), str(path)],
                               check=True, timeout=8)
                report["captures"].append(str(path))

            menu = wait_for(lambda title: title == "Minecraft Beta 1.7.3")
            capture(menu, "main-menu")
            display.key(menu, "Return")
            worlds = wait_for(lambda title: "Select World" in title)
            capture(worlds, "world-list")
            if args.enter_world:
                display.key(worlds, "Down")
                capture(worlds, "world-selection")
                display.key(worlds, "Return")
                game = wait_for(lambda title: "WASD" in title or "sandbox" in title)
                capture(game, "world-preview")
                report["result"] = "opened disposable world preview"
            else:
                display.key(worlds, "Escape")
                menu = wait_for(lambda title: title == "Minecraft Beta 1.7.3")
                capture(menu, "cancel-to-menu")
                display.key(menu, "Escape")
                if process.wait(timeout=8) != 0:
                    raise RuntimeError("menu quit did not exit successfully")
                report["result"] = "list cancel and menu quit passed"
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report))
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        display.close()


if __name__ == "__main__":
    main()
