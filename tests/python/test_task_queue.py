"""Role queues must gate integration on committed peer results, not local copies."""

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[2] / "tools/tasks.py"
REFS = {"codex": "codex/verification", "agy": "agy/development"}


class TaskQueueTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="173st-queue-test-")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        (self.root / "tools").mkdir()
        shutil.copyfile(SCRIPT, self.root / "tools/tasks.py")
        (self.root / "docs/tasks/status").mkdir(parents=True)
        (self.root / "docs/tasks/CARD.md").write_text("Test-only task contract\n")
        tasks = []
        for task_id, owner, dependencies in (
            ("C1", "codex", []), ("C2", "codex", ["C1"]),
            ("A1", "agy", []), ("A2", "agy", ["A1"]),
            ("C3", "codex", ["C1", "C2", "A1", "A2"]),
        ):
            tasks.append({"id": task_id, "owner": owner, "title": task_id,
                          "card": "docs/tasks/CARD.md", "depends_on": dependencies})
        self.write("docs/tasks/queue.json", {"schema_version": 1, "tasks": tasks})
        for role in REFS:
            entries = {task["id"]: {"state": "pending", "summary": "", "checks": []}
                       for task in tasks if task["owner"] == role}
            self.write("docs/tasks/status/" + role + ".json",
                       {"schema_version": 1, "role": role, "tasks": entries})
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Queue test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", "/dev/null")
        self.git("add", ".")
        self.git("commit", "-m", "test-only board")
        for reference in REFS.values():
            self.git("branch", reference)

    def write(self, path, value):
        (self.root / path).write_text(json.dumps(value) + "\n")

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, text=True,
                              capture_output=True, check=True).stdout.strip()

    def mark(self, role, task_ids, state="done", commit=True, evidence=True):
        self.git("switch", REFS[role])
        path = "docs/tasks/status/" + role + ".json"
        document = json.loads((self.root / path).read_text())
        for task_id in task_ids:
            document["tasks"][task_id] = {
                "state": state, "summary": "Synthetic test result",
                "checks": ["test-only evidence"] if evidence else [],
            }
        self.write(path, document)
        if commit:
            self.git("add", path)
            self.git("commit", "-m", "test-only status update")

    def next(self, role):
        result = subprocess.run([sys.executable, "tools/tasks.py", "next", role],
                                cwd=self.root, text=True, capture_output=True, check=False)
        return result.returncode, json.loads(result.stdout or result.stderr)

    def test_first_tasks_are_independent(self):
        self.assertEqual(self.next("codex")[1]["task"]["id"], "C1")
        self.assertEqual(self.next("agy")[1]["task"]["id"], "A1")

    def test_uncommitted_peer_status_cannot_unlock_integration(self):
        self.mark("codex", ["C1", "C2"])
        self.mark("agy", ["A1", "A2"], commit=False)
        code, output = self.next("codex")
        self.assertEqual(code, 2)
        self.assertEqual(output["state"], "waiting")
        self.assertEqual(output["waiting"][0]["dependencies"], {"A1": "pending", "A2": "pending"})

    def test_committed_peer_result_unlocks_integration_with_exact_snapshot(self):
        self.mark("codex", ["C1", "C2"])
        self.mark("agy", ["A1", "A2"])
        peer = self.git("rev-parse", "HEAD")
        self.git("switch", REFS["codex"])
        stale = json.loads((self.root / "docs/tasks/status/agy.json").read_text())
        self.assertEqual(stale["tasks"]["A1"]["state"], "pending")
        code, output = self.next("codex")
        self.assertEqual(code, 0)
        self.assertEqual(output["task"]["id"], "C3")
        self.assertEqual(output["snapshots"]["agy"]["commit"], peer)

    def test_done_requires_evidence(self):
        self.mark("codex", ["C1"], evidence=False)
        code, output = self.next("codex")
        self.assertEqual(code, 1)
        self.assertEqual(output["state"], "error")

    def test_completed_role_returns_complete(self):
        self.mark("agy", ["A1", "A2"])
        code, output = self.next("agy")
        self.assertEqual(code, 3)
        self.assertEqual(output["state"], "complete")

    def test_active_task_is_resumed_and_blocked_task_is_not_skipped(self):
        self.mark("codex", ["C1"], state="in_progress")
        self.assertEqual(self.next("codex")[1]["state"], "resume")
        self.mark("codex", ["C1"], state="blocked")
        code, output = self.next("codex")
        self.assertEqual(code, 2)
        self.assertEqual(output["waiting"][0]["state"], "blocked")

    def test_incompatible_queue_is_error(self):
        self.git("switch", REFS["agy"])
        path = "docs/tasks/queue.json"
        document = json.loads((self.root / path).read_text())
        document["tasks"][0]["title"] = "different contract"
        self.write(path, document)
        self.git("add", path)
        self.git("commit", "-m", "test-only incompatible queue")
        code, output = self.next("codex")
        self.assertEqual(code, 1)
        self.assertEqual(output["state"], "error")

    def test_duplicate_committed_status_cannot_silently_reset_completed_task(self):
        self.git("switch", REFS["codex"])
        path = "docs/tasks/status/codex.json"
        original = (self.root / path).read_text()
        duplicate = '"tasks": {"C1": {"state": "done", "summary": "accepted", "checks": ["proof"]}, '
        (self.root / path).write_text(original.replace('"tasks": {', duplicate, 1))
        self.git("add", path)
        self.git("commit", "-m", "test-only duplicate status")
        code, output = self.next("codex")
        self.assertEqual(code, 1)
        self.assertIn("Duplicate JSON key: C1", output["reason"])

    def test_parallel_role_tasks_are_visible_without_silently_skipping_active_work(self):
        self.mark("codex", ["C1", "C2", "C3"], state="in_progress")
        code, output = self.next("codex")
        self.assertEqual(code, 0)
        self.assertEqual(output["state"], "resume")
        self.assertEqual(output["task"]["id"], "C1")


if __name__ == "__main__":
    unittest.main()
