"""Tests for deterministic context routing."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / ".agents/skills/context-resolve/scripts/resolve_context.py"
SPEC = importlib.util.spec_from_file_location("context_resolver", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load context resolver from {SCRIPT}")
resolver = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(resolver)


class ContextResolverTests(unittest.TestCase):
    def test_fixtures(self) -> None:
        self.assertEqual(resolver.check_fixtures(resolver.load_toml(resolver.MAP_PATH)), 0)

    def test_unknown_path_fails_closed(self) -> None:
        result = resolver.resolve(
            resolver.load_toml(resolver.MAP_PATH), mode="plan", paths=["unknown/place.txt"], task="", concerns=[]
        )
        self.assertIn("UNKNOWN PATH: unknown/place.txt", result["errors"])

    def test_resolution_is_deterministic(self) -> None:
        arguments = dict(
            mode="review",
            paths=["keyless-rust/src/lib.rs", "conformance/vectors.json"],
            task="encoding parity",
            concerns=["conformance"],
        )
        first = resolver.resolve(resolver.load_toml(resolver.MAP_PATH), **arguments)
        second = resolver.resolve(resolver.load_toml(resolver.MAP_PATH), **arguments)
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
