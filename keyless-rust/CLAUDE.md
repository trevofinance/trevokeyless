# CLAUDE.md — trevokeyless

Pure-Rust library for encoding and decoding Trevo keyless accounts. No Substrate/frame dependency;
only `blake2b_simd` (plus optional `parity-scale-codec` + `scale-info`, `bs58`, and `serde` —
see Cargo features below).

## Cargo features

- `std` (default) — `std` features of the dependencies. Disable for `no_std` builds (the
  crate builds for `wasm32v1-none`). The error types implement `core::error::Error`
  unconditionally, so `no_std` builds keep the `Error` impls.
- `ss58` — SS58 address encoding/decoding for `AccountId32` (`to_ss58check*`,
  `from_ss58check*`, `SS58_FORMAT_TREVO_ASSET_HUB = 5335`). `no_std`-compatible but requires
  an allocator.
- `scale` — SCALE codec and metadata support
  (`Encode`/`Decode`/`DecodeWithMemTracking`/`TypeInfo`/`MaxEncodedLen` for every public
  type), for direct use inside a Substrate runtime.
- `serde` — serde support (implies `ss58` — permanently: the human-readable form is
  *defined* as the SS58 address, and with Cargo feature unification a hex fallback in a
  hypothetical `serde`-without-`ss58` would let one crate's `ss58` choice flip every
  other consumer's JSON wire format). `AccountId32` follows `is_human_readable()`: an
  SS58 string in the Trevo format into JSON/YAML/TOML, the raw 32 bytes into a binary format
  (bincode, postcard, MessagePack) — the same split for `AccountId64`/`AccountId128` with
  their hex form. `AccountName` (de)serializes as a string, other public types via derived
  impls. Deserializing `BlockchainAccountInfo` re-validates that the embedded identifiers
  match `account_id`. Its SCALE representation is only `AccountId32` and derives identifiers
  during decode. Its serde shape is the externally tagged Rust enum — not interchangeable
  with the Python/TS info objects.
- `test-accounts` — `AccountId64` / `AccountId128`: fixed-width opaque account ids for
  Substrate runtime tests that instantiate `frame_system::Config::AccountId` at widths other
  than 32 bytes (general, non-keyless substrate tests). Each carries a 32-byte keyless account
  in its leading bytes, right-padded with zeros, and reuses the 32-byte codec via pad/unpad
  conversions to `AccountId32` (`From<AccountId32>` / `to_account_id32() -> Option<AccountId32>`,
  the latter `None` unless the padding is canonical zeros). Full `Config::AccountId` trait
  surface under `scale`/`serde`; no SS58; no Python/TS counterpart. Off by default.

## Encoding semantics

The authoritative description of the keyless encoding — what a keyless account is, the
three account types, decoding/validation rules, `AccountName` constraints, the SS58
codec — lives in the crate documentation (`src/lib.rs`, rendered on docs.rs) and is
deliberately not duplicated here. The quick reference:

```
final_account[0..N]  = open_part               final_account[N..32] = blake2_256(open_part)[N..32]

AppAgent root  (N=5):  [0..=3] AppAgentId LE | [4]=1
Transactional  (N=9):  [0..=3] AppAgentId LE | [4]=2 | [5..=8] TransactionalId LE
Named          (N=15): [0..=3] AppAgentId LE | [4]=3 | [5..=14] AccountName (0-9 a-z A-Z - #)
```

Decoding (`decode_account_info`) recomputes the checksum for the discriminant at byte
[4]; any mismatch, unknown discriminant, or out-of-charset Named name bytes →
`BlockchainAccountInfo::Regular`. The Python and TypeScript implementations apply the
same rules.

## File layout

```
src/
  lib.rs           — crate-level docs, module declarations
  types.rs         — AppAgentId, TransactionalId, AccountId32, AccountName, AccountType,
                     BlockchainAccountInfo, TransactionalAccountIdentifiers,
                     NamedAccountIdentifiers,
                     error types
  keyless.rs       — encode_account, encode_*, decode_*, is_keyless_account
  ss58.rs          — SS58 codec for AccountId32 + serde impls (features `ss58` / `serde`)
  test_accounts.rs — AccountId64/AccountId128 padded wrappers + AccountId32 pad/unpad
                     conversions (feature `test-accounts`)
  tests/           — test suite (compiled only under `cfg(all(test, feature = "std"))`)
    mod.rs         — submodule declarations; the shared test+std gate lives on the
                     `mod tests;` line in lib.rs, so only the extra per-feature gates repeat here
    tests_binary.rs  — byte-level structure tests (hardcoded expected byte values)
    tests_conformance.rs — verifies the committed cross-language fixture (see below)
    tests_encoding.rs — encode→decode round-trip tests
    tests_properties.rs — randomized property tests (deterministic inline PRNG, no extra deps)
    tests_ss58.rs    — SS58 codec tests (feature `ss58`)
    tests_version.rs — pins the README install snippet and fuzz lock to CARGO_PKG_VERSION
    tests_serde.rs   — serde tests (feature `serde`)
    test_accounts.rs — AccountId64/AccountId128 pad/unpad + codec tests (feature `test-accounts`)
    types.rs         — tests for the `types` module: AccountName, AccountId32 display/parsing,
                       SCALE codec (feature `scale`)
examples/
  gen_conformance.rs — generates conformance/vectors.json at the repository root (requires feature `ss58`)
  differential.rs    — the Rust oracle of the repository's cross-language differential check
                       (.github/scripts/check_cross_language.py; required-features: `ss58`;
                       the harness additionally passes `serde`).
                       Repository-internal: excluded from the published crate.
fuzz/
  fuzz_targets/ss58_decode.rs — cargo-fuzz target for the SS58 decoder
  fuzz_targets/accountid_from_str.rs — cargo-fuzz target for the AccountId32 hex parser
  fuzz_targets/classify.rs — cargo-fuzz target for decode_account_info (raw 32-byte input:
    round-trip re-encode, typed-decoder agreement, checksum fragility)
  seeds/<target>/ — committed seed corpora (byte-exact, no trailing newline)
  (all nightly-only; the fuzz crate is a standalone workspace excluded from the
  published crate; CI smoke-runs the targets and fuzz.yml runs them on a schedule)
```

Contribution and release workflow (fixture regeneration rule, per-package tagging,
trusted publishing): see ../CONTRIBUTING.md.

All test modules additionally require the `std` feature; run the full suite with
`cargo test --all-features`. MSRV is 1.85 (`rust-version` in Cargo.toml; CI checks it
by running the **full suite** on the 1.85 toolchain, so the dev-dependencies and the test
code must stay MSRV-compatible too). The
repository pins a newer toolchain for development in `../rust-toolchain.toml`.

## Cross-language conformance fixture

`conformance/vectors.json` (repository root) holds encoding, classification, and SS58
vectors derived from this crate — the source of truth — plus the rejection and contract
groups (invalid SS58, name/format/u32 rejection, hex round-trips, typed-decoder
mismatches, and more). The full, authoritative group inventory and per-group JSON schema
live in `conformance/README.md`; do not restate them here (a second copy only drifts).
The Rust, Python, and TypeScript test suites all verify against the committed fixture,
and CI checks that the fixture matches the generator output. After an intentional change
to the encoding semantics, regenerate with:

```
cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json
```

and review the diff together with the corresponding Python/TypeScript changes.

## Public API

```rust
// Encoding (all parameters taken by value; AccountName is a 10-byte Copy type)
encode_app_agent_account(app_agent_id: AppAgentId) -> AccountId32
encode_transactional_account(identifiers: TransactionalAccountIdentifiers) -> AccountId32
encode_named_account(identifiers: NamedAccountIdentifiers) -> AccountId32

// Decoding (all take &AccountId32)
decode_account_info(account: &AccountId32) -> BlockchainAccountInfo  // classifies any account
decode_app_agent_account(account: &AccountId32) -> Result<AppAgentId, KeylessDecodeError>
decode_transactional_account(account: &AccountId32) -> Result<TransactionalAccountIdentifiers, KeylessDecodeError>
decode_named_account(account: &AccountId32) -> Result<NamedAccountIdentifiers, KeylessDecodeError>
is_keyless_account(account: &AccountId32) -> bool

// Classification helpers
AccountType — Regular | AppAgent | Transactional | Named (mirrors the Python/TS AccountType)
AccountType: TryFrom<u8> (Err = InvalidAccountType) — the checked inverse of `as u8`
BlockchainAccountInfo::{account_id, account_type, app_agent_id, ta_id, account_name, is_keyless_account}
// KeylessDecodeError::WrongAccountType carries `expected` + `found: AccountType` — the
// requested keyless type and the account's actual classification.

// AccountName
AccountName::{new, as_bytes, as_slice, as_str} (+ TryFrom, FromStr, Display)

// AccountId32 bytes. Both types implement AsRef<[u8; N]> *and* AsRef<[u8]>, so a bare
// `.as_ref()` cannot infer — use the inherent accessors.
AccountId32::{new, as_bytes, as_slice}

// AccountId32 hex interchange form ("0x" + 64 hex characters)
AccountId32: Display (renders lowercase) + FromStr/TryFrom<&str> (both digit cases;
Err = ParseAccountIdError) — the contract of the hex_valid/hex_invalid fixture groups

// SS58 (feature `ss58`)
AccountId32::to_ss58check(&self) -> String
AccountId32::to_ss58check_with_version(&self, format: u16) -> Result<String, InvalidSs58Format>
  // InvalidSs58Format is a rejected *argument*, not an Ss58Error (which reports on an
  // address); it carries format() and converts into Ss58Error::InvalidFormat via From.
AccountId32::from_ss58check(address: &str) -> Result<AccountId32, Ss58Error>
AccountId32::from_ss58check_expecting(address: &str, expected_format: u16) -> Result<AccountId32, Ss58ExpectingError>
  // Ss58ExpectingError separates the two failure classes: InvalidExpectedFormat (the
  // parameter is out of the 14-bit range — the decoder twin of InvalidSs58Format) and
  // Address(Ss58Error). Folds into Ss58Error via From for single-taxonomy callers.
  // full validation first, then the format comparison (the Python/TS ss58_format semantics)
AccountId32::from_ss58check_with_version(address: &str) -> Result<(AccountId32, u16), Ss58Error>
SS58_FORMAT_TREVO_ASSET_HUB: u16 = 5335
SS58_MAX_ADDRESS_LENGTH: usize = 50  // decode rejects longer input before base58

// Error types
Ss58Error (the five shared kinds) | InvalidSs58Format (rejected format argument, above)
Ss58ExpectingError (from_ss58check_expecting: InvalidExpectedFormat | Address, above)
ParseAccountIdError (hex parse) | InvalidAccountIdLength (AccountId32::TryFrom<&[u8]>)
InvalidAccountType (AccountType::TryFrom<u8>)
AccountNameError | KeylessDecodeError::WrongAccountType { expected, found }
// AccountId32 also: TryFrom<&[u8]> (Err = InvalidAccountIdLength), TryFrom<&str>/FromStr
// (Err = ParseAccountIdError), Default, From<[u8; 32]>.

// Padded test account ids (feature `test-accounts`) — AccountId64, AccountId128
// A 32-byte keyless account in the leading bytes, right-zero-padded to the full width. The
// runtime's wrapper layer reuses the 32-byte encode_*/decode_account_info through these:
AccountIdN::new([u8; N]) -> Self
AccountIdN: From<AccountId32>            // encode: zero-pad the canonical 32-byte account
AccountIdN::to_account_id32(&self) -> Option<AccountId32>  // decode: unpad iff trailing zeros
AccountIdN: From<[u8; N]> + Into<[u8; N]> + AsRef<[u8; N]> + AsRef<[u8]> + Default
AccountIdN: Display + FromStr ("0x" + 2N hex; Err = ParseAccountIdError)
// Under `scale`: Encode/Decode/DecodeWithMemTracking/TypeInfo/MaxEncodedLen (distinct metadata
// identities per width). Under `serde`: (de)serialize as the "0x"-prefixed hex string.
```
