"""Assert that the packaged crate contains exactly the release allowlist.

`cargo publish --dry-run` says nothing about the *contents* of the .crate file. This is the
Rust counterpart of check_python_artifacts.py and keyless-ts/scripts/check-package.mjs: it
compares the file list selected by Cargo.toml's product allowlist with the exact allowlist
below.

Run from anywhere in the repository:

    python3 .github/scripts/check_rust_artifacts.py

When a file is legitimately added to or removed from the crate, update EXPECTED in the
same pull request.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / "keyless-rust"

# Everything `cargo package` puts in the .crate file. Cargo synthesizes the first three
# entries; the rest are tracked product sources. Repository configuration, the fuzz/ crate,
# the differential driver, and anything under target/ must stay absent.
EXPECTED = {
    ".cargo_vcs_info.json",
    "CHANGELOG.md",
    "Cargo.lock",
    "Cargo.toml",
    "Cargo.toml.orig",
    "LICENSE",
    "README.md",
    "examples/gen_conformance.rs",
    "src/keyless.rs",
    "src/lib.rs",
    "src/ss58.rs",
    "src/test_accounts.rs",
    "src/tests/mod.rs",
    "src/tests/test_accounts.rs",
    "src/tests/tests_binary.rs",
    "src/tests/tests_conformance.rs",
    "src/tests/tests_encoding.rs",
    "src/tests/tests_properties.rs",
    "src/tests/tests_serde.rs",
    "src/tests/tests_ss58.rs",
    "src/tests/tests_version.rs",
    "src/tests/types.rs",
    "src/types.rs",
}


def packaged_files() -> set[str]:
    completed = subprocess.run(
        ["cargo", "package", "--locked", "--list", "--allow-dirty"],
        cwd=CRATE,
        check=True,
        capture_output=True,
        text=True,
    )
    return {line.strip() for line in completed.stdout.splitlines() if line.strip()}


def main() -> None:
    actual = packaged_files()
    missing = sorted(EXPECTED - actual)
    unexpected = sorted(actual - EXPECTED)
    if missing or unexpected:
        print("::error::packaged crate contents differ from the exact allowlist")
        print(f"missing:    {missing}")
        print(f"unexpected: {unexpected}")
        print("Update EXPECTED in .github/scripts/check_rust_artifacts.py if intended.")
        sys.exit(1)
    print(f"crate: {len(actual)} entries - exact contents OK")


if __name__ == "__main__":
    main()
