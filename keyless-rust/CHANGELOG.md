# Changelog — trevo-keyless (Rust)

The byte encoding of keyless accounts is frozen; `conformance/vectors.json` at the
repository root is the executable specification shared with the Python and TypeScript
packages.

## 0.9.0 — 2026-06-12

First public release line. Changes from 0.8.0:

- **Breaking:** `AccountId32`'s `Display` now prints the cross-language raw
  interchange form `0x` + 64 lowercase hex characters (previously bare hex without
  the prefix), so a Rust-rendered account id is accepted verbatim by the Python
  `decode_account_id` and TypeScript `decodeAccountId` entry points.
- The SCALE `Decode` error message for `AccountName` now reads
  `Account name contains invalid characters`, matching the `Display` wording of
  `AccountNameError::InvalidCharacter`.
- New cargo-fuzz target (`fuzz/fuzz_targets/ss58_decode.rs`) for the SS58 decoder;
  excluded from the published crate.
- The cross-language conformance fixture gained additive groups and fields:
  `named_invalid_encode`, `hex_invalid`, `hex_valid` (mixed-case raw account id
  acceptance), and an `error_kind` on every `ss58_invalid` vector pinning the
  `Ss58Error` variant each rejection must produce in all three languages. No value
  field of any existing vector changed.
- README design notes: account-vs-address API rationale, unconditional SCALE
  dependencies, bare-hex `Display`.

## 0.8.0

- Validating serde deserialization for `BlockchainAccountInfo`, richer decode errors
  (`KeylessDecodeError` variants carry the actual classification), property tests,
  SS58 strictness hardening (length pre-bound, canonical structure only).
