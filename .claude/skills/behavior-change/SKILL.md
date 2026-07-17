---
name: behavior-change
description: End-to-end checklist for changing the trevokeyless encoding/validation behavior — keeps Rust, Python, TypeScript, the conformance fixture, and the shared contracts in lockstep within one PR.
---

# Behavioral change checklist

The one rule: every behavioral change lands together with the regenerated conformance
fixture and the matching changes in all three implementations, in the same pull request.

1. **Design in Rust first** (keyless-rust/ is the source of truth). If the change would
   alter a *value* an existing vector pins, stop and confirm with the user — that is a
   change to the frozen on-chain format.
2. **Implement in Rust** with its tests, and extend
   keyless-rust/examples/gen_conformance.rs with vectors pinning the new behavior (a
   self-check assert! per vector, deriving the value from the real implementation).
3. **Regenerate the fixture and review the diff** — then follow the
   add-conformance-vector skill for the count-pin bumps (three suites) and new-group
   extras.
4. **Port to Python and TypeScript**, preserving:
   - the validation ORDER (contractual; pinned by the both-invalid ordering tests in
     test_adversarial.py / adversarial.test.ts);
   - the shared error MESSAGES verbatim (SS58 wording and the two account-name
     messages);
   - the error KINDS (the five Ss58Error kinds; name kinds
     InvalidLength/InvalidCharacter; the typed-decoder (expected, found) pair).
   - Host-only input classes (lone UTF-16 surrogates, host-number representations)
     cannot go in the fixture; add them to BOTH twin adversarial suites, kept
     case-for-case.
5. **Public API additions**, per package: Python `__all__` + tests/test_public_surface.py;
   TS export list + tests/public-surface.test.ts; a README section + executed example
   (bump the pinned block counts); the package needs at least a minor version bump
   before its next release.
6. **Verify**: run the gates skill for all packages, plus
   `python3 .github/scripts/check_cross_language.py --cases 256` (build keyless-ts
   first).
7. Everything above ships as ONE pull request.
