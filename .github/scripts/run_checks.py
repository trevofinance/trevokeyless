#!/usr/bin/env python3
"""Run named reusable repository check sets from one executable inventory."""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SET_ORDER = ("rust", "python", "scripts", "ts", "cross")


@dataclass(frozen=True)
class Check:
    name: str
    cwd: str
    argv: tuple[str, ...]


CHECKS: dict[str, tuple[Check, ...]] = {
    "rust": (
        Check("rustfmt", "keyless-rust", ("cargo", "fmt", "--all", "--", "--check")),
        Check(
            "clippy",
            "keyless-rust",
            ("cargo", "clippy", "--locked", "--all-features", "--all-targets", "--", "-D", "warnings"),
        ),
        Check("tests-all-features", "keyless-rust", ("cargo", "test", "--locked", "--all-features")),
        Check("tests-no-default-features", "keyless-rust", ("cargo", "test", "--locked", "--no-default-features")),
        Check("artifact-allowlist", ".", ("python3", ".github/scripts/check_rust_artifacts.py")),
    ),
    "python": (
        Check(
            "install-development-lock",
            "keyless-python",
            ("python3", "-m", "pip", "install", "--require-hashes", "-r", "requirements-dev.txt"),
        ),
        Check(
            "install-package",
            "keyless-python",
            ("python3", "-m", "pip", "install", "--no-deps", "--no-build-isolation", "."),
        ),
        Check("format", "keyless-python", ("ruff", "format", "--check")),
        Check("lint", "keyless-python", ("ruff", "check")),
        Check("types", "keyless-python", ("mypy", "trevokeyless", "tests")),
        Check("tests", "keyless-python", ("python3", "-m", "unittest", "discover", "--verbose")),
    ),
    "scripts": (
        Check("scripts-format", ".", ("ruff", "format", "--check", ".github/scripts")),
        Check("scripts-lint", ".", ("ruff", "check", ".github/scripts")),
        Check("scripts-types", ".", ("mypy", "--config-file", ".github/scripts/mypy.ini", ".github/scripts")),
        Check(
            "scripts-tests",
            ".",
            ("python3", "-m", "unittest", "discover", "-s", ".github/scripts", "-p", "test_*.py", "-v"),
        ),
    ),
    "ts": (
        Check("npm-install", "keyless-ts", ("npm", "ci")),
        Check("format", "keyless-ts", ("npm", "run", "format-check")),
        Check("lint", "keyless-ts", ("npm", "run", "lint-check")),
        Check("types", "keyless-ts", ("npm", "run", "type-check")),
        Check("build", "keyless-ts", ("npm", "run", "build")),
        Check("tests", "keyless-ts", ("npm", "test")),
        Check("platform-resolvers", "keyless-ts", ("npm", "run", "check-platform-resolvers")),
    ),
    "cross": (),
}

CROSS_SETUP = (
    Check("cross-npm-install", "keyless-ts", ("npm", "ci")),
    Check("cross-typescript-build", "keyless-ts", ("npm", "run", "build")),
)

SCRIPTS_SETUP = (
    Check(
        "scripts-install-development-lock",
        "keyless-python",
        ("python3", "-m", "pip", "install", "--require-hashes", "-r", "requirements-dev.txt"),
    ),
    Check(
        "scripts-install-package",
        "keyless-python",
        ("python3", "-m", "pip", "install", "--no-deps", "--no-build-isolation", "."),
    ),
)


def cross_check(seed: str, cases: int) -> Check:
    return Check(
        "differential",
        ".",
        ("python3", ".github/scripts/check_cross_language.py", "--seed", seed, "--cases", str(cases)),
    )


def normalize_sets(requested: list[str]) -> list[str]:
    values = requested or ["all"]
    unknown = set(values) - {*SET_ORDER, "all"}
    if unknown:
        raise ValueError("unknown check set: " + ", ".join(sorted(unknown)))
    return list(SET_ORDER) if "all" in values else [name for name in SET_ORDER if name in values]


def selected_checks(sets: list[str], *, cross_seed: str, cross_cases: int) -> list[tuple[str, Check]]:
    result: list[tuple[str, Check]] = []
    for set_name in sets:
        checks = CHECKS[set_name]
        if set_name == "scripts" and "python" not in sets:
            checks = (*SCRIPTS_SETUP, *checks)
        if set_name == "cross":
            if "ts" not in sets:
                checks = (*CROSS_SETUP,)
            checks = (*checks, cross_check(cross_seed, cross_cases))
        result.extend((set_name, check) for check in checks)
    return result


def display(check: Check) -> str:
    command = " ".join(shlex.quote(part) for part in check.argv)
    return f"(cd {shlex.quote(check.cwd)} && {command})"


def execute(check: Check) -> int:
    argv = (sys.executable, *check.argv[1:]) if check.argv[0] == "python3" else check.argv
    # The argv comes exclusively from the closed CHECKS inventory above.
    return subprocess.run(  # noqa: S603
        argv, cwd=ROOT / check.cwd, env=os.environ.copy(), check=False
    ).returncode


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sets", nargs="*", help="rust, python, scripts, ts, cross, or all")
    parser.add_argument("--list", action="store_true", help="print the inventory without executing it")
    parser.add_argument("--dry-run", action="store_true", help="print selected commands without executing them")
    parser.add_argument("--cross-seed", default="0x54524556")
    parser.add_argument("--cross-cases", type=int, default=256)
    return parser.parse_args()


def main() -> int:
    options = arguments()
    try:
        sets = normalize_sets(options.sets)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    if options.cross_cases < 1:
        print("--cross-cases must be positive", file=sys.stderr)
        return 2
    checks = selected_checks(sets, cross_seed=options.cross_seed, cross_cases=options.cross_cases)
    if options.list or options.dry_run:
        for set_name, check in checks:
            print(f"{set_name}:{check.name} {display(check)}")
        return 0
    failures = 0
    totals = {name: [0, 0] for name in sets}
    for set_name, check in checks:
        print(f"RUN {set_name}:{check.name} {display(check)}", flush=True)
        returncode = execute(check)
        totals[set_name][1] += 1
        if returncode == 0:
            totals[set_name][0] += 1
            print(f"PASS {set_name}:{check.name}")
        else:
            failures += 1
            print(f"FAIL {set_name}:{check.name} (exit {returncode})")
    for name in sets:
        passed, total = totals[name]
        print(f"{name}: {passed}/{total} passed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
