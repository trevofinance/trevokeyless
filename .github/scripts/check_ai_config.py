#!/usr/bin/env python3
"""Validate provider-neutral core and installed provider adapter contracts."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from ai_config.adapters import Adapter, validate_adapters
from ai_config.boundaries import validate_boundaries
from ai_config.core import validate_core
from ai_config.independence import validate_core_independence
from ai_config.secrets import validate_secret_boundaries

ROOT = Path(__file__).resolve().parents[2]


def validate(scope: str, root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    adapters: list[Adapter] = []
    if scope in {"core", "all"}:
        errors.extend(validate_core(root))
    if scope in {"adapters", "all"}:
        adapters, adapter_errors = validate_adapters(root)
        errors.extend(adapter_errors)
        errors.extend(validate_boundaries(root, adapters))
    if scope == "all":
        errors.extend(validate_secret_boundaries(root, adapters))
        errors.extend(validate_core_independence(root))
    return errors


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scope", choices=("core", "adapters", "all"), default="core")
    return parser.parse_args()


def main() -> int:
    options = arguments()
    errors = validate(options.scope)
    if errors:
        for error in errors:
            print(f"::error::{error}")
        return 1
    print(f"AI configuration {options.scope}: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
