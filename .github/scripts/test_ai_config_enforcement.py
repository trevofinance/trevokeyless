"""Regression tests for AI-config patch isolation and secret boundaries."""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path, PurePosixPath

from ai_config.adapters import Adapter
from ai_config.independence import overlay_working_patch, remove_adapters
from ai_config.secrets import validate_secret_boundaries


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf8")


def git(root: Path, *arguments: str) -> None:
    subprocess.run(["git", *arguments], cwd=root, check=True, capture_output=True)  # noqa: S603


def demo_adapter() -> Adapter:
    return Adapter(
        PurePosixPath(".demo"),
        "demo",
        {"external_owned_paths": ["DEMO.md"]},
    )


class PatchIsolationTests(unittest.TestCase):
    def test_overlay_includes_tracked_and_untracked_work(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "source"
            checkout = Path(directory) / "checkout"
            root.mkdir()
            git(root, "init", "--initial-branch=main")
            git(root, "config", "user.email", "ai-config@example.invalid")
            git(root, "config", "user.name", "AI Config Test")
            write(root / "tracked.txt", "base\n")
            git(root, "add", "tracked.txt")
            git(root, "commit", "-m", "base")
            git(Path(directory), "clone", os.fspath(root), os.fspath(checkout))
            write(root / "tracked.txt", "changed\n")
            write(root / "pending.txt", "pending\n")

            self.assertEqual(overlay_working_patch(root, checkout), [])
            self.assertEqual((checkout / "tracked.txt").read_text(), "changed\n")
            self.assertEqual((checkout / "pending.txt").read_text(), "pending\n")

    def test_adapter_removal_preserves_neutral_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / ".demo/adapter.toml", "adapter\n")
            write(root / "DEMO.md", "shim\n")
            write(root / "AGENTS.md", "neutral\n")
            remove_adapters(root, [demo_adapter()])
            self.assertFalse((root / ".demo").exists())
            self.assertFalse((root / "DEMO.md").exists())
            self.assertTrue((root / "AGENTS.md").is_file())


class SecretBoundaryTests(unittest.TestCase):
    def test_safe_ai_configuration_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / ".demo/README.md", "No account-backed probe was run.\n")
            self.assertEqual(validate_secret_boundaries(root, [demo_adapter()]), [])

    def test_credential_content_reports_only_its_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            field = "api_" + "key"
            value = "sensitive" * 4
            write(root / ".demo/settings.json", f'{{"{field}": "{value}"}}\n')
            errors = validate_secret_boundaries(root, [demo_adapter()])
            rendered = "\n".join(errors)
            self.assertIn(".demo/settings.json", rendered)
            self.assertNotIn(value, rendered)

    def test_transcript_and_environment_paths_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / ".demo/transcripts/run.json", "{}\n")
            write(root / ".agents/.env", "local\n")
            errors = validate_secret_boundaries(root, [demo_adapter()])
            self.assertEqual(len(errors), 2)


if __name__ == "__main__":
    unittest.main()
