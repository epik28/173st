"""Check real worktree isolation and independent agent session ownership."""

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
        self.temp = tempfile.TemporaryDirectory(prefix="173st-worktree-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repository"
        (self.repo / "tools").mkdir(parents=True)
        shutil.copyfile(SCRIPT, self.repo / "tools/agent_session.py")
        (self.repo / "README.md").write_text("initial\n")
        self.git("init", "-b", "main")
        self.git("config", "user.name", "173st test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", "/dev/null")
        self.git("add", "README.md", "tools/agent_session.py")
        self.git("commit", "-m", "test fixture")
        self.checkouts = {agent: self.root / agent for agent in ("codex", "agy")}
        for agent, branch in (("codex", "codex/verification"), ("agy", "agy/development")):
            self.git("worktree", "add", "-b", branch, str(self.checkouts[agent]), "main")

    def git(self, *args, cwd=None):
        return subprocess.run(
            ["git", *args], cwd=cwd or self.repo, text=True,
            capture_output=True, check=True,
        ).stdout.strip()

    def session(self, checkout, *args):
        directory = self.checkouts[checkout]
        return subprocess.run(
            [sys.executable, str(directory / "tools/agent_session.py"), *args],
            cwd=directory, text=True, capture_output=True, check=False,
        )

    def lock(self, agent):
        git_dir = self.git("rev-parse", "--absolute-git-dir", cwd=self.checkouts[agent])
        return Path(git_dir) / "173st-agent-session.json"

    def test_parallel_worktrees_both_start(self):
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(lambda agent: self.session(agent, "start", agent), ("codex", "agy")))
        self.assertTrue(all(result.returncode == 0 for result in results))
        self.assertNotEqual(self.lock("codex"), self.lock("agy"))
        for agent in ("codex", "agy"):
            self.assertEqual(json.loads(self.lock(agent).read_text())["agent"], agent)
        self.assertEqual(self.session("codex", "finish", "codex").returncode, 0)
        self.assertTrue(self.lock("agy").exists())
        self.assertEqual(self.session("agy", "finish", "agy").returncode, 0)

    def test_same_worktree_refuses_second_session_and_wrong_release(self):
        self.assertEqual(self.session("codex", "start", "codex").returncode, 0)
        self.assertNotEqual(self.session("codex", "start", "codex").returncode, 0)
        self.assertNotEqual(self.session("codex", "finish", "agy").returncode, 0)
        self.assertTrue(self.lock("codex").exists())
        self.assertEqual(self.session("codex", "finish", "codex").returncode, 0)

    def test_wrong_role_does_not_switch_branch(self):
        self.assertNotEqual(self.session("codex", "start", "agy").returncode, 0)
        self.assertEqual(self.git("branch", "--show-current", cwd=self.checkouts["codex"]), "codex/verification")
        self.assertFalse(self.lock("codex").exists())

    def test_dirty_start_preserves_files_and_other_worktree(self):
        scratch = self.checkouts["codex"] / "uncommitted.txt"
        scratch.write_text("keep this\n")
        self.assertNotEqual(self.session("codex", "start", "codex").returncode, 0)
        self.assertEqual(scratch.read_text(), "keep this\n")
        self.assertFalse((self.checkouts["agy"] / "uncommitted.txt").exists())
        self.assertFalse(self.lock("codex").exists())
        self.assertEqual(self.session("agy", "start", "agy").returncode, 0)
        self.assertEqual(self.session("agy", "finish", "agy").returncode, 0)

    def test_dirty_finish_keeps_only_own_session(self):
        self.assertEqual(self.session("codex", "start", "codex").returncode, 0)
        readme = self.checkouts["codex"] / "README.md"
        readme.write_text("uncommitted task\n")
        self.assertNotEqual(self.session("codex", "finish", "codex").returncode, 0)
        self.assertTrue(self.lock("codex").exists())
        self.assertEqual((self.checkouts["agy"] / "README.md").read_text(), "initial\n")
        self.assertEqual(self.session("agy", "start", "agy").returncode, 0)
        self.assertEqual(self.session("agy", "finish", "agy").returncode, 0)
        readme.write_text("initial\n")
        self.assertEqual(self.session("codex", "finish", "codex").returncode, 0)

    def test_racing_starts_in_one_worktree_have_one_owner(self):
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(lambda _: self.session("codex", "start", "codex"), range(2)))
        self.assertEqual(sum(result.returncode == 0 for result in results), 1)
        self.assertEqual(self.session("codex", "finish", "codex").returncode, 0)


if __name__ == "__main__":
    unittest.main()
