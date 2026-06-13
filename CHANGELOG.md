# Changelog

The three packages are versioned independently but reach their first stable public
release together at **1.0.0**. The keyless byte encoding and the SS58 format (5335) are
frozen; only the public APIs differ per language. The keyless checksum is classification
and integrity, **not** authentication — see the
[security model](./README.md#security-model).

## 1.0.0 — first stable release

### All implementations
- Frozen byte encoding and SS58 format, pinned by the shared
  [`conformance/vectors.json`](./conformance/vectors.json) (schema:
  [`conformance/README.md`](./conformance/README.md)), verified by every suite and
  regenerated from Rust in CI.
- Strict SS58 decoding: only the canonical structure is accepted (1- or 2-byte prefix,
  32-byte account id, full 2-byte checksum, non-reserved format), with a UTF-8-byte
  length pre-bound. The sole accepted non-canonical spelling is the Substrate-compatible
  two-byte-prefix encoding of formats below 64 (the Trevo format 5335 is unaffected).
- Unified classification vocabulary across languages: `AccountType` and the four
  variants `Regular | AppAgent | Transactional | Named`.
- The typed mismatch error message is `"Provided address is not a/an X address but is a
  Y address"` (Python and TypeScript).

### Rust (`trevokeyless`)
First release under the name `trevokeyless` (previously `trevo-keyless`).
- **Breaking:** `KeylessDecodeError` is now a single `WrongAccountType { expected, found }`
  variant (was `NotAnAppAgentAccount` / `NotATransactionalAccount` / `NotANamedAccount`,
  each carrying only `found`). Its serde and SCALE encodings change accordingly.
- **Breaking:** `decode_transactional_account` and `decode_named_account` return
  `DecodedTransactionalAccount { app_agent_id, ta_id }` and
  `DecodedNamedAccount { app_agent_id, account_name }` instead of bare tuples.
- `AccountId32` parses/render the `0x` + 64-hex interchange form (`FromStr` / `Display`);
  `from_ss58check_expecting` validates fully before comparing the expected format.
- Cargo features `std` (default), `ss58`, `scale`, `serde`; `no_std` (+ alloc) supported.
  SCALE `Decode` of `BlockchainAccountInfo` performs no consistency validation (serde
  deserialization does).

### Python (`trevokeyless`)
Migrating from the 0.1.x line:
- **Breaking:** classification vocabulary renamed to match the reference —
  `AddressType` → `AccountType`, `BlockchainAddressInfo` → `BlockchainAccountInfo`
  (fields `address_type` → `account_type`, `address_name` → `account_name`),
  `AddressName` → `AccountName`, `AddressTypeMismatchError` → `AccountTypeMismatchError`.
  The address-oriented function names and the `BlockchainAddress` / `BlockchainAccountId`
  aliases are unchanged; the nullable `address` field stays.
- **Breaking:** the SS58 error type was renamed `Ss58DecodeError` → `Ss58Error` (with a
  `kind` attribute), and the vendored decoder is now strict — the historical `0x` hex
  passthrough, surrounding-whitespace tolerance, and non-account payload lengths
  (account-index, 33-byte keys) are rejected.
- **Breaking:** `TransactionalAddressId` type alias renamed to `TransactionalId`.
- `decode_transactional_address` / `decode_named_address` now return the
  `DecodedTransactionalAddress` / `DecodedNamedAddress` named tuples (backward compatible
  with tuple unpacking and equality).
- `__version__` is no longer resolved from installed package metadata, so `import
  trevokeyless` works from a source tree or vendored copy.
- Zero runtime dependencies; ships `py.typed`.

### TypeScript (`@trevofinance/trevokeyless`)
First release under the name `@trevofinance/trevokeyless` (previously
`@trevofinance/trevo-keyless`).
- **Breaking:** classification vocabulary renamed — `AddressType` → `AccountType`,
  `BlockchainAddressInfo` → `BlockchainAccountInfo` (fields `addressType` →
  `accountType`, `addressName` → `accountName`), `AddressTypeMismatchError` →
  `AccountTypeMismatchError`. Function names and `NAMED_ADDRESS_LENGTH` are unchanged.
- `encodeNamedAddress` validates `appAgentId` before the name (argument order), matching
  Python.
- Ships a single CommonJS build with type declarations; works from both `require` and
  ESM `import`. Runtime dependencies: `@noble/hashes`, `@scure/base`.
