---
paths:
  - conformance/**
  - keyless-rust/examples/gen_conformance.rs
  - keyless-rust/src/tests/tests_conformance.rs
  - keyless-python/tests/test_conformance.py
  - keyless-ts/tests/conformance.test.ts
---

# Conformance fixture rules

- NEVER hand-edit conformance/vectors.json (a PreToolUse hook blocks it). Regenerate:
  `cd keyless-rust && cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json`
- Review the regenerated diff. A change to a *value* field (hex, ss58, expected,
  account_id) of an existing vector means the frozen on-chain encoding changed — that
  needs a very good reason. New vectors/groups are the normal way to pin new behavior.
- The three suites pin the exact group set AND each group's exact vector count. The pins
  are deliberately hand-maintained; move them together with the fixture:
  - keyless-rust/src/tests/tests_conformance.rs — FIXTURE_GROUPS (and its array length)
  - keyless-python/tests/test_conformance.py — _FIXTURE_GROUP_SIZES
  - keyless-ts/tests/conformance.test.ts — EXPECTED_GROUP_SIZES
- Every generator vector needs a self-check assert! deriving it from the real
  implementation. A new group additionally needs a consuming test in each suite, an
  entry in the TypeScript `Vectors` interface, and a schema section in
  conformance/README.md (the authoritative group inventory).
- Full step-by-step procedure: the add-conformance-vector skill, or
  conformance/README.md ("Adding a vector or a group").
