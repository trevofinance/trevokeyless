"""Validate machine-resolvable core/adapter and cross-adapter dependency edges."""

from __future__ import annotations

import re
from pathlib import Path, PurePosixPath

from .adapters import Adapter
from .common import repository_paths

LINK = re.compile(r"\[[^\]]*\]\(([^)#]+)(?:#[^)]+)?\)")
IMPORT = re.compile(r"(?m)^\s*@([^\s]+)\s*$")


def collapse(path: PurePosixPath) -> PurePosixPath | None:
    """Lexically normalize a relative path, rejecting traversal above the root."""
    parts: list[str] = []
    for part in path.parts:
        if part in {"", "."}:
            continue
        if part == "..":
            if not parts:
                return None
            parts.pop()
        else:
            parts.append(part)
    return PurePosixPath(*parts)


def local_targets(path: PurePosixPath, text: str) -> list[PurePosixPath]:
    targets: list[PurePosixPath] = []
    for raw in [*LINK.findall(text), *IMPORT.findall(text)]:
        if "://" in raw or raw.startswith("#"):
            continue
        candidate = PurePosixPath(raw)
        if candidate.is_absolute():
            continue
        normalized = collapse(PurePosixPath(path.parent, candidate))
        if normalized is not None:
            targets.append(normalized)
    return targets


def validate_symlink(
    root: Path,
    path: PurePosixPath,
    owner: str | None,
    roots: dict[str, str],
) -> list[str]:
    try:
        target = PurePosixPath((root / path).resolve().relative_to(root))
    except (OSError, ValueError):
        return [f"repository symlink escapes the repository: {path}"]
    target_owner = roots.get(target.parts[0]) if target.parts else None
    if owner is None and target_owner is not None:
        return [f"neutral core symlink depends on {target_owner} adapter: {path}"]
    if owner is not None and target_owner is not None and target_owner != owner:
        return [f"{owner} adapter symlink crosses into {target_owner}: {path}"]
    return []


def validate_boundaries(root: Path, adapters: list[Adapter]) -> list[str]:
    errors: list[str] = []
    roots = {adapter.root.parts[0]: adapter.identifier for adapter in adapters}
    external = {
        PurePosixPath(value): adapter.identifier
        for adapter in adapters
        for value in adapter.document.get("external_owned_paths", [])
        if isinstance(value, str)
    }
    for path in repository_paths(root):
        owner = roots.get(path.parts[0]) if path.parts else None
        owner = owner or external.get(path)
        full = root / path
        if full.is_symlink():
            errors.extend(validate_symlink(root, path, owner, roots))
            continue
        if not full.is_file() or full.suffix.lower() not in {".md", ".toml"}:
            continue
        try:
            text = full.read_text(encoding="utf8")
        except UnicodeDecodeError:
            continue
        for normalized in local_targets(path, text):
            target_owner = roots.get(normalized.parts[0]) if normalized.parts else None
            if owner is None and target_owner is not None:
                errors.append(f"neutral core imports or links to {target_owner} adapter: {path}")
            elif owner is not None and target_owner is not None and target_owner != owner:
                errors.append(f"{owner} adapter imports or links to {target_owner}: {path}")
    return errors
