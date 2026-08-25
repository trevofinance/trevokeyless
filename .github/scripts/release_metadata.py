"""Synchronize and verify duplicated package release metadata.

Without arguments this is a read-only CI check. ``--set`` updates every version
declaration for one package; changelogs remain intentional human-authored release
notes and are then enforced by the check.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from datetime import UTC, date, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# Repository publishing uses stable releases only. Pre-release and build metadata are
# deliberately rejected instead of partially reimplementing the full SemVer grammar.
STABLE_VERSION = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
NODE_RANGE = ">=22.13.0"
REPOSITORY = "https://github.com/trevo-finance/trevokeyless"
# The install snippet in keyless-rust/README.md; the README ships inside the .crate, so a
# stale version there is published to crates.io and docs.rs.
RUST_README_DEPENDENCY = r'^(trevokeyless = \{ version = ")([^"]+)(")'
CARGO_LOCK_PACKAGE = r'(\[\[package\]\]\nname = "trevokeyless"\nversion = ")[^"]+("\n)'


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf8")


def is_stable_version(value: str) -> bool:
    """Return whether ``value`` is the release-only ``X.Y.Z`` form."""
    return STABLE_VERSION.fullmatch(value) is not None


def version_assignment(path: str) -> str:
    match = re.search(r'^version = "([^"]+)"$', read(path), re.MULTILINE)
    if not match:
        raise AssertionError(f"no version assignment in {path}")
    return match.group(1)


def python_init_version() -> str:
    match = re.search(r'^__version__ = "([^"]+)"$', read("keyless-python/trevokeyless/__init__.py"), re.MULTILINE)
    if not match:
        raise AssertionError("no __version__ assignment in Python package")
    return match.group(1)


def cargo_lock_version(path: str) -> str:
    """Return the `trevokeyless` version pinned by a Cargo lockfile.

    Both keyless-rust/Cargo.lock and keyless-rust/fuzz/Cargo.lock pin it (the fuzz crate
    depends on the library by path), and `cargo audit` reads both.
    """
    match = re.search(
        r'\[\[package\]\]\nname = "trevokeyless"\nversion = "([^"]+)"',
        read(path),
    )
    if not match:
        raise AssertionError(f"trevokeyless package missing from {path}")
    return match.group(1)


def rust_readme_version() -> str:
    """Return the crate version in the README install snippet (not a manifest, but published)."""
    match = re.search(RUST_README_DEPENDENCY, read("keyless-rust/README.md"), re.MULTILINE)
    if not match:
        raise AssertionError("no trevokeyless dependency snippet in keyless-rust/README.md")
    return match.group(2)


def first_changelog_release(path: str) -> tuple[str, date]:
    match = re.search(r"^## \[([^]]+)] — (\d{4}-\d{2}-\d{2})$", read(path), re.MULTILINE)
    if not match:
        raise AssertionError(f"no release heading in {path}")
    release_date = date.fromisoformat(match.group(2))
    if release_date > datetime.now(tz=UTC).date():
        raise AssertionError(f"future release date {release_date} in {path}")
    return match.group(1), release_date


def changelog_has_unreleased(path: str) -> bool:
    """Whether the changelog carries an ``## [Unreleased]`` section.

    Between releases the notes for the next version accumulate under that heading while
    the manifests may already carry the bumped version; the release PR replaces the
    heading with ``## [X.Y.Z] — date`` (see docs/RELEASING.md).
    """
    return re.search(r"^## \[Unreleased\]$", read(path), re.MULTILINE) is not None


def semver_key(version: str) -> tuple[int, int, int]:
    """The stable ``X.Y.Z`` version as an orderable tuple. Callers validate the shape first."""
    major, minor, patch = version.split(".")
    return (int(major), int(minor), int(patch))


def assert_repository_metadata(package_json: dict[str, object]) -> None:
    rust_repository = re.search(r'^repository = "([^"]+)"$', read("keyless-rust/Cargo.toml"), re.MULTILINE)
    python_repository = re.search(r'^Repository = "([^"]+)"$', read("keyless-python/pyproject.toml"), re.MULTILINE)
    expected = {
        "rust": f"{REPOSITORY}/",
        "python": f"{REPOSITORY}.git",
        "ts": f"git+{REPOSITORY}.git",
    }
    actual = {
        "rust": rust_repository.group(1) if rust_repository else None,
        "python": python_repository.group(1) if python_repository else None,
        "ts": package_json["repository"]["url"],  # type: ignore[index]
    }
    if actual != expected:
        raise AssertionError(f"repository metadata drifted: expected {expected}, got {actual}")
    if package_json["bugs"]["url"] != f"{REPOSITORY}/issues":  # type: ignore[index]
        raise AssertionError("npm issue URL drifted")


def assert_conformance_fixture_fresh() -> None:
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--locked",
            "--quiet",
            "--manifest-path",
            "keyless-rust/Cargo.toml",
            "--example",
            "gen_conformance",
            "--features",
            "ss58",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    committed = read("conformance/vectors.json")
    if completed.stdout != committed:
        raise AssertionError("conformance fixture is stale; regenerate it with the Rust example")


def assert_changelog_release(package: str, path: str, released: str, expected: str) -> None:
    """Check the newest released changelog heading against the manifest version.

    Changelogs are human-authored release notes, not another mechanical sync point:
    between releases the next version's notes accumulate under ``## [Unreleased]`` while
    the manifests already carry the bumped version, so the newest *released* heading may
    lag the manifests exactly while that section exists. The tag check in ``verify``
    still refuses to publish until the heading is written.
    """
    if not is_stable_version(released):
        raise AssertionError(f"{package} changelog release heading {released!r} is not a stable X.Y.Z version")
    if released == expected:
        return
    if changelog_has_unreleased(path) and semver_key(released) < semver_key(expected):
        return
    raise AssertionError(
        f"{package} changelog release {released} disagrees with version {expected} "
        "and there is no [Unreleased] section to account for the gap"
    )


def replace_once(path: str, pattern: str, replacement: str) -> None:
    target = ROOT / path
    # Uncapped on purpose: with count=1 re.subn reports at most 1, so an ambiguous
    # pattern (say, a second `^version = "..."` line landing in a manifest via a tool
    # table) would silently rewrite whichever match comes first. Count every match and
    # refuse to write unless the pattern is unique; only then is the rewrite the one
    # intended.
    updated, count = re.subn(pattern, replacement, target.read_text(encoding="utf8"), flags=re.MULTILINE)
    if count != 1:
        raise AssertionError(f"expected one version declaration in {path}, found {count}")
    target.write_text(updated, encoding="utf8")


def set_version(package: str, version: str) -> None:
    if not is_stable_version(version):
        raise SystemExit(f"invalid stable release version (expected X.Y.Z): {version}")
    if package == "rust":
        replace_once("keyless-rust/Cargo.toml", r'^version = "[^"]+"$', f'version = "{version}"')
        replace_once("keyless-rust/Cargo.lock", CARGO_LOCK_PACKAGE, rf"\g<1>{version}\g<2>")
        # The fuzz crate is a separate workspace with its own lockfile that also pins the
        # library (path dependency), and `cargo audit --file fuzz/Cargo.lock` reads it.
        replace_once("keyless-rust/fuzz/Cargo.lock", CARGO_LOCK_PACKAGE, rf"\g<1>{version}\g<2>")
        replace_once("keyless-rust/README.md", RUST_README_DEPENDENCY, rf"\g<1>{version}\g<3>")
    elif package == "python":
        replace_once("keyless-python/pyproject.toml", r'^version = "[^"]+"$', f'version = "{version}"')
        replace_once(
            "keyless-python/trevokeyless/__init__.py",
            r'^__version__ = "[^"]+"$',
            f'__version__ = "{version}"',
        )
    else:
        subprocess.run(  # noqa: S603
            ["npm", "version", version, "--no-git-tag-version", "--allow-same-version"],
            cwd=ROOT / "keyless-ts",
            check=True,
        )
    print(
        f"set {package} declarations to {version}; add the {version} changelog heading, "
        "then run this script without --set"
    )


def check_ts_matrix_node_versions() -> list[str]:
    """The Node versions of the check-ts.yml test matrix.

    Extracted with a line-anchored regex rather than a YAML parser (the scripts are
    stdlib-only); the pattern is deliberately rigid, so a reshaped workflow fails here
    loudly instead of leaving the matrix silently unchecked.
    """
    match = re.search(
        r"^\s*node_version:\s*\[([^\]]+)\]\s*$",
        read(".github/workflows/check-ts.yml"),
        re.MULTILINE,
    )
    if not match:
        raise AssertionError("no node_version matrix in check-ts.yml")
    return [item.strip().strip('"') for item in match.group(1).split(",")]


def assert_node_floor_tested() -> None:
    """The engines floor is "the supported and exactly tested floor" (docs/versioning.md):
    the check-ts.yml matrix must test that exact version. Nothing else
    couples the two files, so raising one without the other would silently either
    test an unsupported version or ship an untested floor."""
    node_floor = NODE_RANGE.removeprefix(">=")
    matrix_versions = check_ts_matrix_node_versions()
    if node_floor not in matrix_versions:
        raise AssertionError(
            f"check-ts.yml node_version matrix {matrix_versions} does not test the engines floor {node_floor}"
        )


def verify() -> None:
    package_json = json.loads(read("keyless-ts/package.json"))
    package_lock = json.loads(read("keyless-ts/package-lock.json"))
    versions = {
        "rust": version_assignment("keyless-rust/Cargo.toml"),
        "python": version_assignment("keyless-python/pyproject.toml"),
        "ts": package_json["version"],
    }
    changelog_paths = {
        "rust": "keyless-rust/CHANGELOG.md",
        "python": "keyless-python/CHANGELOG.md",
        "ts": "keyless-ts/CHANGELOG.md",
    }
    changelogs = {package: first_changelog_release(path) for package, path in changelog_paths.items()}
    declarations = {
        "rust": [
            cargo_lock_version("keyless-rust/Cargo.lock"),
            cargo_lock_version("keyless-rust/fuzz/Cargo.lock"),
            rust_readme_version(),
        ],
        "python": [python_init_version()],
        "ts": [
            package_lock["version"],
            package_lock["packages"][""]["version"],
        ],
    }
    for package, expected in versions.items():
        if not is_stable_version(expected):
            raise AssertionError(f"{package} has invalid stable release version {expected}")
        if any(actual != expected for actual in declarations[package]):
            raise AssertionError(f"{package} version {expected} disagrees with {declarations[package]}")

        assert_changelog_release(package, changelog_paths[package], changelogs[package][0], expected)

    if package_json["engines"]["node"] != NODE_RANGE:
        raise AssertionError("package.json Node engine range drifted")
    if package_lock["packages"][""]["engines"]["node"] != NODE_RANGE:
        raise AssertionError("package-lock.json Node engine range drifted")
    assert_node_floor_tested()
    assert_repository_metadata(package_json)
    assert_conformance_fixture_fresh()

    tag = os.environ.get("GITHUB_REF_NAME", "")
    for package, prefix in (("rust", "rust-v"), ("python", "python-v"), ("ts", "ts-v")):
        if not tag.startswith(prefix):
            continue
        if tag.removeprefix(prefix) != versions[package]:
            raise AssertionError(f"tag {tag} disagrees with {package} version {versions[package]}")
        if changelogs[package][0] != versions[package]:
            raise AssertionError(
                f"tag {tag} requires a {versions[package]} release heading in the {package} "
                "changelog; move the [Unreleased] notes under it before tagging"
            )
    print("release metadata, repositories, conformance fixture, tag, and Node engines agree")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--set", nargs=2, metavar=("PACKAGE", "VERSION"))
    args = parser.parse_args()
    if args.set:
        package, version = args.set
        if package not in {"rust", "python", "ts"}:
            parser.error("PACKAGE must be rust, python, or ts")
        set_version(package, version)
        return
    verify()


if __name__ == "__main__":
    main()
