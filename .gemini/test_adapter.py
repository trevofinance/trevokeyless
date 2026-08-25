"""Mutation tests for the Gemini CLI adapter boundary."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path, PurePosixPath
from unittest import mock

import check_adapter


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf8")


class GeminiAdapterTests(unittest.TestCase):
    def test_committed_adapter_is_valid(self) -> None:
        self.assertEqual(check_adapter.validate(), [])

    def test_expanded_or_nested_shim_is_rejected(self) -> None:
        paths = (PurePosixPath("GEMINI.md"), PurePosixPath("package/GEMINI.md"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "GEMINI.md", check_adapter.SHIM + "extra policy\n")
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_shim(paths)
        self.assertTrue(errors)

    def test_provider_local_skill_copy_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".agents/skills").mkdir(parents=True)
            (root / ".gemini/skills").mkdir(parents=True)
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_native_skills()
        self.assertTrue(any("provider-local skills" in error for error in errors))

    def test_branch_restricted_workflow_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(
                root / check_adapter.WORKFLOW,
                "branches: [main]\n"
                "persist-credentials: false\n"
                "python3 .gemini/check_adapter.py\n"
                "python3 -m unittest discover -s .gemini -p 'test_*.py' --verbose\n"
                "python3 .github/scripts/check_ai_config.py --scope adapters\n",
            )
            with mock.patch.object(check_adapter, "ROOT", root):
                errors = check_adapter.validate_workflow()
        self.assertTrue(any("restricted" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
