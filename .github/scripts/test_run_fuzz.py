"""Tests for the repository-owned fuzz command builder."""

from __future__ import annotations

import unittest
from pathlib import Path

import run_fuzz


class RunFuzzTests(unittest.TestCase):
    def test_normal_run_uses_all_repository_pins(self) -> None:
        command = run_fuzz.fuzz_command("classify", 30, None)
        self.assertIn(f"+{run_fuzz.NIGHTLY}", command)
        self.assertIn(run_fuzz.TARGET_TRIPLE, command)
        self.assertIn("fuzz/seeds/classify", command)
        self.assertEqual(command[-1], "-max_total_time=30")

    def test_reproducer_replaces_corpus_arguments(self) -> None:
        artifact = Path("fuzz/artifacts/classify/crash")
        command = run_fuzz.fuzz_command("classify", 5, artifact)
        self.assertIn(str(artifact), command)
        self.assertNotIn("fuzz/seeds/classify", command)

    def test_rejects_unknown_target_and_bad_duration(self) -> None:
        errors = run_fuzz.validate("unknown", 0, None)
        self.assertTrue(any("unknown fuzz target" in error for error in errors))
        self.assertTrue(any("duration must be positive" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
