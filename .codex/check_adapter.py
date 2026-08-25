#!/usr/bin/env python3
"""Validate the Codex host adapter and native core discovery."""

from __future__ import annotations

import sys
from pathlib import Path, PurePosixPath
from typing import Any

import tomllib

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / ".github" / "scripts"))

from ai_config.adapters import validate_adapters  # noqa: E402
from ai_config.boundaries import validate_boundaries  # noqa: E402
from ai_config.common import repository_paths  # noqa: E402

EXPECTED_CONFIG: dict[str, Any] = {
    "model": "gpt-5.6",
    "model_reasoning_effort": "xhigh",
    "approval_policy": "never",
    "sandbox_mode": "danger-full-access",
    "web_search": "live",
    "agents": {
        "enabled": True,
        "default_subagent_model": "gpt-5.6",
        "default_subagent_reasoning_effort": "max",
    },
}
SCHEMA_LINE = "#:schema https://developers.openai.com/codex/config-schema.json"
REQUIRED_INSTRUCTIONS = {
    PurePosixPath("AGENTS.md"),
    PurePosixPath("keyless-python/AGENTS.md"),
    PurePosixPath("keyless-rust/AGENTS.md"),
    PurePosixPath("keyless-ts/AGENTS.md"),
}
WORKFLOW = PurePosixPath(".github/workflows/check-codex-adapter.yml")


def validate_contract() -> list[str]:
    adapters, errors = validate_adapters(ROOT)
    if [adapter.identifier for adapter in adapters].count("codex") != 1:
        errors.append("exactly one Codex adapter manifest is required")
    errors.extend(validate_boundaries(ROOT, adapters))
    return errors


def validate_config() -> list[str]:
    path = ROOT / ".codex" / "config.toml"
    try:
        text = path.read_text(encoding="utf8")
        document = tomllib.loads(text)
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"Codex project config is missing or invalid TOML: {error}"]
    errors = (
        []
        if text.splitlines()[:1] == [SCHEMA_LINE]
        else ["Codex config must declare the official schema"]
    )
    if document != EXPECTED_CONFIG:
        errors.append("Codex config must match the closed host posture")
    return errors


def validate_native_discovery(
    paths: tuple[PurePosixPath, ...] | None = None,
) -> list[str]:
    paths = repository_paths(ROOT) if paths is None else paths
    available = set(paths)
    errors = [
        f"Codex native instruction source is missing: {path}"
        for path in sorted(REQUIRED_INSTRUCTIONS - available)
    ]
    errors.extend(
        f"Codex must use native AGENTS.md discovery, not a shim: {path}"
        for path in paths
        if path.name == "CODEX.md"
    )
    for name in ("agents", "skills", "hooks", "rules"):
        path = ROOT / ".codex" / name
        if path.exists() or path.is_symlink():
            errors.append(f"Codex adapter must not define provider-local {name}")
    return errors


def validate_workflow() -> list[str]:
    try:
        text = (ROOT / WORKFLOW).read_text(encoding="utf8")
    except OSError as error:
        return [f"Codex adapter workflow is missing: {error}"]
    required = (
        "persist-credentials: false",
        "python3 .codex/check_adapter.py",
        "python3 -m unittest discover -s .codex -p 'test_*.py' --verbose",
        "python3 .github/scripts/check_ai_config.py --scope adapters",
    )
    errors = [
        f"Codex adapter workflow is missing: {item}"
        for item in required
        if item not in text
    ]
    if "branches:" in text:
        errors.append("Codex adapter workflow must not be restricted to one branch")
    return errors


def validate() -> list[str]:
    return [
        *validate_contract(),
        *validate_config(),
        *validate_native_discovery(),
        *validate_workflow(),
    ]


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"::error::{error}")
        return 1
    print("Codex adapter: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
