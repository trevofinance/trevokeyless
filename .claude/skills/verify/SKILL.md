---
name: verify
description: Prove a change to the trevokeyless implementations actually works end-to-end — the fixture-drift check, the affected package suites, and the cross-language differential harness.
---

# Verifying a change

1. **Identify the blast radius**: which of the three implementations changed, and did
   observable behavior change (encoding, validation, errors, public API) or only
   internals?
2. **Fixture drift check** (any behavioral change): regenerate the generator output to
   a scratch file and diff it against the committed fixture — any difference that is
   not part of the diff under review is a missed pin:
   ```
   cd keyless-rust
   cargo run --example gen_conformance --features ss58 > "$SCRATCH/vectors.json"
   diff "$SCRATCH/vectors.json" ../conformance/vectors.json
   ```
   (use the session scratchpad directory for `$SCRATCH`)
3. **Run the changed package's full suite** (exact commands: the gates skill); for
   cross-cutting changes, all three.
4. **Run the differential harness** — the strongest end-to-end signal in this repo
   (Rust as oracle, Python and TypeScript compared against it on random cases):
   ```
   cd keyless-ts && npm run build      # the harness runs dist/esm
   cd .. && python3 .github/scripts/check_cross_language.py --cases 256
   ```
5. **Error-message or validation-order changes**: additionally run the twin adversarial
   suites (keyless-python/tests/test_adversarial.py,
   keyless-ts/tests/adversarial.test.ts) and confirm they still mirror each other
   case-for-case.

Report what was actually executed and observed, including failures verbatim — never
"should work".
