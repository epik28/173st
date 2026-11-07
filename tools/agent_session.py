#!/usr/bin/env python3
"""Cooperative single-checkout session lock; not a security boundary."""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
BRANCHES = {"codex": "codex/verification", "agy": "agy/development"}


def git(*arguments: str) -> str:
    result = subprocess.run(
        ["git", *arguments], cwd=ROOT, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or "Git command failed")
    return result.stdout.strip()


def require_clean() -> None:
    if git("status", "--porcelain", "--untracked-files=normal"):
        raise RuntimeError("Working tree is dirty. Commit your task before switching or releasing it.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("start", "finish", "status"))
    parser.add_argument("agent", nargs="?", choices=tuple(BRANCHES))
    args = parser.parse_args()
    if args.action != "status" and args.agent is None:
        parser.error("start/finish requires an agent")
    lock = Path(git("rev-parse", "--absolute-git-dir")) / "173st-agent-session.json"
    if args.action == "start":
        metadata = {
            "agent": args.agent,
            "branch": BRANCHES[args.agent],
            "started_at": datetime.now(timezone.utc).isoformat(),
        }
        try:
            descriptor = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        except FileExistsError:
            raise RuntimeError("Project is occupied. Use 'status'; do not switch branches or edit files.")
        try:
            with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
                json.dump(metadata, stream)
                stream.write("\n")
            require_clean()
            git("switch", metadata["branch"])
        except BaseException:
            lock.unlink(missing_ok=True)
            raise
        print(f"Session started: {args.agent}, branch={metadata['branch']}")
        return 0
    if not lock.exists():
        if args.action == "status":
            print(f"Session: free; branch={git('branch', '--show-current')}")
            return 0
        raise RuntimeError("No active session")
    metadata = json.loads(lock.read_text(encoding="utf-8"))
    if args.action == "status":
        print(json.dumps(metadata, ensure_ascii=False, indent=2))
        print(f"Current branch: {git('branch', '--show-current')}")
        return 0
    if metadata["agent"] != args.agent:
        raise RuntimeError("Another agent owns the session; it must finish its own task first.")
    require_clean()
    branch = git("branch", "--show-current")
    if branch not in (metadata["branch"], "main" if args.agent == "codex" else metadata["branch"]):
        raise RuntimeError("Unexpected branch; return to the session branch before finishing.")
    lock.unlink()
    print(f"Session finished: {args.agent}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError, KeyError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        raise SystemExit(1)
