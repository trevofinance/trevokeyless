"""Mutation tests for the Claude adapter boundary."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path, PurePosixPath
from unittest import mock

import check_adapter


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf8")


class ClaudeAdapterTests(unittest.TestCase):
    def test_committed_adapter_is_valid(self) -> None:
        self.assertEqual(check_adapter.validate(), [])

    def test_shim_mutation_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "AGENTS.md", "core\n")
            write(root / "CLAUDE.md", check_adapter.ROOT_SHIM + "extra policy\n")
            paths = (PurePosixPath("AGENTS.md"), PurePosixPath("CLAUDE.md"))
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_shims(paths)
        self.assertTrue(any("thin import shim" in error for error in errors))

    def test_missing_nested_shim_is_rejected(self) -> None:
        paths = (
            PurePosixPath("AGENTS.md"),
            PurePosixPath("CLAUDE.md"),
            PurePosixPath("package/AGENTS.md"),
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "CLAUDE.md", check_adapter.ROOT_SHIM)
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_shims(paths)
        self.assertTrue(any("no Claude shim" in error for error in errors))

    def test_skill_copy_or_wrong_target_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / ".agents/skills/demo/SKILL.md", "skill\n")
            write(root / ".claude/skills/demo/SKILL.md", "copied\n")
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_skills()
        self.assertTrue(any("must target" in error for error in errors))

    def test_settings_narrowing_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            settings = json.loads(json.dumps(check_adapter.EXPECTED_SETTINGS))
            settings["permissions"] = {"defaultMode": "plan"}
            write(root / ".claude/settings.json", json.dumps(settings))
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_settings()
        self.assertTrue(errors)

    def test_provider_local_agent_layer_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".claude/agents").mkdir(parents=True)
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_surfaces()
        self.assertTrue(any("provider-local agents" in error for error in errors))

    def test_branch_restricted_workflow_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(
                root / check_adapter.WORKFLOW,
                "branches: [main]\n"
                "persist-credentials: false\n"
                "python3 .claude/check_adapter.py\n"
                "python3 -m unittest discover -s .claude -p 'test_*.py' --verbose\n"
                "python3 .github/scripts/check_ai_config.py --scope adapters\n",
            )
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_workflow()
        self.assertTrue(any("restricted" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
