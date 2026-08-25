#!/usr/bin/env python3
"""Run or replay a checked-in Rust fuzz target using repository-owned pins."""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST_ROOT = ROOT / "keyless-rust"
TARGETS = ("ss58_decode", "accountid_from_str", "classify")
NIGHTLY = "nightly-2026-07-01"
CARGO_FUZZ_VERSION = "0.13.2"
TARGET_TRIPLE = "x86_64-unknown-linux-gnu"


def tool_setup_commands() -> list[tuple[str, ...]]:
    return [
        ("rustup", "toolchain", "install", NIGHTLY),
        ("cargo", "install", "cargo-fuzz", "--version", CARGO_FUZZ_VERSION, "--locked"),
    ]


def fuzz_command(target: str, duration: int, reproducer: Path | None) -> tuple[str, ...]:
    prefix = ("cargo", f"+{NIGHTLY}", "fuzz", "run", target)
    inputs = (os.fspath(reproducer),) if reproducer else (f"fuzz/corpus/{target}", f"fuzz/seeds/{target}")
    return (*prefix, *inputs, "--target", TARGET_TRIPLE, "--", f"-max_total_time={duration}")


def validate(target: str, duration: int, reproducer: Path | None) -> list[str]:
    errors: list[str] = []
    if target not in TARGETS:
        errors.append(f"unknown fuzz target: {target}")
    elif not (RUST_ROOT / "fuzz" / "fuzz_targets" / f"{target}.rs").is_file():
        errors.append(f"fuzz target source is missing: {target}")
    if duration < 1:
        errors.append("duration must be positive")
    if reproducer and not reproducer.is_file():
        errors.append(f"reproducer does not exist: {reproducer}")
    return errors


def run(command: tuple[str, ...], *, dry_run: bool) -> int:
    print(" ".join(shlex.quote(part) for part in command))
    if dry_run:
        return 0
    # Command parts come from argparse choices and repository-owned constants.
    return subprocess.run(command, cwd=RUST_ROOT, check=False).returncode  # noqa: S603


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, default="ss58_decode")
    parser.add_argument("--duration", type=int, default=300)
    parser.add_argument("--reproducer", type=Path)
    parser.add_argument("--ensure-tools", action="store_true")
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--list", action="store_true")
    return parser.parse_args()


def main() -> int:
    options = arguments()
    if options.list:
        print(f"targets={','.join(TARGETS)}")
        print(f"nightly={NIGHTLY}")
        print(f"cargo-fuzz={CARGO_FUZZ_VERSION}")
        print(f"target-triple={TARGET_TRIPLE}")
        return 0
    errors = validate(options.target, options.duration, options.reproducer)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 2
    if options.ensure_tools:
        for command in tool_setup_commands():
            if run(command, dry_run=options.dry_run):
                return 1
    lock_check = ("cargo", "metadata", "--locked", "--manifest-path", "fuzz/Cargo.toml")
    if run(lock_check, dry_run=options.dry_run):
        return 1
    if options.check_only:
        return 0
    if not options.reproducer and not options.dry_run:
        (RUST_ROOT / "fuzz" / "corpus" / options.target).mkdir(parents=True, exist_ok=True)
    return run(fuzz_command(options.target, options.duration, options.reproducer), dry_run=options.dry_run)


if __name__ == "__main__":
    raise SystemExit(main())
