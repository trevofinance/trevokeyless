"""Mutation-contract tests for the tracked-state review guard."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GUARD = ROOT / ".agents/skills/review-guard/scripts/review_guard.py"


class ReviewGuardTests(unittest.TestCase):
    def git(self, root: Path, *args: str) -> None:
        subprocess.run(  # noqa: S603
            ["git", "-C", str(root), *args], check=True, capture_output=True
        )

    def test_allows_existing_dirty_state_and_new_untracked_diagnostics(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            root.mkdir()
            self.git(root, "init", "-q")
            self.git(root, "config", "user.email", "test@example.invalid")
            self.git(root, "config", "user.name", "Test")
            tracked = root / "tracked.txt"
            tracked.write_text("base\n")
            self.git(root, "add", "tracked.txt")
            self.git(root, "commit", "-qm", "initial")
            tracked.write_text("preexisting\n")
            snapshot = Path(directory) / "snapshot.json"
            subprocess.run(  # noqa: S603
                [sys.executable, str(GUARD), "snapshot", "--repo-root", str(root), "--snapshot", str(snapshot)],
                check=True,
                capture_output=True,
            )
            (root / "diagnostic.log").write_text("allowed\n")
            result = subprocess.run(  # noqa: S603
                [sys.executable, str(GUARD), "verify", "--repo-root", str(root), "--snapshot", str(snapshot)],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(json.loads(snapshot.read_text())["schema_version"], 1)

    def test_rejects_new_tracked_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            root.mkdir()
            self.git(root, "init", "-q")
            self.git(root, "config", "user.email", "test@example.invalid")
            self.git(root, "config", "user.name", "Test")
            tracked = root / "tracked.txt"
            tracked.write_text("base\n")
            self.git(root, "add", "tracked.txt")
            self.git(root, "commit", "-qm", "initial")
            snapshot = Path(directory) / "snapshot.json"
            subprocess.run(  # noqa: S603
                [sys.executable, str(GUARD), "snapshot", "--repo-root", str(root), "--snapshot", str(snapshot)],
                check=True,
                capture_output=True,
            )
            tracked.write_text("changed\n")
            result = subprocess.run(  # noqa: S603
                [sys.executable, str(GUARD), "verify", "--repo-root", str(root), "--snapshot", str(snapshot)],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 1)
            self.assertIn("tracked worktree changed", result.stdout)


if __name__ == "__main__":
    unittest.main()
