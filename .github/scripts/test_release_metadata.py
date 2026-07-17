"""Regression tests for release metadata validation that do not mutate manifests."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import shutil
import tempfile
import unittest
from datetime import date
from pathlib import Path
from types import ModuleType
from unittest import mock


def load_release_metadata() -> ModuleType:
    path = Path(__file__).with_name("release_metadata.py")
    spec = importlib.util.spec_from_file_location("release_metadata", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load release_metadata.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RELEASE_METADATA = load_release_metadata()

# Every file the Rust half of `--set` rewrites. Each one independently pins the crate
# version and is published or audited, so all four must move together.
RUST_SYNC_POINTS = (
    "keyless-rust/Cargo.toml",
    "keyless-rust/Cargo.lock",
    "keyless-rust/fuzz/Cargo.lock",
    "keyless-rust/README.md",
)


class TestStableVersion(unittest.TestCase):
    def test_accepts_stable_versions(self) -> None:
        for version in ("0.0.0", "0.1.0", "1.2.3", "10.20.30"):
            with self.subTest(version=version):
                self.assertTrue(RELEASE_METADATA.is_stable_version(version))

    def test_rejects_non_release_and_malformed_versions(self) -> None:
        for version in (
            "",
            "1",
            "1.2",
            "01.2.3",
            "1.02.3",
            "1.2.03",
            "1.2.3-alpha",
            "1.2.3-alpha..1",
            "1.2.3+build.1",
            "v1.2.3",
            "1.2.3.4",
            "1.٢.3",
            " 1.2.3",
        ):
            with self.subTest(version=version):
                self.assertFalse(RELEASE_METADATA.is_stable_version(version))


class TestSemverKey(unittest.TestCase):
    def test_orders_numerically_not_lexically(self) -> None:
        self.assertEqual(RELEASE_METADATA.semver_key("2.0.0"), (2, 0, 0))
        self.assertLess(RELEASE_METADATA.semver_key("1.9.0"), RELEASE_METADATA.semver_key("1.10.0"))
        self.assertLess(RELEASE_METADATA.semver_key("1.0.0"), RELEASE_METADATA.semver_key("2.0.0"))


class TestChangelogPendingRelease(unittest.TestCase):
    """A changelog may lag the manifests only while an [Unreleased] section exists."""

    def test_unreleased_section_detection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            changelog = root / "CHANGELOG.md"
            changelog.write_text(
                "# Changelog\n\n## [Unreleased]\n\n- pending\n\n## [1.0.0] — 2026-07-05\n",
                encoding="utf8",
            )
            with mock.patch.object(RELEASE_METADATA, "ROOT", root):
                self.assertTrue(RELEASE_METADATA.changelog_has_unreleased("CHANGELOG.md"))
                # The [Unreleased] heading carries no date, so the newest *released*
                # heading is still the one the checker reads.
                self.assertEqual(
                    RELEASE_METADATA.first_changelog_release("CHANGELOG.md"),
                    ("1.0.0", date(2026, 7, 5)),
                )
            changelog.write_text("# Changelog\n\n## [1.0.0] — 2026-07-05\n", encoding="utf8")
            with mock.patch.object(RELEASE_METADATA, "ROOT", root):
                self.assertFalse(RELEASE_METADATA.changelog_has_unreleased("CHANGELOG.md"))

    def test_committed_changelogs_agree_or_lag_under_unreleased(self) -> None:
        manifests = {
            "keyless-rust/CHANGELOG.md": RELEASE_METADATA.version_assignment("keyless-rust/Cargo.toml"),
            "keyless-python/CHANGELOG.md": RELEASE_METADATA.version_assignment("keyless-python/pyproject.toml"),
            "keyless-ts/CHANGELOG.md": json.loads(RELEASE_METADATA.read("keyless-ts/package.json"))["version"],
        }
        for path, manifest in manifests.items():
            with self.subTest(path=path):
                released, _release_date = RELEASE_METADATA.first_changelog_release(path)
                if released == manifest:
                    continue
                self.assertTrue(
                    RELEASE_METADATA.changelog_has_unreleased(path),
                    f"{path} lags the manifest ({released} < {manifest}) without an [Unreleased] section",
                )
                self.assertLess(RELEASE_METADATA.semver_key(released), RELEASE_METADATA.semver_key(manifest))


class TestNodeMatrixFloor(unittest.TestCase):
    """The check-ts.yml matrix must test the exact engines floor: nothing else couples
    the two files, so a floor raised in only one of them must fail here."""

    def test_committed_matrix_tests_the_engines_floor(self) -> None:
        node_floor = RELEASE_METADATA.NODE_RANGE.removeprefix(">=")
        matrix_versions = RELEASE_METADATA.check_ts_matrix_node_versions()
        self.assertIn(node_floor, matrix_versions)

    def test_matrix_extraction_is_rigid(self) -> None:
        # A reshaped workflow (renamed key, multi-line matrix) must fail loudly rather
        # than leave the coupling silently unchecked.
        with (
            mock.patch.object(RELEASE_METADATA, "read", return_value="jobs: {}\n"),
            self.assertRaises(AssertionError),
        ):
            RELEASE_METADATA.check_ts_matrix_node_versions()
        with mock.patch.object(
            RELEASE_METADATA,
            "read",
            return_value='        node_version: ["22.13.0", "24"]\n',
        ):
            self.assertEqual(
                RELEASE_METADATA.check_ts_matrix_node_versions(),
                ["22.13.0", "24"],
            )


class TestRustSyncPoints(unittest.TestCase):
    """The committed tree must already agree, and `--set` must move every declaration."""

    def test_committed_rust_declarations_agree(self) -> None:
        expected = RELEASE_METADATA.version_assignment("keyless-rust/Cargo.toml")
        self.assertEqual(RELEASE_METADATA.cargo_lock_version("keyless-rust/Cargo.lock"), expected)
        self.assertEqual(RELEASE_METADATA.cargo_lock_version("keyless-rust/fuzz/Cargo.lock"), expected)
        self.assertEqual(RELEASE_METADATA.rust_readme_version(), expected)

    def test_set_rust_updates_every_sync_point(self) -> None:
        # Copy the four Rust sync points into a throwaway tree and point the helper at it,
        # so `--set` is exercised for real without mutating the repository.
        real_root = Path(RELEASE_METADATA.ROOT)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in RUST_SYNC_POINTS:
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(real_root / relative, destination)
            with mock.patch.object(RELEASE_METADATA, "ROOT", root), contextlib.redirect_stdout(io.StringIO()):
                RELEASE_METADATA.set_version("rust", "9.10.11")
                self.assertEqual(RELEASE_METADATA.version_assignment("keyless-rust/Cargo.toml"), "9.10.11")
                self.assertEqual(RELEASE_METADATA.cargo_lock_version("keyless-rust/Cargo.lock"), "9.10.11")
                self.assertEqual(RELEASE_METADATA.cargo_lock_version("keyless-rust/fuzz/Cargo.lock"), "9.10.11")
                self.assertEqual(RELEASE_METADATA.rust_readme_version(), "9.10.11")
        # The real tree is untouched.
        self.assertNotEqual(RELEASE_METADATA.version_assignment("keyless-rust/Cargo.toml"), "9.10.11")

    def test_set_rejects_non_release_versions(self) -> None:
        with self.assertRaises(SystemExit), contextlib.redirect_stdout(io.StringIO()):
            RELEASE_METADATA.set_version("rust", "9.10.11-rc.1")


class TestReplaceOnce(unittest.TestCase):
    """`replace_once` must refuse both zero matches and — the case `count=1` used to cap
    away — more than one, instead of silently rewriting whichever match comes first."""

    def _replace(self, content: str) -> str:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(content, encoding="utf8")
            with mock.patch.object(RELEASE_METADATA, "ROOT", root):
                RELEASE_METADATA.replace_once("Cargo.toml", r'^version = "[^"]+"$', 'version = "9.9.9"')
            return (root / "Cargo.toml").read_text(encoding="utf8")

    def test_replaces_a_unique_match(self) -> None:
        self.assertEqual(
            self._replace('name = "x"\nversion = "1.0.0"\n'),
            'name = "x"\nversion = "9.9.9"\n',
        )

    def test_rejects_zero_matches(self) -> None:
        with self.assertRaisesRegex(AssertionError, "found 0"):
            self._replace('name = "x"\n')

    def test_rejects_an_ambiguous_pattern_and_leaves_the_file_untouched(self) -> None:
        # A second `version = "..."` line (e.g. a tool table) must abort the rewrite.
        content = 'version = "1.0.0"\n[tool.something]\nversion = "0.4.0"\n'
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(content, encoding="utf8")
            with (
                mock.patch.object(RELEASE_METADATA, "ROOT", root),
                self.assertRaisesRegex(AssertionError, "found 2"),
            ):
                RELEASE_METADATA.replace_once("Cargo.toml", r'^version = "[^"]+"$', 'version = "9.9.9"')
            self.assertEqual((root / "Cargo.toml").read_text(encoding="utf8"), content)
