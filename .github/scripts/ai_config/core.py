"""Validate provider-neutral AI configuration and selected policy bindings."""

from __future__ import annotations

import hashlib
import os
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

from .common import frontmatter, load_toml, repository_paths, run_python_help, skill_metadata
from .evaluations import validate_evaluations

POLICY_HASHES = {
    "docs/AI-CONFIG.md": "30109273a45c558878f8d06574a51a5b736e17a3b28b9eb2ddd92d3ce5ee36c6",
    "docs/ai-config/ADAPTERS.md": "9417699d9cb78bc48aea8cce48e095d6b94d77578d32d12a8e65801bc3068ac7",
    "docs/ai-config/CONTEXT-RESOLUTION.md": "c42c4ce9d32838fc1529d7b3599705fe522d5711d8229e26f373826ff114bfab",
    "docs/ai-config/MIGRATION.md": "e0291c35772dc96dc8360207c11b38f27dea6cfebcc8fe941ecd9b3831bf30cb",
    "docs/ai-config/PROFILES.md": "66de72510bfcc734fd0e1f6b0f0361cd3993e574894377002b2194c6dc567c34",
    "docs/ai-config/REFERENCES.md": "083f58477078ca138e5b585c844f75a35b8cab08df0de8cdbf051a3fdca7c77c",
    "docs/ai-config/SKILLS-AND-DELEGATION.md": "1bb915bebf4d0f17de91e18dea2a1ce50e7f3deecf195b600a8c34b47c463801",
    "docs/ai-config/TEMPLATES.md": "8e336bd7bfbc675d21ba87c95828ebd13824c308a3d1fa8ab753bc2ebccb0ec1",
    "docs/ai-config/VALIDATION.md": "8d53799d5d12ee8acb79d5ac83068763a0459f712f6fd946b7d0c23c04ac8739",
}
EXPECTED_MANIFEST = {
    "schema_version": 1,
    "policy_bundle": "provider-neutral-ai-config",
    "policy_version": "3.0.0",
    "bootstrap_profile": "canonical-router",
    "authority_profile": "review-only-behavioral",
    "delegation_profile": "skill-centered",
    "command_profile": "executable-owner",
    "capability_surface_profile": "conditional-surfaces",
    "validation_profile": "core-and-adapter-contracts",
    "context_resolution_skill": "context-resolve",
    "review_guard_skill": "review-guard",
    "bootstrap_owner": "AGENTS.md",
    "validation_owner": ".github/scripts/check_ai_config.py",
    "validation_guidance": "docs/development.md",
    "decision_log": "docs/DECISIONS.md",
}
REQUIRED_PATHS = {
    ".agents/ai-config.toml",
    ".agents/skills/context-resolve/SKILL.md",
    ".agents/skills/review-guard/SKILL.md",
    ".agents/workflow-risk.toml",
    ".github/scripts/check_ai_config.py",
    ".github/scripts/run_checks.py",
    "AGENTS.md",
    "docs/AI-AGENT-GUIDE.md",
    "docs/DECISIONS.md",
    "docs/DOCUMENTATION.md",
    "docs/ai-configuration.md",
    "keyless-python/AGENTS.md",
    "keyless-rust/AGENTS.md",
    "keyless-ts/AGENTS.md",
}
ALLOWED_FRONTMATTER = {"name", "description", "license", "compatibility", "metadata"}
ALLOWED_AUTHORITIES = {"authoring", "non-authoring", "review-only"}
ALLOWED_GUARDS = {"none", "required-at-top-level"}
SKILL_PATH_PARTS = 4
CODE_REFERENCE = re.compile(r"`([^`\n]+)`")
COMPOSED_SKILL = re.compile(r"`([a-z0-9]+(?:-[a-z0-9]+)*)` skill\b")
STALE_TERMS = ("trevo/", "trevo-skill-", "polkadot-sdk", "system-parachains", "relay/paseo")
STALE_SURFACES = (
    ".agents/ai-config.toml",
    "AGENTS.md",
    "docs/AI-AGENT-GUIDE.md",
    "docs/DECISIONS.md",
    "docs/DOCUMENTATION.md",
    "docs/ai-configuration.md",
)
CONDITIONAL_SURFACES = (
    ".agents/rules",
    ".agents/roles",
    ".agents/hooks",
    ".agents/integrations",
    ".agents/generated",
    ".agents/catalogs",
)


