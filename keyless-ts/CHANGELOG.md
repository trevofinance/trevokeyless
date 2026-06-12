# Changelog — @trevofinance/trevokeyless (TypeScript)

The byte encoding of keyless addresses is frozen; `conformance/vectors.json` at the
repository root is the executable specification shared with the Rust and Python
packages.

## 0.7.0 — 2026-06-12

First public release line. Breaking changes from 0.6.0 (pre-1.0 API freeze):

- **The typed entry points are renamed** to include the noun:
  `encodeAppAgentAddress`, `encodeTransactionalAddress`, `encodeNamedAddress`,
  `decodeAppAgentAddress`, `decodeTransactionalAddress`, `decodeNamedAddress`.
  No aliases.
- **`decodeAddress` and `isKeylessAddress` accept SS58 strings only.** Raw account ids
  (`0x` + exactly 64 hex characters) are classified with the new `decodeAccountId`,
  which takes no `ss58Format` parameter — raw bytes carry no format, and the old
  passthrough silently bypassed the expected-format check.
- **New `encodeAddress`**: renders a raw account id (`0x` + exactly 64 hex characters,
  the `decodeAccountId` input contract) as its canonical SS58 address, in the Trevo
  format by default — the previously missing reverse direction between the two
  interchange formats.
- **`decodeAccountId` results carry `address: null`**: raw account bytes have no SS58
  format, so there is no address to report (previously the field held the raw input
  verbatim); render one explicitly with `encodeAddress`.
- **The `Ss58ErrorKind` type is exported** (the type of `Ss58Error.kind`), matching
  the Python package's public `Ss58ErrorKind` alias.
- **`AddressType` is an erasable `const` object + union type, not a TS `enum`.** The
  values (0–3) and `AddressType.AppAgent`-style usage are unchanged; the numeric
  reverse lookup (`AddressType[1] === "AppAgent"`) no longer exists — derive names
  via `Object.entries(AddressType)` if needed. The values still match the Python
  `AddressType` and the Rust `AccountType` SCALE indices.
- **`@polkadot/util` and `@polkadot/util-crypto` are gone.** The SS58 codec is now a
  direct port of the Rust reference on `@noble/hashes` + `@scure/base` (the only two
  production dependencies, both audited and zero-dependency). Address output is
  byte-identical; SS58 failures now throw the exported `Ss58Error`, whose `kind`
  mirrors the Rust `Ss58Error` variants and whose messages match the Rust `Display`
  output (e.g. `Expected SS58 format 5335 but found 42` with `expected`/`found`
  fields).
- **`SS58_FORMAT__TREVO_ASSET_HUB` renamed to `SS58_FORMAT_TREVO_ASSET_HUB`** (single
  underscore, matching the Rust crate). No alias.
- **Error wording unified cross-language:** name validation now throws
  `Address name must be exactly 10 characters long` /
  `Address name contains invalid characters`.
- **The SS58 length pre-bound is measured in UTF-8 bytes**, matching the Rust
  reference decoder (which checks the byte length of the input): non-ASCII input over
  50 bytes now throws kind `InvalidLength` instead of `InvalidBase58`, even when it
  stays within 50 UTF-16 units. Pinned by new shared `ss58_invalid` vectors.
- **Node >= 22 required** (Node 20 reached end of life in April 2026); compiled output
  targets ES2022.
- Package metadata: `keywords` and the `bugs` URL for the npm registry page.

## 0.6.0

- Validation alignment with the Rust reference (u32 ranges, ss58Format range check,
  canonical SS58 structure enforcement on top of the @polkadot decoder).
