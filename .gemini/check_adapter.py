#!/usr/bin/env python3
"""Validate the Gemini CLI adapter and its explicit fallback surfaces."""

from __future__ import annotations

import sys
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / ".github" / "scripts"))

from ai_config.adapters import validate_adapters  # noqa: E402
from ai_config.boundaries import validate_boundaries  # noqa: E402
from ai_config.common import repository_paths  # noqa: E402

SHIM = """@AGENTS.md

# Gemini-specific

Gemini CLI mechanics and fallback behavior are documented in `.gemini/README.md`. Shared
skills remain canonical under `.agents/skills`.
"""
WORKFLOW = PurePosixPath(".github/workflows/check-gemini-adapter.yml")


def validate_contract() -> list[str]:
    adapters, errors = validate_adapters(ROOT)
    if [adapter.identifier for adapter in adapters].count("gemini") != 1:
        errors.append("exactly one Gemini adapter manifest is required")
    errors.extend(validate_boundaries(ROOT, adapters))
    return errors


def validate_shim(paths: tuple[PurePosixPath, ...] | None = None) -> list[str]:
    paths = repository_paths(ROOT) if paths is None else paths
    shims = tuple(path for path in paths if path.name == "GEMINI.md")
    if shims != (PurePosixPath("GEMINI.md"),):
        return ["Gemini must expose exactly one root import shim"]
    try:
        text = (ROOT / "GEMINI.md").read_text(encoding="utf8")
    except OSError as error:
        return [f"Gemini root shim is missing: {error}"]
    return (
        []
        if text == SHIM
        else ["Gemini root instruction adapter is not the thin import shim"]
    )


def validate_native_skills() -> list[str]:
    canonical = ROOT / ".agents" / "skills"
    errors = [] if canonical.is_dir() else ["Gemini canonical skill source is missing"]
    for name in ("skills", "agents", "hooks", "settings.json"):
        path = ROOT / ".gemini" / name
        if path.exists() or path.is_symlink():
            errors.append(f"Gemini adapter must not define provider-local {name}")
    return errors


def validate_workflow() -> list[str]:
    try:
        text = (ROOT / WORKFLOW).read_text(encoding="utf8")
    except OSError as error:
        return [f"Gemini adapter workflow is missing: {error}"]
    required = (
        "persist-credentials: false",
        "python3 .gemini/check_adapter.py",
        "python3 -m unittest discover -s .gemini -p 'test_*.py' --verbose",
        "python3 .github/scripts/check_ai_config.py --scope adapters",
    )
    errors = [
        f"Gemini adapter workflow is missing: {item}"
        for item in required
        if item not in text
    ]
    if "branches:" in text:
        errors.append("Gemini adapter workflow must not be restricted to one branch")
    return errors


def validate() -> list[str]:
    return [
        *validate_contract(),
        *validate_shim(),
        *validate_native_skills(),
        *validate_workflow(),
    ]


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"::error::{error}")
        return 1
    print("Gemini adapter: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
