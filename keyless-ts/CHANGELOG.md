# Changelog — @trevofinance/trevo-keyless (TypeScript)

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
- **Node >= 22 required** (Node 20 reached end of life in April 2026); compiled output
  targets ES2022.

## 0.6.0

- Validation alignment with the Rust reference (u32 ranges, ss58Format range check,
  canonical SS58 structure enforcement on top of the @polkadot decoder).
