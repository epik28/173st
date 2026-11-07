#!/usr/bin/env python3
"""Run the implemented workspace checks; this is not a parity test runner."""

from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
COMMANDS = (
    [sys.executable, "-m", "unittest", "discover", "-s", "tests/python", "-v"],
    ["cargo", "fmt", "--all", "--", "--check"],
    ["cargo", "check", "--workspace", "--all-targets", "--locked", "--offline"],
    ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"],
    ["cargo", "test", "--workspace", "--locked", "--offline"],
)


def main() -> int:
    for command in COMMANDS:
        print("+ " + " ".join(command), flush=True)
        try:
            result = subprocess.run(command, cwd=ROOT, check=False)
        except OSError as error:
            print(str(error), file=sys.stderr)
            return 1
        if result.returncode:
            return result.returncode if result.returncode > 0 else 1
    print("Workspace checks passed. Parity coverage must be checked separately.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
