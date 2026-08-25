#!/usr/bin/env python3
"""Validate the Claude host adapter and its one-way core dependency."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / ".github" / "scripts"))

from ai_config.adapters import validate_adapters  # noqa: E402
from ai_config.boundaries import validate_boundaries  # noqa: E402
from ai_config.common import repository_paths  # noqa: E402

ROOT_SHIM = """@AGENTS.md

# Claude-specific

Claude Code settings and skill discovery are documented in
`.claude/README.md`. Shared skill definitions are canonical under `.agents/skills`.
"""
PACKAGE_SHIM = """@AGENTS.md

# Claude-specific

No additional Claude-specific instructions.
"""
EXPECTED_SETTINGS: dict[str, Any] = {
    "$schema": "https://json.schemastore.org/claude-code-settings.json",
    "model": "opus",
    "effortLevel": "xhigh",
    "alwaysThinkingEnabled": True,
    "teammateDefaultModel": "opus",
    "disableSkillShellExecution": False,
    "skipWebFetchPreflight": True,
    "env": {"CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"},
    "permissions": {"defaultMode": "bypassPermissions"},
    "sandbox": {"enabled": False},
}
WORKFLOW = PurePosixPath(".github/workflows/check-claude-adapter.yml")


def validate_contract() -> list[str]:
    adapters, errors = validate_adapters(ROOT)
    if [adapter.identifier for adapter in adapters].count("claude") != 1:
        errors.append("exactly one Claude adapter manifest is required")
    errors.extend(validate_boundaries(ROOT, adapters))
    return errors


def validate_shims(paths: tuple[PurePosixPath, ...] | None = None) -> list[str]:
    paths = repository_paths(ROOT) if paths is None else paths
    available = set(paths)
    errors: list[str] = []
    for agents in (path for path in paths if path.name == "AGENTS.md"):
        shim = agents.with_name("CLAUDE.md")
        if shim not in available:
            errors.append(
                f"provider-neutral instructions have no Claude shim: {agents}"
            )
    for shim in (path for path in paths if path.name == "CLAUDE.md"):
        if shim.with_name("AGENTS.md") not in available:
            errors.append(f"Claude shim has no sibling AGENTS.md: {shim}")
            continue
        expected = ROOT_SHIM if shim == PurePosixPath("CLAUDE.md") else PACKAGE_SHIM
        if (ROOT / shim).read_text(encoding="utf8") != expected:
            errors.append(
                f"Claude instruction adapter is not a thin import shim: {shim}"
            )
    return errors


def canonical_skills() -> tuple[str, ...]:
    base = ROOT / ".agents" / "skills"
    return tuple(
        sorted(path.name for path in base.iterdir() if (path / "SKILL.md").is_file())
    )


def validate_skills() -> list[str]:
    expected = canonical_skills()
    discovery = ROOT / ".claude" / "skills"
    if discovery.is_symlink() or not discovery.is_dir():
        return ["Claude skill discovery must be a real directory of per-skill symlinks"]
    actual = tuple(sorted(path.name for path in discovery.iterdir()))
    errors = (
        []
        if actual == expected
        else ["Claude skill mirror must exactly match canonical skills"]
    )
    for skill in expected:
        link = discovery / skill
        target = f"../../.agents/skills/{skill}"
        if not link.is_symlink() or os.readlink(link) != target:
            errors.append(
                f"Claude skill link must target the canonical package: {skill}"
            )
    return errors


def validate_settings() -> list[str]:
    path = ROOT / ".claude" / "settings.json"
    try:
        document = json.loads(path.read_text(encoding="utf8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"Claude settings are missing or invalid JSON: {error}"]
    return (
        []
        if document == EXPECTED_SETTINGS
        else ["Claude settings must match the closed host posture"]
    )


def validate_surfaces() -> list[str]:
    errors: list[str] = []
    for name in ("agents", "hooks", "rules"):
        path = ROOT / ".claude" / name
        if path.exists() or path.is_symlink():
            errors.append(f"Claude adapter must not define provider-local {name}")
    return errors


def validate_workflow() -> list[str]:
    try:
        text = (ROOT / WORKFLOW).read_text(encoding="utf8")
    except OSError as error:
        return [f"Claude adapter workflow is missing: {error}"]
    required = (
        "persist-credentials: false",
        "python3 .claude/check_adapter.py",
        "python3 -m unittest discover -s .claude -p 'test_*.py' --verbose",
        "python3 .github/scripts/check_ai_config.py --scope adapters",
    )
    errors = [
        f"Claude adapter workflow is missing: {item}"
        for item in required
        if item not in text
    ]
    if "branches:" in text:
        errors.append("Claude adapter workflow must not be restricted to one branch")
    return errors


def validate() -> list[str]:
    return [
        *validate_contract(),
        *validate_shims(),
        *validate_skills(),
        *validate_settings(),
        *validate_surfaces(),
        *validate_workflow(),
    ]


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"::error::{error}")
        return 1
    print("Claude adapter: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
