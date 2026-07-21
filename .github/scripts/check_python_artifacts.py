"""Assert that the Python wheel and sdist have exactly the release allowlists.

Run from keyless-python/ after ``python -m build``. The same checker is used by
pull-request and publish workflows so unexpected generated, test, or local files
cannot silently enter a release artifact.
"""

from __future__ import annotations

import sys
import tarfile
import zipfile
from pathlib import Path

if sys.version_info >= (3, 11):
    import tomllib
else:
    # This script runs on every interpreter in the check-python matrix, and tomllib is
    # only in the standard library from 3.11. tomli is its backport (and the same code):
    # it is in requirements-dev.txt on every platform, via pip-audit.
    import tomli as tomllib


def only(pattern: str) -> Path:
    matches = list(Path().glob(pattern))
    if len(matches) != 1:
        message = f"expected exactly one {pattern}, found {matches}"
        raise AssertionError(message)
    return matches[0]


def assert_exact(kind: str, actual: set[str], expected: set[str]) -> None:
    missing = sorted(expected - actual)
    unexpected = sorted(actual - expected)
    if missing or unexpected:
        message = f"{kind} contents differ from the exact allowlist\nmissing: {missing}\nunexpected: {unexpected}"
        raise AssertionError(message)


version = tomllib.loads(Path("pyproject.toml").read_text())["project"]["version"]
dist_info = f"trevokeyless-{version}.dist-info"
wheel_expected = {
    "trevokeyless/__init__.py",
    "trevokeyless/_ss58.py",
    "trevokeyless/keyless.py",
    "trevokeyless/py.typed",
    f"{dist_info}/licenses/LICENSE",
    f"{dist_info}/METADATA",
    f"{dist_info}/WHEEL",
    f"{dist_info}/top_level.txt",
    f"{dist_info}/RECORD",
}

wheel_path = only(f"dist/trevokeyless-{version}-*.whl")
with zipfile.ZipFile(wheel_path) as wheel:
    wheel_actual = set(wheel.namelist())
assert_exact("wheel", wheel_actual, wheel_expected)

root = f"trevokeyless-{version}"
sdist_expected = {
    root,
    f"{root}/CHANGELOG.md",
    f"{root}/LICENSE",
    f"{root}/MANIFEST.in",
    f"{root}/PKG-INFO",
    f"{root}/README.md",
    f"{root}/pyproject.toml",
    f"{root}/setup.cfg",
    f"{root}/trevokeyless",
    f"{root}/trevokeyless/__init__.py",
    f"{root}/trevokeyless/_ss58.py",
    f"{root}/trevokeyless/keyless.py",
    f"{root}/trevokeyless/py.typed",
    f"{root}/trevokeyless.egg-info",
    f"{root}/trevokeyless.egg-info/PKG-INFO",
    f"{root}/trevokeyless.egg-info/SOURCES.txt",
    f"{root}/trevokeyless.egg-info/dependency_links.txt",
    # No egg-info/requires.txt: the package has zero runtime dependencies and no extras,
    # so setuptools emits none. Its reappearance would mean a dependency crept in.
    f"{root}/trevokeyless.egg-info/top_level.txt",
}

sdist_path = only(f"dist/trevokeyless-{version}.tar.gz")
with tarfile.open(sdist_path) as sdist:
    sdist_actual = set(sdist.getnames())
assert_exact("sdist", sdist_actual, sdist_expected)

# The publish workflow uploads and publishes *all* of dist/, so the directory itself must
# hold exactly the two artifacts checked above: a stray file — say a tarball of another
# version left by an earlier build — would match neither versioned glob above, yet still
# be published. The Rust and TypeScript checkers assert their exact file sets already.
dist_actual = {path.name for path in Path("dist").iterdir()}
assert_exact("dist/", dist_actual, {wheel_path.name, sdist_path.name})

print(f"wheel: {len(wheel_actual)} entries; sdist: {len(sdist_actual)} entries - exact contents OK")
