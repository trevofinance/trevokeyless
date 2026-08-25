"""Tests for the executable repository check inventory."""

from __future__ import annotations

import unittest

import run_checks


class RunChecksTests(unittest.TestCase):
    def test_all_uses_canonical_order_and_cross_after_typescript(self) -> None:
        sets = run_checks.normalize_sets(["all"])
        self.assertEqual(sets, list(run_checks.SET_ORDER))
        checks = run_checks.selected_checks(sets, cross_seed="7", cross_cases=8)
        names = [f"{set_name}:{check.name}" for set_name, check in checks]
        self.assertLess(names.index("ts:build"), names.index("cross:differential"))
        self.assertNotIn("cross:cross-typescript-build", names)

    def test_standalone_cross_owns_its_prerequisite(self) -> None:
        checks = run_checks.selected_checks(["cross"], cross_seed="9", cross_cases=10)
        self.assertEqual(
            [check.name for _, check in checks], ["cross-npm-install", "cross-typescript-build", "differential"]
        )
        self.assertEqual(checks[-1][1].argv[-4:], ("--seed", "9", "--cases", "10"))

    def test_standalone_scripts_owns_python_setup(self) -> None:
        checks = run_checks.selected_checks(["scripts"], cross_seed="0", cross_cases=1)
        self.assertEqual(
            [check.name for _, check in checks[:2]],
            ["scripts-install-development-lock", "scripts-install-package"],
        )

    def test_unknown_set_fails(self) -> None:
        with self.assertRaisesRegex(ValueError, "unknown check set"):
            run_checks.normalize_sets(["unknown"])


if __name__ == "__main__":
    unittest.main()
