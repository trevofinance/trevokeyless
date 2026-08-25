"""Validate high-risk workflow registration and deterministic evaluation corpora."""

from __future__ import annotations

import json
from pathlib import Path

from .common import load_toml

CASE_FIELDS = {
    "schema",
    "id",
    "case_type",
    "prompt",
    "expected_entrypoint",
    "expected_activation",
    "expected_authority",
    "expected_guard",
    "expected_output_contains",
    "allowed_side_effects",
    "forbidden_side_effects",
}


def validate_evaluations(root: Path) -> list[str]:  # noqa: C901, PLR0912, PLR0915
    registry, errors = load_toml(root / ".agents/workflow-risk.toml", "workflow risk registry")
    if registry is None:
        return errors
    workflows = registry.get("workflow")
    required_types = registry.get("required_case_types")
    case_schema = registry.get("case_schema")
    if not isinstance(workflows, list) or not workflows:
        return [*errors, "workflow risk registry must declare workflows"]
    if not isinstance(required_types, list) or not required_types:
        errors.append("workflow risk registry must declare required_case_types")
        required_types = []
    registered: set[str] = set()
    identifiers: set[str] = set()
    case_ids: set[str] = set()
    for workflow in workflows:
        if not isinstance(workflow, dict):
            errors.append("workflow risk entries must be tables")
            continue
        identifier = workflow.get("id")
        suite = workflow.get("suite")
        entrypoint = workflow.get("entrypoint")
        if not isinstance(identifier, str) or not isinstance(suite, str) or not isinstance(entrypoint, str):
            errors.append("workflow risk entry requires string id, suite, and entrypoint")
            continue
        if identifier in identifiers:
            errors.append(f"duplicate workflow risk id: {identifier}")
        identifiers.add(identifier)
        registered.add(suite)
        for skill in [entrypoint, *workflow.get("participants", [])]:
            if not isinstance(skill, str) or not (root / ".agents/skills" / skill / "SKILL.md").is_file():
                errors.append(f"workflow {identifier} references missing canonical skill: {skill}")
        path = root / suite
        try:
            cases = [json.loads(line) for line in path.read_text(encoding="utf8").splitlines() if line.strip()]
        except (OSError, json.JSONDecodeError) as error:
            errors.append(f"workflow eval suite is missing or invalid ({suite}): {error}")
            continue
        actual_types: set[str] = set()
        for case in cases:
            case_id = case.get("id") if isinstance(case, dict) else None
            if not isinstance(case, dict) or set(case) != CASE_FIELDS:
                errors.append(f"workflow eval case has the wrong fields: {suite}:{case_id}")
                continue
            actual_types.add(case["case_type"])
            if case.get("schema") != case_schema:
                errors.append(f"workflow eval schema mismatch: {suite}:{case_id}")
            for field, expected in (
                ("expected_entrypoint", entrypoint),
                ("expected_authority", workflow.get("authority")),
                ("expected_guard", workflow.get("review_guard")),
            ):
                if case.get(field) != expected:
                    errors.append(f"workflow eval {field} mismatch: {suite}:{case_id}")
            if case_id in case_ids:
                errors.append(f"duplicate workflow eval case id: {case_id}")
            elif isinstance(case_id, str):
                case_ids.add(case_id)
        if actual_types != set(required_types):
            errors.append(f"workflow eval case types do not match registry: {suite}")
    discovered = {path.relative_to(root).as_posix() for path in (root / ".agents/evals").glob("*/cases.jsonl")}
    if registered != discovered:
        errors.append("workflow risk registry and integration suites do not match exactly")
    return errors