def validate_policy(root: Path) -> list[str]:
    errors: list[str] = []
    for relative, expected in POLICY_HASHES.items():
        path = root / relative
        if not path.is_file():
            errors.append(f"immutable policy file is missing: {relative}")
        elif hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            errors.append(f"immutable policy file changed: {relative}")
    return errors


def validate_manifest(root: Path) -> list[str]:
    document, errors = load_toml(root / ".agents/ai-config.toml", "AI configuration manifest")
    if document is None:
        return errors
    if document != EXPECTED_MANIFEST:
        errors.append("AI configuration manifest does not match the selected repository profile")
    for field in ("bootstrap_owner", "validation_owner", "validation_guidance", "decision_log"):
        value = document.get(field)
        if isinstance(value, str) and not (root / value).exists():
            errors.append(f"AI configuration owner is missing ({field}): {value}")
    for field in ("context_resolution_skill", "review_guard_skill"):
        value = document.get(field)
        if isinstance(value, str) and not (root / ".agents/skills" / value / "SKILL.md").is_file():
            errors.append(f"AI configuration selected skill is missing ({field}): {value}")
    return errors


def skill_reference(path: PurePosixPath, rendered: str) -> PurePosixPath | None:
    rendered = rendered.split("#", 1)[0]
    if not rendered or "://" in rendered or "<" in rendered or ">" in rendered or " " in rendered:
        return None
    candidate = PurePosixPath(rendered)
    if candidate.is_absolute() or candidate.suffix.lower() not in {
        ".md",
        ".toml",
        ".yml",
        ".yaml",
        ".py",
        ".json",
        ".jsonl",
    }:
        return None
    if "/" not in rendered:
        if rendered == "AGENTS.md":
            return candidate
        if path.parts[:2] == (".agents", "skills") and rendered != "adapter.toml":
            return path.parent / candidate
        return None
    return candidate if path.parts[:2] == (".agents", "skills") or len(path.parts) == 1 else path.parent / candidate


def validate_references(root: Path, paths: tuple[PurePosixPath, ...]) -> list[str]:
    errors: list[str] = []
    sources = [
        path
        for path in paths
        if path.name == "AGENTS.md" or (path.parts[:2] == (".agents", "skills") and path.name == "SKILL.md")
    ]
    for source in sources:
        for match in CODE_REFERENCE.finditer((root / source).read_text(encoding="utf8")):
            reference = skill_reference(source, match.group(1))
            if reference is not None and not (root / reference).exists():
                errors.append(f"canonical AI file references missing path ({reference}): {source}")
    return errors


def validate_skill(root: Path, directory: Path) -> list[str]:  # noqa: C901, PLR0912
    errors: list[str] = []
    relative = directory.relative_to(root)
    skill_file = directory / "SKILL.md"
    if directory.is_symlink() or not skill_file.is_file():
        return [f"canonical skill package is invalid: {relative}"]
    try:
        parsed = frontmatter(skill_file)
    except ValueError as error:
        return [str(error)]
    if parsed is None:
        return [f"canonical skill has invalid frontmatter: {relative}/SKILL.md"]
    scalars, body = parsed
    unsupported = set(scalars) - ALLOWED_FRONTMATTER
    if unsupported:
        errors.append(
            f"canonical skill has unsupported frontmatter ({', '.join(sorted(unsupported))}): {relative}/SKILL.md"
        )
    if scalars.get("name") != directory.name:
        errors.append(f"canonical skill name must match directory: {relative}/SKILL.md")
    if not scalars.get("description"):
        errors.append(f"canonical skill needs a description: {relative}/SKILL.md")
    metadata, metadata_errors = skill_metadata(scalars.get("metadata", ""), relative / "SKILL.md")
    errors.extend(metadata_errors)
    if metadata is not None:
        if set(metadata) != {"authority", "review_guard"}:
            errors.append(f"canonical skill metadata fields are invalid: {relative}/SKILL.md")
        if metadata.get("authority") not in ALLOWED_AUTHORITIES:
            errors.append(f"canonical skill authority is invalid: {relative}/SKILL.md")
        if metadata.get("review_guard") not in ALLOWED_GUARDS:
            errors.append(f"canonical skill review_guard is invalid: {relative}/SKILL.md")
    for composed in COMPOSED_SKILL.findall(body):
        if not (root / ".agents/skills" / composed / "SKILL.md").is_file():
            errors.append(f"canonical skill composes missing skill ({composed}): {relative}/SKILL.md")
    scripts = directory / "scripts"
    manifest = directory / "scripts.toml"
    if scripts.is_dir() != manifest.is_file():
        errors.append(f"canonical skill scripts and scripts.toml must exist together: {relative}")
    if manifest.is_file():
        document, parse_errors = load_toml(manifest, f"skill script manifest {relative}")
        errors.extend(parse_errors)
        if document is not None:
            if set(document) != {"schema_version", "entrypoints"} or document.get("schema_version") != 1:
                errors.append(f"skill script manifest fields are invalid: {relative}/scripts.toml")
            entries = document.get("entrypoints")
            if not isinstance(entries, list) or not entries or entries != sorted(set(entries)):
                errors.append(f"skill entrypoints must be sorted and duplicate-free: {relative}/scripts.toml")
            else:
                discovered = sorted(path.relative_to(directory).as_posix() for path in scripts.glob("*.py"))
                if entries != discovered:
                    errors.append(f"skill entrypoints do not match scripts: {relative}")
                for entry in entries:
                    errors.extend(run_python_help(root, PurePosixPath(relative, entry)))
    return errors


