"""Regression tests for provider-neutral AI configuration conformance."""

from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path

import check_ai_config
from ai_config import core

ROOT = Path(__file__).resolve().parents[2]


class CoreConformanceTests(unittest.TestCase):
    def test_committed_core_conforms(self) -> None:
        self.assertEqual(check_ai_config.validate("core"), [])

    def test_immutable_policy_mutation_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in core.POLICY_HASHES:
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / relative, destination)
            changed = root / "docs/AI-CONFIG.md"
            changed.write_text(changed.read_text() + "\nmutation\n")
            errors = core.validate_policy(root)
        self.assertTrue(any("immutable policy file changed" in error for error in errors))

    def test_manifest_missing_owner_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / ".agents/ai-config.toml"
            manifest.parent.mkdir(parents=True)
            shutil.copy2(ROOT / ".agents/ai-config.toml", manifest)
            for field in ("bootstrap_owner", "validation_owner", "validation_guidance", "decision_log"):
                value = core.EXPECTED_MANIFEST[field]
                assert isinstance(value, str)  # noqa: S101
                path = root / value
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("owner\n")
            for field in ("context_resolution_skill", "review_guard_skill"):
                value = core.EXPECTED_MANIFEST[field]
                assert isinstance(value, str)  # noqa: S101
                path = root / ".agents/skills" / value / "SKILL.md"
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("skill\n")
            (root / "docs/DECISIONS.md").unlink()
            errors = core.validate_manifest(root)
        self.assertTrue(any("decision_log" in error for error in errors))

    def test_non_neutral_skill_metadata_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            skill = root / ".agents/skills/example"
            skill.mkdir(parents=True)
            (skill / "SKILL.md").write_text(
                "---\nname: example\ndescription: Example.\nmodel: provider\n"
                'metadata: {"authority":"authoring","review_guard":"none"}\n---\n'
            )
            errors = core.validate_skill(root, skill)
        self.assertTrue(any("unsupported frontmatter" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
