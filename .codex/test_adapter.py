"""Mutation tests for the Codex adapter boundary."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path, PurePosixPath
from unittest import mock

import check_adapter


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf8")


class CodexAdapterTests(unittest.TestCase):
    def test_committed_adapter_is_valid(self) -> None:
        self.assertEqual(check_adapter.validate(), [])

    def test_config_mutation_is_rejected(self) -> None:
        source = (check_adapter.ROOT / ".codex/config.toml").read_text(encoding="utf8")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(
                root / ".codex/config.toml",
                source.replace(
                    'approval_policy = "never"', 'approval_policy = "on-request"'
                ),
            )
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_config()
        self.assertTrue(any("closed host posture" in error for error in errors))

    def test_missing_native_instruction_is_rejected(self) -> None:
        paths = tuple(
            check_adapter.REQUIRED_INSTRUCTIONS
            - {PurePosixPath("keyless-rust/AGENTS.md")}
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_native_discovery(paths)
        self.assertTrue(any("keyless-rust" in error for error in errors))

    def test_shim_and_provider_local_agent_layer_are_rejected(self) -> None:
        paths = (*check_adapter.REQUIRED_INSTRUCTIONS, PurePosixPath("CODEX.md"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".codex/agents").mkdir(parents=True)
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_native_discovery(tuple(paths))
        self.assertTrue(any("not a shim" in error for error in errors))
        self.assertTrue(any("provider-local agents" in error for error in errors))

    def test_branch_restricted_workflow_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(
                root / check_adapter.WORKFLOW,
                "branches: [main]\n"
                "persist-credentials: false\n"
                "python3 .codex/check_adapter.py\n"
                "python3 -m unittest discover -s .codex -p 'test_*.py' --verbose\n"
                "python3 .github/scripts/check_ai_config.py --scope adapters\n",
            )
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_workflow()
        self.assertTrue(any("restricted" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
