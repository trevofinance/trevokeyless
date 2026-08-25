"""Regression tests for generic adapter manifests and boundaries."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path, PurePosixPath

import tomllib

from ai_config.adapters import Adapter, validate_adapters
from ai_config.boundaries import validate_boundaries
from ai_config.evidence import evidence_digest

MANIFEST = """schema_version = 1
id = "demo"
core_entry = "AGENTS.md"
skill_source = ".agents/skills"
static_check = ".demo/check_adapter.py"
mutation_test = ".demo/test_adapter.py"
live_smoke = ".demo/SMOKE.md"
live_evidence = ".demo/live-smoke-evidence.toml"
boundary_signature = "adapter-to-core-only/v1"
external_owned_paths = []

[support]
root_context = "native"
nested_context = "fallback"
portable_skills = "native"
delegation = "limited"
"""


class AdapterContractTests(unittest.TestCase):
    def create_adapter(self, root: Path) -> None:
        (root / ".agents/skills").mkdir(parents=True)
        (root / "AGENTS.md").write_text("core\n")
        adapter = root / ".demo"
        adapter.mkdir()
        (adapter / "adapter.toml").write_text(MANIFEST)
        for name in ("check_adapter.py", "test_adapter.py", "SMOKE.md"):
            (adapter / name).write_text("adapter\n")
        document = tomllib.loads(MANIFEST)
        digest = evidence_digest(root, PurePosixPath(".demo"), document)
        (adapter / "live-smoke-evidence.toml").write_text(
            f"""schema_version = 1
adapter = "demo"
client_version = "UNAVAILABLE"
recorded_at = "2026-08-24"
input_digest = "{digest}"

[probe.root_context]
result = "NOT_RUN"
summary = "Client unavailable."

[probe.nested_context]
result = "NOT_RUN"
summary = "Client unavailable."

[probe.portable_skill]
result = "NOT_RUN"
summary = "Client unavailable."
"""
        )

    def test_valid_generic_adapter_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_adapter(root)
            adapters, errors = validate_adapters(root)
        self.assertEqual(errors, [])
        self.assertEqual([adapter.identifier for adapter in adapters], ["demo"])

    def test_legacy_json_manifest_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            legacy = root / ".demo/adapter.json"
            legacy.parent.mkdir()
            legacy.write_text("{}\n")
            _, errors = validate_adapters(root)
        self.assertTrue(any("legacy adapter.json" in error for error in errors))

    def test_neutral_link_into_adapter_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_adapter(root)
            (root / "CORE.md").write_text("[bad](.demo/SMOKE.md)\n")
            adapter = Adapter(PurePosixPath(".demo"), "demo", {"external_owned_paths": []})
            errors = validate_boundaries(root, [adapter])
        self.assertTrue(any("neutral core imports or links" in error for error in errors))

    def test_parent_segments_cannot_hide_adapter_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_adapter(root)
            docs = root / "docs"
            docs.mkdir()
            (docs / "CORE.md").write_text("[bad](../.demo/SMOKE.md)\n")
            adapter = Adapter(PurePosixPath(".demo"), "demo", {"external_owned_paths": []})
            errors = validate_boundaries(root, [adapter])
        self.assertTrue(any("neutral core imports or links" in error for error in errors))

    def test_configuration_change_makes_live_evidence_stale(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_adapter(root)
            (root / ".demo/SMOKE.md").write_text("changed procedure\n")
            _, errors = validate_adapters(root)
        self.assertTrue(any("live evidence is stale" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
