# Changelog — trevo-keyless (Rust)

The byte encoding of keyless accounts is frozen; `conformance/vectors.json` at the
repository root is the executable specification shared with the Python and TypeScript
packages.

## 0.9.0 — 2026-06-12

First public release line. Changes from 0.8.0:

- The SCALE `Decode` error message for `AccountName` now reads
  `Account name contains invalid characters`, matching the `Display` wording of
  `AccountNameError::InvalidCharacter`.
- New cargo-fuzz target (`fuzz/fuzz_targets/ss58_decode.rs`) for the SS58 decoder;
  excluded from the published crate.
- The cross-language conformance fixture gained two additive groups
  (`named_invalid_encode`, `hex_invalid`); no existing vector changed.
- README design notes: account-vs-address API rationale, unconditional SCALE
  dependencies, bare-hex `Display`.

## 0.8.0

- Validating serde deserialization for `BlockchainAccountInfo`, richer decode errors
  (`KeylessDecodeError` variants carry the actual classification), property tests,
  SS58 strictness hardening (length pre-bound, canonical structure only).
