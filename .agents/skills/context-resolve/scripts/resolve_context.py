#!/usr/bin/env python3
"""Resolve deterministic provider-neutral repository context."""

from __future__ import annotations

import argparse
import fnmatch
import json
import sys
import tomllib
from pathlib import Path, PurePosixPath


SCRIPT = Path(__file__).resolve()
SKILL_ROOT = SCRIPT.parents[1]
REPO_ROOT = SCRIPT.parents[4]
MAP_PATH = SKILL_ROOT / "context-map.toml"
FIXTURES_PATH = SKILL_ROOT / "fixtures.toml"


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def normalize(raw: str) -> str:
    candidate = Path(raw)
    if candidate.is_absolute():
        try:
            candidate = candidate.resolve().relative_to(REPO_ROOT)
        except ValueError:
            return candidate.as_posix()
    value = PurePosixPath(candidate).as_posix()
    while value.startswith("./"):
        value = value[2:]
    return value or "."


def matches(path: str, pattern: str) -> bool:
    if pattern.endswith("/**"):
        prefix = pattern[:-3].rstrip("/")
        return path == prefix or path.startswith(prefix + "/")
    return fnmatch.fnmatchcase(path, pattern)


def installed_adapter_roots() -> list[str]:
    manifests = [*REPO_ROOT.glob(".*/adapter.toml"), *REPO_ROOT.glob("*/adapter.toml")]
    return sorted({item.parent.relative_to(REPO_ROOT).as_posix() for item in manifests})


def resolve(config: dict, *, mode: str, paths: list[str], task: str,
            concerns: list[str], adapter_roots: list[str] | None = None) -> dict:
    if mode not in config["modes"]:
        return {"errors": [f"unknown mode: {mode}"]}
    scopes = config.get("scope", [])
    concern_entries = {entry["id"]: entry for entry in config.get("concern", [])}
    normalized_paths = [normalize(path) for path in paths]
    selected_scopes = {"root"}
    errors: list[str] = []
    roots = adapter_roots if adapter_roots is not None else installed_adapter_roots()
    adapter_paths: set[str] = set()

    for path in normalized_paths:
        found = {scope["id"] for scope in scopes if any(matches(path, p) for p in scope.get("patterns", []))}
        if any(path == root or path.startswith(root + "/") for root in roots):
            found.add("root")
            adapter_paths.add(path)
        if not found:
            errors.append(f"UNKNOWN PATH: {path}")
        selected_scopes.update(found)

    lowered_task = " ".join(task.lower().replace("_", "-").split())
    if not normalized_paths:
        inferred = {
            scope["id"] for scope in scopes
            if any(alias.lower() in lowered_task for alias in scope.get("aliases", []))
        }
        selected_scopes.update(inferred)

    selected_concerns = {value.lower().replace("_", "-") for value in concerns}
    for concern_id, entry in concern_entries.items():
        if any(keyword.lower() in lowered_task for keyword in entry.get("keywords", [])):
            selected_concerns.add(concern_id)
    if adapter_paths:
        selected_concerns.add("ai-config")
    unknown_concerns = selected_concerns - concern_entries.keys()
    errors.extend(f"UNKNOWN CONCERN: {value}" for value in sorted(unknown_concerns))
    selected_concerns &= concern_entries.keys()
    if not normalized_paths and not task.strip():
        errors.append("UNRESOLVED SCOPE: provide --path or --task")
    elif not normalized_paths and selected_scopes == {"root"} and not selected_concerns:
        errors.append("UNRESOLVED SCOPE: task did not match a registered alias or concern")

    documents: list[str] = []
    reasons: dict[str, list[str]] = {}

    def add(path: str, reason: str) -> None:
        if path not in documents:
            documents.append(path)
        reasons.setdefault(path, []).append(reason)

    for path in config.get("defaults", {}).get("always_documents", []):
        add(path, "repository baseline")
    by_scope = {entry["id"]: entry for entry in scopes}
    for scope_id in sorted(selected_scopes):
        for path in by_scope[scope_id].get("documents", []):
            add(path, f"{scope_id} scope")
    for concern_id in sorted(selected_concerns):
        for path in concern_entries[concern_id].get("documents", []):
            add(path, f"{concern_id} concern")

    missing = [path for path in documents if not (REPO_ROOT / path).exists()]
    errors.extend(f"MISSING KNOWLEDGE: {path}" for path in missing)
    return {
        "mode": mode,
        "paths": normalized_paths,
        "scopes": sorted(selected_scopes),
        "concerns": sorted(selected_concerns),
        "documents": documents,
        "reasons": reasons,
        "errors": errors,
    }


def render(result: dict) -> str:
    lines = [f"# Context resolution ({result.get('mode', 'unknown')})", ""]
    if result.get("errors"):
        lines.append("## Errors")
        lines.extend(f"- {error}" for error in result["errors"])
        lines.append("")
    lines.extend(("## Scopes", *(f"- {item}" for item in result.get("scopes", [])), "", "## Concerns"))
    lines.extend(f"- {item}" for item in result.get("concerns", []))
    lines.extend(("", "## Mandatory knowledge"))
    for path in result.get("documents", []):
        lines.append(f"- `{path}` — {', '.join(result['reasons'][path])}")
    return "\n".join(lines) + "\n"


def check_fixtures(config: dict) -> int:
    fixtures = load_toml(FIXTURES_PATH).get("fixture", [])
    failures: list[str] = []
    for fixture in fixtures:
        result = resolve(
            config,
            mode=fixture["mode"],
            paths=fixture.get("paths", []),
            task=fixture.get("task", ""),
            concerns=fixture.get("concerns", []),
            adapter_roots=fixture.get("adapter_roots"),
        )
        for field, expected in (("scopes", fixture.get("expect_scopes", [])),
                                ("concerns", fixture.get("expect_concerns", [])),
                                ("documents", fixture.get("expect_documents", []))):
            missing = sorted(set(expected) - set(result.get(field, [])))
            if missing:
                failures.append(f"{fixture['name']}: missing {field}: {', '.join(missing)}")
        if result["errors"]:
            failures.append(f"{fixture['name']}: {'; '.join(result['errors'])}")
    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 1
    print(f"PASS {len(fixtures)} context resolver fixtures")
    return 0


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("understand", "plan", "implement", "review", "validate", "docs", "release"))
    parser.add_argument("--path", action="append", default=[])
    parser.add_argument("--task", default="")
    parser.add_argument("--concern", action="append", default=[])
    parser.add_argument("--format", choices=("markdown", "json"), default="markdown")
    parser.add_argument("--check-fixtures", action="store_true")
    options = parser.parse_args()
    if not options.check_fixtures and not options.mode:
        parser.error("--mode is required unless --check-fixtures is used")
    return options


def main() -> int:
    options = arguments()
    config = load_toml(MAP_PATH)
    if options.check_fixtures:
        return check_fixtures(config)
    result = resolve(config, mode=options.mode, paths=options.path, task=options.task, concerns=options.concern)
    print(json.dumps(result, indent=2, sort_keys=True) if options.format == "json" else render(result), end="\n" if options.format == "json" else "")
    return 1 if result["errors"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
