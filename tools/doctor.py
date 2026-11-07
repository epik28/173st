#!/usr/bin/env python3
"""Inspect local prerequisites without reading or printing credentials."""

from pathlib import Path
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
TOOLS = (
    ("Git", ["git", "--version"]),
    ("Rust", ["rustc", "--version"]),
    ("Cargo", ["cargo", "--version"]),
    ("rustfmt", ["cargo", "fmt", "--version"]),
    ("Clippy", ["cargo", "clippy", "--version"]),
    ("Java", ["java", "-version"]),
    ("javac", ["javac", "-version"]),
)


def main() -> int:
    failed = False
    print(f"Project: {ROOT}")
    print(f"Python: {sys.version.split()[0]}")
    for label, command in TOOLS:
        try:
            result = subprocess.run(
                command, cwd=ROOT, capture_output=True, text=True, timeout=30, check=False
            )
            output = (result.stdout + "\n" + result.stderr).strip().splitlines()
            if result.returncode:
                print(f"MISSING: {label} (exit {result.returncode})")
                failed = True
            else:
                version = output[0] if output else "available"
                if label in ("Java", "javac"):
                    prefixes = ("openjdk ", "java ") if label == "Java" else ("javac ",)
                    version = next((line for line in output if line.startswith(prefixes)), version)
                print(f"OK: {label}: {version}")
        except (OSError, subprocess.TimeoutExpired):
            print(f"MISSING: {label}")
            failed = True

    for key in ("user.name", "user.email"):
        try:
            result = subprocess.run(
                ["git", "config", "--get", key], cwd=ROOT,
                capture_output=True, timeout=10, check=False
            )
            configured = result.returncode == 0 and bool(result.stdout.strip())
        except (OSError, subprocess.TimeoutExpired):
            configured = False
        print(f"Git {key}: {'configured' if configured else 'missing'}")
        failed |= not configured

    config_path = ROOT / "config/local.toml"
    if not config_path.is_file():
        config_path = ROOT / "config/local.example.toml"
    print(f"Path configuration: {config_path.relative_to(ROOT)}")
    try:
        with config_path.open("rb") as stream:
            paths = tomllib.load(stream).get("paths", {})
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"ERROR: path configuration: {error}")
        return 1
    for key in ("beta_wiki", "reference_sources", "minecraft_reference", "prism_instance", "client_jar", "server_jar"):
        value = paths.get(key)
        if value is None:
            print(f"REFERENCE: {key}: not configured")
            continue
        path = Path(value).expanduser()
        if not path.is_absolute():
            path = ROOT / path
        print(f"REFERENCE: {key}: {'present' if path.exists() else 'missing'}")

    print("This checks the environment only; it does not validate Minecraft compatibility or GitHub write access.")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
