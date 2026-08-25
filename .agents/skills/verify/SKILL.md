---
name: verify
description: Prove a change to the trevokeyless implementations actually works end-to-end — the fixture-drift check, the affected package suites, and the cross-language differential harness.
metadata: {"authority":"non-authoring","review_guard":"required-at-top-level"}
---

# Verifying a change

This is non-authoring diagnostic work. Establish one `review-guard` when invoked at the
top level, or reuse an explicitly caller-owned guard when composed by an authoring or
review workflow. Verify the guard after every selected check completes.

Resolve paths from the repository root.

1. Identify the blast radius through `docs/development.md`; execute named sets through
   `.github/scripts/run_checks.py`.
2. For behavior that can affect the fixture, read `docs/conformance.md`, create a
   scratch directory with `mktemp -d`, run the documented generator with its output
   redirected into that directory, and compare it byte-for-byte with the committed
   fixture. Treat unexplained drift as a missed pin.
3. Apply the `gates` skill for every affected package. Use `all` for a cross-cutting change
   rather than invoking the same package sets again separately.
4. Invoke the `cross` gate when shared behavior, the fixture, or a refactor on a shared
   encode/decode path could affect parity and it was not already included by `all`.
5. For error-message, validation-order, account-name, or host-input changes, run the
   focused adversarial coverage required by `docs/cross-language-contracts.md` in
   addition to the normal gates, and compare the twin suites where the contract requires
   it.

Report only what was actually executed and observed. Preserve decisive failure text
verbatim but keep excerpts compact; never replace evidence with "should work".
