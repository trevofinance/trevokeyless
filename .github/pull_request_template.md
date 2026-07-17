## What and why

<!-- What changes, and what problem it solves. Link any issue. -->

## Checklist

- [ ] **The one rule** (see
      [CONTRIBUTING.md](https://github.com/trevo-finance/trevokeyless/blob/main/CONTRIBUTING.md),
      linked absolutely because relative links do not resolve from an instantiated PR
      body): if this changes behaviour of
      the encoding, decoding, classification, SS58 codec, or the shared error taxonomy, then the
      regenerated `conformance/vectors.json` **and** the matching changes to **all three**
      implementations (Rust, Python, TypeScript) are in *this* pull request.
      <!-- Regenerate with:
           cd keyless-rust && cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json -->
- [ ] No `value` field of an *existing* conformance vector changed — or, if one did, the
      description below says why the frozen on-chain encoding had to change.
- [ ] Shared error wording (the SS58 messages and the two account-name messages) is still
      identical across the three implementations, or was changed in all three at once.
- [ ] Public API added to a package? Its **minor** version is bumped and its `CHANGELOG.md` has
      an entry.
- [ ] Docs updated (package `README.md` examples stay executable — CI runs them; adding or
      removing a Python/TypeScript example means updating the pinned block count in
      `keyless-python/tests/test_readme_examples.py` / `keyless-ts/scripts/run-readme.mjs`).
