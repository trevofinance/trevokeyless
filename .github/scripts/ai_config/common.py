"""Shared path, parsing, and process helpers for AI configuration checks."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Any, cast

import tomllib


def load_toml(path: Path, label: str) -> tuple[dict[str, Any] | None, list[str]]:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle), []
    except (OSError, tomllib.TOMLDecodeError) as error:
        return None, [f"{label} is missing or invalid TOML: {error}"]


def repository_paths(root: Path, *, include_untracked: bool = True) -> tuple[PurePosixPath, ...]:
    arguments = ["git", "ls-files", "--cached"]
    if include_untracked:
        arguments.extend(("--others", "--exclude-standard"))
    arguments.append("-z")
    result = subprocess.run(  # noqa: S603
        arguments, cwd=root, capture_output=True, check=False
    )
    if result.returncode == 0:
        return tuple(PurePosixPath(os.fsdecode(value)) for value in result.stdout.split(b"\0") if value)
    ignored = {".git", ".mypy_cache", ".pytest_cache", ".ruff_cache", ".venv", "__pycache__"}
    return tuple(
        sorted(
            PurePosixPath(path.relative_to(root).as_posix())
            for path in root.rglob("*")
            if path.is_file() and not ignored.intersection(path.relative_to(root).parts)
        )
    )


def normalized_path(value: object, field: str) -> PurePosixPath:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field} must be a non-empty repository-relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or path.as_posix() != value:
        raise ValueError(f"{field} must be normalized and repository-relative: {value}")
    return path


def frontmatter(path: Path) -> tuple[dict[str, str], str] | None:
    lines = path.read_text(encoding="utf8").splitlines(keepends=True)
    if not lines or lines[0].rstrip("\r\n") != "---":
        return None
    try:
        end = next(index for index, line in enumerate(lines[1:], 1) if line.rstrip("\r\n") == "---")
    except StopIteration:
        return None
    scalars: dict[str, str] = {}
    for line in lines[1:end]:
        stripped = line.rstrip("\r\n")
        if stripped and not stripped[0].isspace() and ":" in stripped:
            key, _, value = stripped.partition(":")
            if key in scalars:
                raise ValueError(f"duplicate frontmatter key {key}: {path}")
            scalars[key] = value.strip()
    return scalars, "".join(lines[end + 1 :])


def skill_metadata(value: str, path: Path) -> tuple[dict[str, str] | None, list[str]]:
    if not value:
        return None, [f"canonical skill must declare metadata: {path}"]
    try:
        document = json.loads(value)
    except json.JSONDecodeError as error:
        return None, [f"canonical skill metadata must be inline JSON: {path}: {error}"]
    if not isinstance(document, dict) or any(
        not isinstance(key, str) or not isinstance(item, str) for key, item in document.items()
    ):
        return None, [f"canonical skill metadata must be a string map: {path}"]
    return cast("dict[str, str]", document), []


def run_python_help(root: Path, path: PurePosixPath) -> list[str]:
    result = subprocess.run(  # noqa: S603
        [sys.executable, os.fspath(root / path), "--help"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        detail = (result.stderr or result.stdout).strip().splitlines()
        suffix = f": {detail[-1]}" if detail else ""
        return [f"skill entrypoint --help failed ({path}){suffix}"]
    return []


def sorted_unique_strings(value: object, field: str, *, allow_empty: bool = False) -> tuple[str, ...]:
    if not isinstance(value, list) or (not value and not allow_empty):
        raise ValueError(f"{field} must be {'a' if allow_empty else 'a non-empty'} list")
    if any(not isinstance(item, str) or not item for item in value):
        raise ValueError(f"{field} entries must be non-empty strings")
    strings = cast("list[str]", value)
    if strings != sorted(set(strings)):
        raise ValueError(f"{field} must be sorted and duplicate-free")
    return tuple(strings)
