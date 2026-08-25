"""Prove that the complete working patch keeps the neutral core adapter-independent."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath

from .adapters import Adapter, discover_adapters
from .common import normalized_path


def run_git(root: Path, arguments: list[str], *, data: bytes | None = None) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(  # noqa: S603
        ["git", *arguments],
        cwd=root,
        input=data,
        capture_output=True,
        check=False,
    )


def copy_untracked(root: Path, checkout: Path) -> list[str]:
    result = run_git(root, ["ls-files", "--others", "--exclude-standard", "-z"])
    if result.returncode:
        return ["core independence could not enumerate untracked working-patch files"]
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        relative = PurePosixPath(os.fsdecode(raw))
        source = root / relative
        target = checkout / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if source.is_symlink():
            target.symlink_to(source.readlink())
        elif source.is_file():
            shutil.copy2(source, target)
    return []


def overlay_working_patch(root: Path, checkout: Path) -> list[str]:
    patch = run_git(root, ["diff", "--binary", "--full-index", "HEAD", "--", "."])
    if patch.returncode:
        return ["core independence could not capture the complete tracked working patch"]
    if patch.stdout:
        applied = run_git(checkout, ["apply", "--binary", "--whitespace=nowarn", "-"], data=patch.stdout)
        if applied.returncode:
            return ["core independence could not apply the tracked working patch in isolation"]
    return copy_untracked(root, checkout)


def adapter_owned_paths(adapter: Adapter) -> tuple[PurePosixPath, ...]:
    paths = [adapter.root]
    external = adapter.document.get("external_owned_paths")
    if isinstance(external, list):
        for value in external:
            try:
                paths.append(normalized_path(value, "external_owned_paths entry"))
            except ValueError:
                continue
    return tuple(sorted(set(paths)))


def remove_path(root: Path, relative: PurePosixPath) -> None:
    path = root / relative
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.is_dir():
        shutil.rmtree(path)


def remove_adapters(checkout: Path, adapters: list[Adapter]) -> None:
    for relative in sorted(
        {path for adapter in adapters for path in adapter_owned_paths(adapter)},
        key=lambda path: (len(path.parts), path.as_posix()),
        reverse=True,
    ):
        remove_path(checkout, relative)


def validate_core_independence(root: Path) -> list[str]:
    if run_git(root, ["rev-parse", "--is-inside-work-tree"]).stdout.strip() != b"true":
        return ["core independence requires a Git working tree"]
    errors: list[str] = []
    with tempfile.TemporaryDirectory(prefix="ai-core-independence-") as directory:
        checkout = Path(directory) / "checkout"
        added = run_git(root, ["worktree", "add", "--detach", os.fspath(checkout), "HEAD"])
        if added.returncode:
            return ["core independence could not create an isolated worktree"]
        try:
            errors.extend(overlay_working_patch(root, checkout))
            if errors:
                return errors
            adapters, discovery_errors = discover_adapters(checkout)
            errors.extend(discovery_errors)
            if errors:
                return errors
            remove_adapters(checkout, adapters)
            result = subprocess.run(
                [sys.executable, ".github/scripts/check_ai_config.py", "--scope", "core"],
                cwd=checkout,
                capture_output=True,
                text=True,
                check=False,
            )
            if result.returncode:
                detail = (result.stderr or result.stdout).strip().splitlines()
                suffix = f": {detail[-1]}" if detail else ""
                errors.append(f"provider-neutral core fails with installed adapters removed{suffix}")
        finally:
            removed = run_git(root, ["worktree", "remove", "--force", os.fspath(checkout)])
            run_git(root, ["worktree", "prune"])
            if removed.returncode:
                errors.append("core independence could not clean up its isolated worktree")
    return errors
