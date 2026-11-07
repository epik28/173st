"""Protect the shared checkout from accidental agent handoff conflicts."""

from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[2] / "tools/agent_session.py"


class AgentSessionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="173st-session-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "tools").mkdir()
        shutil.copyfile(SCRIPT, self.root / "tools/agent_session.py")
        (self.root / "README.md").write_text("initial\n")
        self.git("init", "-b", "main")
        self.git("config", "user.name", "173st test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", "/dev/null")
        self.git("add", "README.md", "tools/agent_session.py")
        self.git("commit", "-m", "test fixture")
        for branch in ("codex/verification", "agy/development"):
            self.git("branch", branch)

    def git(self, *args):
        return subprocess.run(
            ["git", *args], cwd=self.root, text=True,
            capture_output=True, check=True,
        ).stdout.strip()

    def session(self, *args):
        return subprocess.run(
            [sys.executable, str(self.root / "tools/agent_session.py"), *args],
            cwd=self.root, text=True, capture_output=True, check=False,
        )

    def lock(self):
        return self.root / ".git/173st-agent-session.json"

    def test_handoff_refuses_second_agent_and_wrong_owner(self):
        self.assertEqual(self.session("start", "codex").returncode, 0)
        self.assertNotEqual(self.session("start", "agy").returncode, 0)
        self.assertEqual(self.git("branch", "--show-current"), "codex/verification")
        self.assertNotEqual(self.session("finish", "agy").returncode, 0)
        self.assertTrue(self.lock().exists())
        self.assertEqual(self.session("finish", "codex").returncode, 0)
        self.assertEqual(self.session("start", "agy").returncode, 0)
        self.assertEqual(self.git("branch", "--show-current"), "agy/development")
        self.assertEqual(self.session("finish", "agy").returncode, 0)

    def test_dirty_start_preserves_files_and_branch(self):
        scratch = self.root / "uncommitted.txt"
        scratch.write_text("keep this\n")
        self.assertNotEqual(self.session("start", "codex").returncode, 0)
        self.assertEqual(scratch.read_text(), "keep this\n")
        self.assertEqual(self.git("branch", "--show-current"), "main")
        self.assertFalse(self.lock().exists())

    def test_dirty_finish_keeps_session_owned(self):
        self.assertEqual(self.session("start", "codex").returncode, 0)
        readme = self.root / "README.md"
        readme.write_text("uncommitted task\n")
        self.assertNotEqual(self.session("finish", "codex").returncode, 0)
        self.assertTrue(self.lock().exists())
        self.assertEqual(readme.read_text(), "uncommitted task\n")
        readme.write_text("initial\n")
        self.assertEqual(self.session("finish", "codex").returncode, 0)

    def test_simultaneous_starts_have_one_owner(self):
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(lambda agent: self.session("start", agent), ("codex", "agy")))
        self.assertEqual(sum(result.returncode == 0 for result in results), 1)
        owner = json.loads(self.lock().read_text())["agent"]
        self.assertEqual(
            self.git("branch", "--show-current"),
            {"codex": "codex/verification", "agy": "agy/development"}[owner],
        )
        self.assertEqual(self.session("finish", owner).returncode, 0)


if __name__ == "__main__":
    unittest.main()
