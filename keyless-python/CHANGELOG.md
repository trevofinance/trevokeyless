# Changelog — trevokeyless (Python)

The byte encoding of keyless addresses is frozen; `conformance/vectors.json` at the
repository root is the executable specification shared with the Rust and TypeScript
packages.

## 0.5.0 — 2026-06-12

First public release line. Breaking changes from 0.4.0 (pre-1.0 API freeze):

- **`decode_address` and `is_keyless_address` accept SS58 strings only.** Raw account
  ids (`0x` + exactly 64 hex characters) are classified with the new
  `decode_account_id`, which takes no `ss58_format` parameter — raw bytes carry no
  format, and the old passthrough silently bypassed the expected-format check.
- **`trevokeyless.ss58` is private.** The vendored generic Substrate SS58 codec moved
  to `trevokeyless._ss58`; its permissive semantics (hex passthrough, non-32-byte
  payloads) were never part of this library's contract.
- **`SS58_FORMAT__TREVO_ASSET_HUB` renamed to `SS58_FORMAT_TREVO_ASSET_HUB`** (single
  underscore, matching the Rust crate). No alias.
- **New `Ss58FormatMismatchError`** (a `ValueError` subclass with `expected`/`found`
  attributes) replaces the bare `ValueError("Invalid SS58 format")`; the message is now
  `Expected SS58 format {expected} but found {found}`.
- **Error wording unified cross-language:** the name-length error is now
  `Address name must be exactly 10 characters long`, and the
  `AddressTypeMismatchError` message no longer ends with a period (byte-identical to
  TypeScript).
- Dependency floor raised to `base58>=2.1.1`; added the `Typing :: Typed` classifier.

## 0.4.0

- Validation alignment with the Rust reference (u32 ranges, bool rejection, SS58
  strictness, charset-validated name decoding), vendored SS58 hardening.
