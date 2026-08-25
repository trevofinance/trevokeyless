"""Discover and validate provider adapter contracts without enumerating providers."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

from .common import load_toml, normalized_path, sorted_unique_strings
from .evidence import evidence_digest, validate_evidence

SUPPORT_STATUSES = {"native", "fallback", "limited", "unsupported"}
MANIFEST_KEYS = {
    "schema_version",
    "id",
    "core_entry",
    "skill_source",
    "static_check",
    "mutation_test",
    "live_smoke",
    "live_evidence",
    "boundary_signature",
    "external_owned_paths",
    "support",
}


@dataclass(frozen=True)
class Adapter:
    root: PurePosixPath
    identifier: str
    document: dict[str, Any]


def discover_adapters(root: Path) -> tuple[list[Adapter], list[str]]:
    errors: list[str] = []
    adapters: list[Adapter] = []
    manifests = sorted({*root.glob(".*/adapter.toml"), *root.glob("*/adapter.toml")})
    for path in manifests:
        document, parse_errors = load_toml(path, f"adapter manifest {path.relative_to(root)}")
        errors.extend(parse_errors)
        if document is None:
            continue
        identifier = document.get("id")
        if not isinstance(identifier, str):
            errors.append(f"adapter manifest id must be a string: {path.relative_to(root)}")
            continue
        adapters.append(Adapter(PurePosixPath(path.parent.relative_to(root).as_posix()), identifier, document))
    for legacy in sorted(root.glob(".*/adapter.json")):
        errors.append(f"legacy adapter.json must be migrated to adapter.toml: {legacy.relative_to(root)}")
    identifiers = [adapter.identifier for adapter in adapters]
    if len(identifiers) != len(set(identifiers)):
        errors.append("adapter identifiers must be unique")
    return adapters, errors


def manifest_path(document: dict[str, Any], field: str, errors: list[str]) -> PurePosixPath | None:
    try:
        return normalized_path(document.get(field), f"adapter {field}")
    except ValueError as error:
        errors.append(str(error))
        return None


def validate_adapter_evidence(
    root: Path,
    adapter: Adapter,
    evidence: PurePosixPath,
) -> list[str]:
    try:
        digest = evidence_digest(root, adapter.root, adapter.document)
    except ValueError as error:
        return [f"{adapter.identifier} live evidence inputs are invalid: {error}"]
    return validate_evidence(root / evidence, adapter.identifier, digest)


def validate_adapter(root: Path, adapter: Adapter) -> list[str]:  # noqa: C901, PLR0912
    document = adapter.document
    errors: list[str] = []
    if set(document) != MANIFEST_KEYS:
        errors.append(f"{adapter.identifier} adapter manifest fields do not match schema")
    if document.get("schema_version") != 1:
        errors.append(f"{adapter.identifier} adapter schema_version must be 1")
    if not adapter.identifier or adapter.identifier.lower() != adapter.identifier:
        errors.append("adapter id must be non-empty lowercase text")
    expected_root = f".{adapter.identifier}"
    if adapter.root.as_posix() != expected_root:
        errors.append(f"adapter {adapter.identifier} must live at {expected_root}")
    paths: dict[str, PurePosixPath] = {}
    for field in ("core_entry", "skill_source", "static_check", "mutation_test", "live_smoke", "live_evidence"):
        path = manifest_path(document, field, errors)
        if path is not None:
            paths[field] = path
    for field in ("core_entry", "skill_source", "static_check", "mutation_test", "live_smoke"):
        path = paths.get(field)
        if path is not None and not (root / path).exists():
            errors.append(f"{adapter.identifier} adapter {field} is missing: {path}")
    for field in ("static_check", "mutation_test", "live_smoke", "live_evidence"):
        path = paths.get(field)
        if path is not None and path.parts[:1] != adapter.root.parts[:1]:
            errors.append(f"{adapter.identifier} adapter must own {field}: {path}")
    if document.get("boundary_signature") != "adapter-to-core-only/v1":
        errors.append(f"{adapter.identifier} adapter boundary signature is invalid")
    support = document.get("support")
    if not isinstance(support, dict) or set(support) != {
        "root_context",
        "nested_context",
        "portable_skills",
        "delegation",
    }:
        errors.append(f"{adapter.identifier} adapter must declare four support outcomes")
    else:
        for outcome, status in support.items():
            if status not in SUPPORT_STATUSES:
                errors.append(f"{adapter.identifier} adapter support status is invalid: {outcome}")
    try:
        external = sorted_unique_strings(document.get("external_owned_paths"), "external_owned_paths", allow_empty=True)
    except ValueError as error:
        errors.append(f"{adapter.identifier} {error}")
        external = ()
    for value in external:
        try:
            path = normalized_path(value, "external_owned_paths entry")
        except ValueError as error:
            errors.append(f"{adapter.identifier} {error}")
            continue
        if path.parts[:1] == adapter.root.parts[:1]:
            errors.append(f"{adapter.identifier} external_owned_paths must be outside its directory: {path}")
        if not (root / path).exists():
            errors.append(f"{adapter.identifier} external owned path is missing: {path}")
    evidence = paths.get("live_evidence")
    if evidence is not None:
        errors.extend(validate_adapter_evidence(root, adapter, evidence))
    return errors


def validate_adapters(root: Path) -> tuple[list[Adapter], list[str]]:
    adapters, errors = discover_adapters(root)
    for adapter in adapters:
        errors.extend(validate_adapter(root, adapter))
    return adapters, errors
