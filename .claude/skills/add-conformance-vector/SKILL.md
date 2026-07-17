---
name: add-conformance-vector
description: Add a vector or a new group to the cross-language conformance fixture (conformance/vectors.json) — the generator change, the regeneration, the three count-pin bumps, and the new-group extras.
---

# Adding a conformance vector or group

The fixture is generated, never hand-edited, and all three suites pin the exact group
set and per-group vector counts — so an addition that stops at the fixture fails every
suite. The full path (authoritative copy: conformance/README.md, "Adding a vector or a
group"):

1. **Generate.** Add the vectors (or the new group) to
   keyless-rust/examples/gen_conformance.rs. Follow the existing pattern: derive every
   value from the real implementation and add a self-check assert! that would catch the
   vector being wrong at generation time. For a new group, add its print_group(...)
   call — the last group (and only it) passes `true`.
2. **Regenerate** and review the diff:
   `cd keyless-rust && cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json`
   A changed *value* field of an existing vector is a frozen-format break — stop and
   confirm. Additions are the normal case.
3. **Bump the count pins in all three suites** (hand-maintained on purpose):
   - keyless-rust/src/tests/tests_conformance.rs — FIXTURE_GROUPS (and its array length)
   - keyless-python/tests/test_conformance.py — _FIXTURE_GROUP_SIZES
   - keyless-ts/tests/conformance.test.ts — EXPECTED_GROUP_SIZES
4. **For a new group**: add a consuming test to each of the three suites (a
   generated-but-unconsumed group fails everywhere), the group's entry in the
   TypeScript `Vectors` interface (conformance.test.ts), and a schema section in
   conformance/README.md — that file is the authoritative group inventory.
5. **Run everything**: the three test suites and
   `python3 .github/scripts/check_cross_language.py --cases 256` (its decode section
   replays the ss58_invalid and ss58_decode_only groups; build keyless-ts first).

All of it lands in one PR together with the behavioral change it pins.
