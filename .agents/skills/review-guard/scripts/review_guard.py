#!/usr/bin/env python3
"""Snapshot and verify review-only tracked-source and Git-state postconditions."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
GIT_OPERATION_FILES = ("BISECT_LOG", "CHERRY_PICK_HEAD", "MERGE_HEAD", "REBASE_HEAD", "REVERT_HEAD")


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("snapshot", "verify"))
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--snapshot", type=Path, required=True)
    return parser.parse_args()


def git(root: Path, *args: str, allow_failure: bool = False) -> bytes:
    result = subprocess.run(["git", "-C", os.fspath(root), *args], capture_output=True)
    if result.returncode and not allow_failure:
        raise RuntimeError(result.stderr.decode(errors="replace").strip())
    return result.stdout if result.returncode == 0 else b""


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def tracked_worktree(root: Path) -> dict[str, str]:
    state: dict[str, str] = {}
    for encoded in git(root, "ls-files", "-z").split(b"\0"):
        if not encoded:
            continue
        relative = os.fsdecode(encoded)
        path = root / relative
        try:
            metadata = path.lstat()
        except FileNotFoundError:
            state[relative] = "missing"
            continue
        mode = stat.S_IMODE(metadata.st_mode)
        if path.is_symlink():
            payload, kind = os.fsencode(os.readlink(path)), "symlink"
        elif path.is_file():
            payload, kind = path.read_bytes(), "file"
        else:
            payload, kind = b"", "other"
        state[relative] = f"{kind}:{mode:o}:{digest(payload)}"
    return state


def operation_state(git_dir: Path) -> dict[str, str]:
    state = {name: digest((git_dir / name).read_bytes()) if (git_dir / name).is_file() else "absent" for name in GIT_OPERATION_FILES}
    for name in ("rebase-apply", "rebase-merge"):
        path = git_dir / name
        if not path.exists():
            state[name] = "absent"
            continue
        payload = b"".join(os.fsencode(child.relative_to(path).as_posix()) + b"\0" + child.read_bytes() + b"\0" for child in sorted(path.rglob("*")) if child.is_file())
        state[name] = digest(payload)
    return state


def capture(root: Path) -> dict[str, Any]:
    resolved = Path(os.fsdecode(git(root, "rev-parse", "--show-toplevel")).strip()).resolve()
    if resolved != root.resolve():
        raise RuntimeError(f"--repo-root must be Git root: {resolved}")
    git_dir = Path(os.fsdecode(git(root, "rev-parse", "--absolute-git-dir")).strip())
    return {
        "repo_root": os.fspath(resolved),
        "head_oid": os.fsdecode(git(root, "rev-parse", "--verify", "HEAD")).strip(),
        "head_ref": os.fsdecode(git(root, "symbolic-ref", "-q", "HEAD", allow_failure=True)).strip(),
        "refs": digest(git(root, "show-ref", "--head")),
        "index": digest(git(root, "ls-files", "--stage", "-z")),
        "status": digest(git(root, "status", "--porcelain=v2", "-z", "--untracked-files=no")),
        "tracked": tracked_worktree(root),
        "operations": operation_state(git_dir),
    }


def changed(before: dict[str, Any], after: dict[str, Any]) -> list[str]:
    failures = [label for field, label in (("repo_root", "repository root changed"), ("head_oid", "HEAD changed"), ("head_ref", "checked-out ref changed"), ("refs", "Git refs changed"), ("index", "Git index changed"), ("status", "tracked Git status changed"), ("operations", "Git operation state changed")) if before.get(field) != after.get(field)]
    before_files, after_files = before.get("tracked", {}), after.get("tracked", {})
    paths = sorted(path for path in before_files.keys() | after_files.keys() if before_files.get(path) != after_files.get(path))
    if paths:
        failures.append("tracked worktree changed: " + ", ".join(paths))
    return failures


def main() -> int:
    options = arguments()
    try:
        if options.command == "snapshot":
            options.snapshot.write_text(json.dumps({"schema_version": SCHEMA_VERSION, "state": capture(options.repo_root)}, indent=2, sort_keys=True) + "\n")
            print(f"Review baseline recorded: {options.snapshot}")
            return 0
        document = json.loads(options.snapshot.read_text())
        if document.get("schema_version") != SCHEMA_VERSION:
            raise RuntimeError("unsupported snapshot schema")
        failures = changed(document["state"], capture(options.repo_root))
    except (OSError, KeyError, json.JSONDecodeError, RuntimeError) as error:
        print(f"REVIEW POSTCONDITION ERROR: {error}", file=sys.stderr)
        return 2
    if failures:
        print("REVIEW POSTCONDITION FAILED")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print("PASS review introduced no tracked-source or Git-state mutation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
