"""Validate the high-risk workflow registry and deterministic corpora."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any, ClassVar

import tomllib

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / ".agents/workflow-risk.toml"
SKILLS = ROOT / ".agents/skills"
EVALS = ROOT / ".agents/evals"
CASE_FIELDS = {
    "schema",
    "id",
    "case_type",
    "prompt",
    "expected_entrypoint",
    "expected_activation",
    "expected_authority",
    "expected_guard",
    "expected_output_contains",
    "allowed_side_effects",
    "forbidden_side_effects",
}


class WorkflowRiskTests(unittest.TestCase):
    registry: ClassVar[dict[str, Any]]

    @classmethod
    def setUpClass(cls) -> None:
        with REGISTRY.open("rb") as handle:
            cls.registry = tomllib.load(handle)

    def test_registry_and_suites_match_exactly(self) -> None:
        registered = {entry["suite"] for entry in self.registry["workflow"]}
        discovered = {path.relative_to(ROOT).as_posix() for path in EVALS.glob("*/cases.jsonl")}
        self.assertEqual(registered, discovered)

    def test_entrypoints_and_participants_are_canonical_skills(self) -> None:
        for workflow in self.registry["workflow"]:
            names = [workflow["entrypoint"], *workflow.get("participants", [])]
            for name in names:
                with self.subTest(workflow=workflow["id"], skill=name):
                    self.assertTrue((SKILLS / name / "SKILL.md").is_file())

    def test_every_suite_has_complete_case_contract(self) -> None:
        required_types = set(self.registry["required_case_types"])
        seen_ids: set[str] = set()
        for workflow in self.registry["workflow"]:
            cases = [json.loads(line) for line in (ROOT / workflow["suite"]).read_text().splitlines() if line.strip()]
            self.assertEqual({case["case_type"] for case in cases}, required_types)
            for case in cases:
                with self.subTest(workflow=workflow["id"], case=case.get("id")):
                    self.assertEqual(set(case), CASE_FIELDS)
                    self.assertEqual(case["schema"], self.registry["case_schema"])
                    self.assertEqual(case["expected_entrypoint"], workflow["entrypoint"])
                    self.assertEqual(case["expected_authority"], workflow["authority"])
                    self.assertEqual(case["expected_guard"], workflow["review_guard"])
                    self.assertNotIn(case["id"], seen_ids)
                    seen_ids.add(case["id"])
                    self.assertIsInstance(case["expected_activation"], bool)
                    self.assertTrue(case["prompt"])
                    self.assertTrue(case["expected_output_contains"])
                    self.assertTrue(case["forbidden_side_effects"])


if __name__ == "__main__":
    unittest.main()
