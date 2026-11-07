#!/usr/bin/env python3
"""Select role-owned tasks from committed local Git snapshots; never execute them."""

import argparse
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
REFS = {"codex": "refs/heads/codex/verification", "agy": "refs/heads/agy/development"}
STATES = {"pending", "in_progress", "done", "blocked"}


def git(root, *arguments):
    result = subprocess.run(
        ["git", *arguments], cwd=root, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise ValueError(result.stderr.strip() or "Cannot read Git task state")
    return result.stdout.strip()


def load_queue(root):
    document = json.loads((root / "docs/tasks/queue.json").read_text())
    if document.get("schema_version") != 1:
        raise ValueError("Unsupported queue schema")
    tasks = document["tasks"]
    ids = [task["id"] for task in tasks]
    if not tasks or len(set(ids)) != len(ids):
        raise ValueError("Task IDs must be nonempty and unique")
    for task in tasks:
        if not task["id"] or task["owner"] not in REFS:
            raise ValueError("Invalid task identity or owner")
        if not (root / task["card"]).is_file():
            raise ValueError("Missing task card: " + task["card"])
        if any(dependency not in ids for dependency in task["depends_on"]):
            raise ValueError("Unknown task dependency")
    resolved = set()
    while len(resolved) < len(tasks):
        ready = {task["id"] for task in tasks if set(task["depends_on"]) <= resolved}
        if ready <= resolved:
            raise ValueError("Task dependency cycle")
        resolved |= ready
    return tasks


def load_board(root):
    tasks = load_queue(root)
    states = {}
    snapshots = {}
    for role, reference in REFS.items():
        commit = git(root, "rev-parse", "--verify", reference + "^{commit}")
        peer_queue = json.loads(git(root, "show", commit + ":docs/tasks/queue.json"))
        if peer_queue.get("schema_version") != 1 or peer_queue.get("tasks") != tasks:
            raise ValueError("Queue differs between working contract and role snapshot: " + role)
        path = "docs/tasks/status/" + role + ".json"
        document = json.loads(git(root, "show", commit + ":" + path))
        if document.get("schema_version") != 1 or document.get("role") != role:
            raise ValueError("Invalid status schema or owner: " + role)
        owned_ids = {task["id"] for task in tasks if task["owner"] == role}
        entries = document["tasks"]
        if set(entries) != owned_ids:
            raise ValueError("Status task IDs do not match role queue: " + role)
        for task_id, entry in entries.items():
            if entry.get("state") not in STATES:
                raise ValueError("Invalid task state: " + task_id)
            if not isinstance(entry.get("summary"), str) or not isinstance(entry.get("checks"), list):
                raise ValueError("Invalid evidence fields: " + task_id)
            if any(not isinstance(check, str) or not check.strip() for check in entry["checks"]):
                raise ValueError("Invalid check evidence: " + task_id)
            if entry["state"] == "done" and (not entry["summary"].strip() or not entry["checks"]):
                raise ValueError("Completed task requires summary and checks: " + task_id)
            if entry["state"] == "blocked" and not entry["summary"].strip():
                raise ValueError("Blocked task requires a reason: " + task_id)
        if sum(entry["state"] == "in_progress" for entry in entries.values()) > 1:
            raise ValueError("A role may have at most one in-progress task")
        states.update(entries)
        snapshots[role] = {"ref": reference, "commit": commit}
    return tasks, states, snapshots


def select_task(tasks, states, role):
    owned = [task for task in tasks if task["owner"] == role]
    if all(states[task["id"]]["state"] == "done" for task in owned):
        return {"state": "complete", "role": role}, 3
    active = [task for task in owned if states[task["id"]]["state"] == "in_progress"]
    candidates = active or owned
    for task in candidates:
        state = states[task["id"]]["state"]
        if state in ("pending", "in_progress") and all(
            states[dependency]["state"] == "done" for dependency in task["depends_on"]
        ):
            return {"state": "resume" if state == "in_progress" else "ready", "role": role, "task": task}, 0
    waiting = []
    for task in owned:
        if states[task["id"]]["state"] != "done":
            waiting.append({
                "id": task["id"],
                "state": states[task["id"]]["state"],
                "reason": states[task["id"]]["summary"],
                "dependencies": {
                    dependency: states[dependency]["state"]
                    for dependency in task["depends_on"] if states[dependency]["state"] != "done"
                },
            })
    return {"state": "waiting", "role": role, "waiting": waiting}, 2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("status", "next"))
    parser.add_argument("role", choices=tuple(REFS), nargs="?")
    args = parser.parse_args()
    if args.command == "next" and args.role is None:
        parser.error("next requires codex or agy")
    tasks, states, snapshots = load_board(ROOT)
    if args.command == "status":
        output = {"state": "status", "tasks": [{**task, **states[task["id"]]} for task in tasks]}
        code = 0
    else:
        output, code = select_task(tasks, states, args.role)
    output["snapshots"] = snapshots
    print(json.dumps(output, ensure_ascii=False, indent=2))
    return code


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({"state": "error", "reason": str(error)}, ensure_ascii=False), file=sys.stderr)
        raise SystemExit(1)