def validate_skills(root: Path, paths: tuple[PurePosixPath, ...]) -> list[str]:
    errors: list[str] = []
    invalid = [
        path
        for path in paths
        if path.name == "SKILL.md" and (len(path.parts) != SKILL_PATH_PARTS or path.parts[:2] != (".agents", "skills"))
    ]
    errors.extend(f"skill definitions are allowed only at .agents/skills/<name>/SKILL.md: {path}" for path in invalid)
    skills_root = root / ".agents/skills"
    directories = sorted(path for path in skills_root.iterdir() if path.is_dir()) if skills_root.is_dir() else []
    if not directories:
        return [*errors, "no canonical skills found"]
    for directory in directories:
        errors.extend(validate_skill(root, directory))
    return errors


def validate_resolver(root: Path) -> list[str]:
    script = root / ".agents/skills/context-resolve/scripts/resolve_context.py"
    result = subprocess.run(  # noqa: S603
        [sys.executable, os.fspath(script), "--check-fixtures"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    return (
        []
        if result.returncode == 0
        else [f"context resolver fixtures failed: {(result.stderr or result.stdout).strip()}"]
    )


def validate_repository_binding(root: Path) -> list[str]:
    errors: list[str] = []
    for relative in STALE_SURFACES:
        text = (root / relative).read_text(encoding="utf8")
        for term in STALE_TERMS:
            if term in text:
                errors.append(f"repository-owned AI configuration contains stale reference term ({term}): {relative}")
    for relative in CONDITIONAL_SURFACES:
        path = root / relative
        if path.is_dir() and not any(path.iterdir()):
            errors.append(f"conditional AI configuration surface must not be empty: {relative}")
    return errors


def validate_duplicate_documents(root: Path) -> list[str]:
    digests: dict[str, str] = {}
    errors: list[str] = []
    excluded = set(POLICY_HASHES)
    for path in sorted((root / "docs").glob("*.md")):
        relative = path.relative_to(root).as_posix()
        if relative in excluded:
            continue
        normalized = re.sub(r"\s+", " ", path.read_text(encoding="utf8")).strip().lower()
        digest = hashlib.sha256(normalized.encode()).hexdigest()
        if digest in digests:
            errors.append(
                f"repository-owned canonical documents duplicate normalized content: {digests[digest]}, {relative}"
            )
        digests[digest] = relative
    return errors


def validate_core(root: Path) -> list[str]:
    paths = repository_paths(root)
    errors = [
        f"required provider-neutral AI configuration is missing: {path}"
        for path in sorted(REQUIRED_PATHS)
        if not (root / path).exists()
    ]
    errors.extend(validate_policy(root))
    errors.extend(validate_manifest(root))
    errors.extend(validate_skills(root, paths))
    errors.extend(validate_references(root, paths))
    errors.extend(validate_resolver(root))
    errors.extend(validate_evaluations(root))
    errors.extend(validate_repository_binding(root))
    errors.extend(validate_duplicate_documents(root))
    return errors
