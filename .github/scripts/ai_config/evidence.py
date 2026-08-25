"""Bind non-secret live-smoke evidence to its complete configuration inputs."""

from __future__ import annotations

import hashlib
import os
import re
from pathlib import Path, PurePosixPath
from typing import Any

from .common import load_toml, normalized_path, repository_paths

LIVE_RESULTS = {"PASS", "FAIL", "NOT_RUN", "INDETERMINATE"}
PROBES = {"root_context", "nested_context", "portable_skill"}
DATE = re.compile(r"\d{4}-\d{2}-\d{2}")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}")


def selected_inputs(
    root: Path,
    adapter_root: PurePosixPath,
    document: dict[str, Any],
) -> tuple[PurePosixPath, ...]:
    skill_source = normalized_path(document.get("skill_source"), "adapter skill_source")
    evidence = normalized_path(document.get("live_evidence"), "adapter live_evidence")
    external_values = document.get("external_owned_paths")
    external = (
        {normalized_path(value, "external_owned_paths entry") for value in external_values}
        if isinstance(external_values, list)
        else set()
    )
    fixed = {
        PurePosixPath(".agents/ai-config.toml"),
        PurePosixPath(".agents/workflow-risk.toml"),
    }
    selected: list[PurePosixPath] = []
    for path in repository_paths(root):
        if path == evidence or not (root / path).exists():
            continue
        if (
            path.name == "AGENTS.md"
            or path in fixed
            or path in external
            or path.parts[: len(adapter_root.parts)] == adapter_root.parts
            or path.parts[: len(skill_source.parts)] == skill_source.parts
        ):
            selected.append(path)
    return tuple(sorted(set(selected)))


def evidence_digest(
    root: Path,
    adapter_root: PurePosixPath,
    document: dict[str, Any],
) -> str:
    digest = hashlib.sha256()
    for relative in selected_inputs(root, adapter_root, document):
        path = root / relative
        digest.update(relative.as_posix().encode())
        digest.update(b"\0")
        if path.is_symlink():
            digest.update(b"link\0")
            digest.update(os.fsencode(path.readlink()))
        else:
            digest.update(b"file\0")
            digest.update(path.read_bytes())
        digest.update(b"\0")
    return f"sha256:{digest.hexdigest()}"


def validate_header(document: dict[str, Any], adapter_id: str, expected_digest: str) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1 or document.get("adapter") != adapter_id:
        errors.append(f"{adapter_id} live evidence identity or schema is invalid")
    client_version = document.get("client_version")
    if not isinstance(client_version, str) or not client_version:
        errors.append(f"{adapter_id} live evidence client_version must be non-empty")
    recorded_at = document.get("recorded_at")
    if not isinstance(recorded_at, str) or DATE.fullmatch(recorded_at) is None:
        errors.append(f"{adapter_id} live evidence recorded_at must be YYYY-MM-DD")
    input_digest = document.get("input_digest")
    if not isinstance(input_digest, str) or DIGEST.fullmatch(input_digest) is None:
        errors.append(f"{adapter_id} live evidence input_digest must be a SHA-256 binding")
    elif input_digest != expected_digest:
        errors.append(f"{adapter_id} live evidence is stale for the selected configuration inputs")
    return errors


def validate_probes(probes: object, adapter_id: str) -> list[str]:
    if not isinstance(probes, dict) or set(probes) != PROBES:
        return [f"{adapter_id} live evidence must declare exactly the R, N, and S probes"]
    errors: list[str] = []
    for name, probe in probes.items():
        if not isinstance(probe, dict) or set(probe) != {"result", "summary"}:
            errors.append(f"{adapter_id} live evidence probe has invalid fields: {name}")
            continue
        if probe.get("result") not in LIVE_RESULTS:
            errors.append(f"{adapter_id} live evidence probe has invalid result: {name}")
        if not isinstance(probe.get("summary"), str) or not probe["summary"]:
            errors.append(f"{adapter_id} live evidence probe needs a non-secret summary: {name}")
    return errors


def validate_evidence(
    path: Path,
    adapter_id: str,
    expected_digest: str,
) -> list[str]:
    if not path.exists():
        return [f"{adapter_id} live evidence is missing: {path.name}"]
    document, errors = load_toml(path, f"{adapter_id} live evidence")
    if document is None:
        return errors
    expected = {"schema_version", "adapter", "client_version", "recorded_at", "input_digest", "probe"}
    if set(document) != expected:
        errors.append(f"{adapter_id} live evidence must contain exactly {', '.join(sorted(expected))}")
        return errors
    errors.extend(validate_header(document, adapter_id, expected_digest))
    errors.extend(validate_probes(document.get("probe"), adapter_id))
    return errors
