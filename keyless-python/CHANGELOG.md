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
- **New `encode_address`**: renders a raw account id (`0x` + exactly 64 hex
  characters, the `decode_account_id` input contract) as its canonical SS58 address,
  in the Trevo format by default — the previously missing reverse direction between
  the two interchange formats.
- **The SS58 codec is a strict typed port of the Rust reference decoder.** The
  vendored Polkascan codec — hex passthrough, account-index and 33-byte payloads,
  `is_valid_ss58_address` — is gone (the module stays private as
  `trevokeyless._ss58`). Acceptance behavior is unchanged and pinned by the shared
  conformance fixture.
- **SS58 failures raise the new `Ss58DecodeError`** (a `ValueError` subclass): its
  `kind` attribute mirrors the Rust `Ss58Error` variants (`InvalidBase58 |
  InvalidLength | InvalidChecksum | InvalidFormat | UnexpectedFormat`) and the
  messages match the Rust `Display` output. `Ss58FormatMismatchError`
  (`expected`/`found` attributes) is its `UnexpectedFormat` case and is raised only
  after an address fully validates — a corrupt address of a non-expected format now
  raises its actual validation error instead of a misleading format mismatch. The
  `Ss58ErrorKind` type alias is exported.
- **`decode_account_id` results carry `address=None`**: raw account bytes have no
  SS58 format, so there is no address to report (previously the field held the raw
  input verbatim); render one explicitly with `encode_address`.
- **Non-string inputs raise `TypeError`** (previously an `AttributeError` leaked from
  string method lookups) for address, account id, and name parameters.
- **Wrong-type numeric parameters raise `TypeError` too** (previously `ValueError`): a
  non-int (`bool` included) where an AppAgent ID, TA ID, or `ss58_format` is expected
  is a type error, consistent with the string parameters; out-of-range ints still
  raise `ValueError`.
- **`SS58_FORMAT__TREVO_ASSET_HUB` renamed to `SS58_FORMAT_TREVO_ASSET_HUB`** (single
  underscore, matching the Rust crate). No alias.
- **Error wording unified cross-language:** the name-length error is now
  `Address name must be exactly 10 characters long`, and the
  `AddressTypeMismatchError` message no longer ends with a period (byte-identical to
  TypeScript).
- **The SS58 length pre-bound is measured in UTF-8 bytes**, matching the Rust
  reference decoder (which checks the byte length of the input): non-ASCII input over
  50 bytes now raises kind `InvalidLength` instead of `InvalidBase58`, even when it
  stays within 50 characters. Pinned by new shared `ss58_invalid` vectors.
- **Zero runtime dependencies:** the strict base58 codec is vendored in
  `trevokeyless._ss58`; the `base58` package dependency is gone (its `str.rstrip`
  input leniency was already being defended against with an explicit charset check,
  which stays). Added the `Typing :: Typed` classifier.

## 0.4.0

- Validation alignment with the Rust reference (u32 ranges, bool rejection, SS58
  strictness, charset-validated name decoding), vendored SS58 hardening.
