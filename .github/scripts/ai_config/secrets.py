"""Reject credential and transcript material from tracked AI configuration surfaces."""

from __future__ import annotations

import re
from pathlib import Path, PurePosixPath

from .adapters import Adapter, discover_adapters
from .common import repository_paths

TEXT_SUFFIXES = {".json", ".jsonl", ".md", ".toml", ".yaml", ".yml"}
SENSITIVE_NAME = re.compile(r"(?i)^(?:auth|credential|oauth|secret|token)s?(?:\..+)?$")
TRANSCRIPT_NAME = re.compile(r"(?i)(?:^|[-_.])(?:audit|conversation|session|transcript)s?(?:[-_.]|$)")
ASSIGNMENT = re.compile(
    r"(?i)(?:api[_-]?key|access[_-]?token|client[_-]?secret|refresh[_-]?token)"
    r"[\"']?\s*[:=]\s*[\"'][A-Za-z0-9_+/=.-]{16,}[\"']"
)
BEARER = re.compile(r"(?i)authorization[\"']?\s*[:=]\s*[\"']bearer\s+[A-Za-z0-9_+/=.-]{16,}")
KEY_PREFIX = re.compile(r"\bsk-[A-Za-z0-9_-]{16,}\b")
PRIVATE_KEY = re.compile(r"-{5}BEGIN (?:EC |OPENSSH |RSA )?PRIVATE KEY-{5}")
TRANSCRIPT_BODY = re.compile(r"[\"'](?:conversation|messages|turns)[\"']\s*:\s*\[")


def adapter_roots(adapters: list[Adapter]) -> set[str]:
    return {adapter.root.parts[0] for adapter in adapters}


def ai_owned(path: PurePosixPath, roots: set[str]) -> bool:
    return bool(path.parts) and (path.parts[0] == ".agents" or path.parts[0] in roots)


def suspicious_name(path: PurePosixPath) -> bool:
    if path.name == ".env.example":
        return False
    if path.name == ".env" or path.name.startswith(".env."):
        return True
    if SENSITIVE_NAME.fullmatch(path.name):
        return True
    return any(TRANSCRIPT_NAME.search(part) for part in path.parts)


def suspicious_text(path: Path) -> bool:
    if path.suffix.lower() not in TEXT_SUFFIXES:
        return False
    try:
        text = path.read_text(encoding="utf8")
    except (OSError, UnicodeDecodeError):
        return False
    return any(pattern.search(text) for pattern in (ASSIGNMENT, BEARER, KEY_PREFIX, PRIVATE_KEY, TRANSCRIPT_BODY))


def validate_secret_boundaries(root: Path, adapters: list[Adapter] | None = None) -> list[str]:
    if adapters is None:
        adapters, discovery_errors = discover_adapters(root)
        if discovery_errors:
            return discovery_errors
    roots = adapter_roots(adapters)
    errors: list[str] = []
    for relative in repository_paths(root):
        if not ai_owned(relative, roots):
            continue
        if suspicious_name(relative):
            errors.append(f"AI configuration contains credential or transcript path: {relative}")
        elif suspicious_text(root / relative):
            errors.append(f"AI configuration contains credential or transcript content: {relative}")
    return errors
