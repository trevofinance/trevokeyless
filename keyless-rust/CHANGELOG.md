# Changelog — trevokeyless (Rust)

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the
crate follows [semantic versioning](https://semver.org). The byte encoding itself is
frozen and never changes at any version.

## [2.0.0] — 2026-07-21

Everything below is the delta against 1.1.0, the previous release on crates.io.

### Added

- `ACCOUNT_ID_LENGTH` (32) and `ACCOUNT_NAME_LENGTH` (10) constants, exported under the
  same names by the Python and TypeScript implementations (which in turn now export the
  byte-`[4]` discriminants this crate already had).
- `InvalidSs58Format`, the dedicated error of `AccountId32::to_ss58check_with_version`.
  Carries the rejected `format()`, renders the same message as `Ss58Error::InvalidFormat`,
  and converts into it via `From`.
- `parity_scale_codec::EncodeLike` for `BlockchainAccountInfo`. Its hand-written `Encode`
  impl had silently lacked it, which made the type unusable as a Substrate pallet call
  argument, event field, or storage value (`Parameter` and `FullCodec` both require it).
- `TryFrom<&[u8]>` and `Default` for `AccountId32`, and `TryFrom<&str>` for `AccountId64` /
  `AccountId128` — closing conversion asymmetries with `AccountName` and `AccountId32`
  respectively. The slice conversion returns the new `InvalidAccountIdLength`.
- `TryFrom<u8>` for `AccountType`, the checked inverse of `AccountType as u8` (returning
  the new `InvalidAccountType` with the rejected `value()`). The numeric values are a
  cross-language contract and, for the keyless variants, the byte-`[4]` discriminant, so a
  consumer holding that byte no longer hand-writes the match.
- Inherent `as_bytes` / `as_slice` accessors on `AccountId32` and `AccountName`. Both
  types implement `AsRef<[u8; N]>` *and* `AsRef<[u8]>`, so a bare `.as_ref()` cannot
  infer; the inherent accessors are the unambiguous way to reach the bytes.

### Changed

- **Breaking:** `AppAgentId` and `TransactionalId` are distinct newtypes, and the
  Transactional and Named encoders accept named identifier structs
  (`TransactionalAccountIdentifiers`, `NamedAccountIdentifiers`), preventing the
  two numeric identifiers from being silently transposed.
- **Breaking:** the SCALE representation of `BlockchainAccountInfo` is now exactly
  its 32-byte `AccountId32`; decoding derives the classification and identifiers, so
  SCALE cannot construct an internally inconsistent value. Its struct variants are
  non-exhaustive so downstream crates cannot construct inconsistent values directly,
  and serde deserialization re-validates that the embedded identifiers match
  `account_id`.
  *Migrating persisted 1.x values:* the 1.x wire form was the derived enum layout
  (a variant-index byte, then the fields); 2.0.0 reads exactly the 32 account-id
  bytes, so 1.x payloads no longer decode. A runtime (or off-chain store) holding
  1.x-encoded values needs a one-time storage migration: decode each value with a
  vendored copy of the 1.x enum definition and re-encode it — which now means writing
  just its `account_id` bytes, since the classification re-derives on decode. Values
  that only ever crossed the wire transiently (call arguments, events) need no
  migration, only a coordinated upgrade on both sides of the wire.
- **Breaking:** the serde impls of `AccountId32` (and of the `test-accounts`
  `AccountId64` / `AccountId128`) now follow `is_human_readable()`: an SS58 address
  string (the `0x` hex form for the wide ids) into human-readable formats such as
  JSON, the raw bytes into binary formats such as bincode or postcard. Previously
  every format got the string form.
- **Breaking:** `AccountId32::to_ss58check_with_version` now returns
  `Result<String, InvalidSs58Format>` instead of `Result<String, Ss58Error>`. An unusable
  format is the only way encoding can fail — there is no base58 to parse, no checksum to
  verify and no expected format to compare — so callers no longer have to match on three
  unreachable `Ss58Error` variants. It is a rejected *argument*, which is how the Python and
  TypeScript implementations classify it (`ValueError` / `RangeError`, not
  their `Ss58Error`). Callers wanting a single error type across encode and decode can keep
  using `?` or `.into()`; the five `Ss58Error` kinds and every rendered message are unchanged,
  as is the conformance fixture.
- **Breaking:** `AccountId32::from_ss58check_expecting` now returns the two-variant
  `Ss58ExpectingError` instead of folding a rejected `expected_format` argument into
  `Ss58Error::InvalidFormat`. An `expected_format` outside the 14-bit range is reported
  as `InvalidExpectedFormat(InvalidSs58Format)` — the decoder twin of the encoder's
  dedicated argument error above, and of the Python/TypeScript parameter-validation
  errors — while everything the address can be blamed for arrives as `Address(Ss58Error)`,
  so a caller can finally tell "my parameter was malformed" from "the address is bad" by
  type. Converts into `Ss58Error` via `From` for callers keeping the previous single
  taxonomy. `from_ss58check` is unchanged (its constant format cannot be rejected); the
  five `Ss58Error` kinds, every rendered message, and the conformance fixture are
  unchanged.
- The `AccountNameError::InvalidLength` display message now reads "Account name must
  be exactly 10 characters long" (previously "… 10 bytes long"), unifying the
  account-name validation wording shared verbatim by the Rust, Python, and TypeScript
  implementations. The validated quantity is unchanged (byte length; identical to the
  character count for every accepted name).
- The error types implement `core::error::Error` unconditionally instead of gating
  `std::error::Error` behind the `std` feature (the two are the same trait, stable in
  `core` since Rust 1.81). `no_std` builds now get the `Error` impls; `std` builds are
  unaffected. The `std` feature now only enables the `std` features of the dependencies.
- Public enum SCALE indices and public struct field order are explicit and covered by
  golden-byte tests, making ABI changes deliberate.

### Fixed

- `cargo test` on the published crate no longer fails on the absent conformance fixture: the
  fixture tests now skip when the repository's `conformance/` directory is not present, and
  still fail loudly when it is. `examples/differential.rs` is no longer shipped.
- `AccountId64` and `AccountId128` parsers now report their actual required hex
  widths instead of the 32-byte account width.
- Repository URL in the crate metadata and documentation links pointed at a
  nonexistent repository (`trevo-keyless` instead of `trevokeyless`).
- The `scale` feature now requests `parity-scale-codec/derive` explicitly instead of relying
  on `scale-info` to pull the derive macros in transitively.
- `extern crate alloc` is gated on `serde` as well as `ss58`; the serde impls use `alloc`
  directly and compiled only by way of `serde` implying `ss58`.
- docs.rs now labels feature-gated items with the feature that provides them.

### Internal

- The `named_invalid_encode` conformance group now pins the *kind* of every account-name
  rejection (`error_kind`: `InvalidLength` or `InvalidCharacter`, read back from this
  crate's own validation while generating) and gained non-ASCII vectors — a 10-UTF-8-byte
  name with a non-ASCII character, an emoji name, a combining-mark (NFD) name — that
  separate the UTF-8 byte length from the code-point and UTF-16 lengths. The shared rule is
  this crate's: byte length first, then the charset. Rejection was already pinned; the
  classification previously drifted between the implementations. No encoding change.
- The conformance fixture gained encode-side invalid-format vectors
  (`format_invalid_encode` group), positive AppAgent/Transactional classification
  vectors, a plain-lowercase `hex_valid` vector, and canonical spellings for the
  `ss58_decode_only` group; the Rust conformance suite now also exercises
  `from_ss58check_with_version` (unknown-format decoding) against the fixture.
- The conformance suite pins each fixture group's exact vector count, so a generator edit
  that silently shrinks a group can no longer regenerate cleanly, and pins the numeric
  `AccountType` values and the layout constants shared with the Python/TypeScript libraries.
- New `classify` fuzz target over `decode_account_info` (arbitrary 32-byte account ids),
  asserting the classification invariants: the reported account id is the input,
  `is_keyless_account` agrees with it, keyless classifications re-encode to the exact input
  bytes and agree with the typed decoders, the other typed decoders report the actual type,
  and any single-bit edit of a keyless account demotes it to `Regular`. Seeded from the
  conformance fixture; SCALE `MaxEncodedLen`/`TypeInfo` values, negative SCALE decoding,
  and the serde wire shapes of all public types are pinned by tests, and committed fuzz
  seed corpora exercise the accept paths of all fuzz targets.
- Added a seeded Rust/Python/TypeScript differential repository check.
- Locked the standalone fuzz harness dependencies and added their lockfile to the
  RustSec audit and Dependabot paths; CI also pins its nightly and cargo-fuzz versions.

## [1.1.0] — 2026-06-24

### Added

- The `test-accounts` feature: `AccountId64` and `AccountId128`, fixed-width padded
  account-id containers for Substrate runtime tests that instantiate
  `frame_system::Config::AccountId` at widths other than 32 bytes. Each carries a
  32-byte keyless account in its leading bytes, right-padded with zeros, and reuses the
  32-byte codec through `From<AccountId32>` / `to_account_id32()`. Off by default.

## [1.0.1] — 2026-06-23

### Changed

- Development-dependency and CI refresh (`array-bytes` 6.2.3 → 9.3.0, GitHub Actions
  bumps). The library, its runtime dependencies, and the encoding are unchanged.

## [1.0.0] — 2026-06-23

### Added

- Initial stable release: keyless account encoding/decoding/classification
  (`encode_*_account`, `decode_account_info`, typed `decode_*_account`,
  `is_keyless_account`), `AccountId32` with the `0x` + 64-hex interchange form, the
  SS58 codec (feature `ss58`), SCALE (feature `scale`) and serde (feature `serde`)
  support, `no_std` compatibility, and the cross-language conformance fixture.
