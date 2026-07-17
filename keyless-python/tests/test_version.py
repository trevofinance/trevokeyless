"""Pins ``trevokeyless.__version__``.

Two guarantees:

1. The hardcoded ``__version__`` matches the ``version`` in ``pyproject.toml`` (they
   would otherwise drift now that the string is not resolved from package metadata).
2. ``import trevokeyless`` succeeds even when no installed package metadata is on the
   path — the failure mode that an import-time ``importlib.metadata.version`` lookup
   would (re)introduce. The check runs in a ``-S`` subprocess (no site packages) against
   a copy of the package source that carries no ``.dist-info``.
"""

import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import trevokeyless

# The package dir follows the import (source tree or site-packages) — it is copied into
# the no-site-packages subprocess below. pyproject.toml, in contrast, is resolved from
# this test file, so the suite also passes when run against an *installed* package,
# whose parent directory is site-packages and holds no manifest.
_PACKAGE_DIR = Path(trevokeyless.__file__).resolve().parent
_PYPROJECT = Path(__file__).resolve().parents[1] / "pyproject.toml"


def _pyproject_version() -> str:
    # A regex rather than tomllib: the support matrix includes Python 3.10, which has
    # no tomllib, and only the [project] version line is needed.
    text = _PYPROJECT.read_text(encoding="utf-8")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', text)
    if match is None:
        msg = "version not found in pyproject.toml"
        raise RuntimeError(msg)
    return match.group(1)


class TestVersion(unittest.TestCase):
    def test_version_matches_pyproject(self: "TestVersion") -> None:
        self.assertEqual(trevokeyless.__version__, _pyproject_version())

    def test_import_succeeds_without_installed_metadata(self: "TestVersion") -> None:
        # Copy only the package source (no .dist-info) to a throwaway tree and import it
        # from a -S subprocess, so neither this package's installed metadata nor any
        # other site package is reachable. A reintroduced import-time
        # importlib.metadata.version("trevokeyless") would raise PackageNotFoundError
        # here and fail the test.
        with tempfile.TemporaryDirectory() as tmp:
            shutil.copytree(_PACKAGE_DIR, Path(tmp) / "trevokeyless")
            result = subprocess.run(
                [sys.executable, "-S", "-c", "import trevokeyless; print(trevokeyless.__version__)"],
                cwd=tmp,
                env={"PYTHONPATH": tmp},
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(result.returncode, 0, msg=f"import failed:\n{result.stderr}")
        self.assertEqual(result.stdout.strip(), _pyproject_version())


if __name__ == "__main__":
    unittest.main()
