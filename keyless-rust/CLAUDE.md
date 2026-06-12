# CLAUDE.md — trevokeyless

Pure-Rust library for encoding and decoding Trevo keyless accounts. No Substrate/frame dependency;
only `blake2b_simd` (plus optional `parity-scale-codec` + `scale-info`, `bs58`, and `serde` —
see Cargo features below).

## Cargo features

- `std` (default) — `std::error::Error` impls and `std` features of the dependencies. Disable
  for `no_std` builds (the crate builds for `wasm32v1-none`).
- `ss58` — SS58 address encoding/decoding for `AccountId32` (`to_ss58check*`,
  `from_ss58check*`, `SS58_FORMAT_TREVO_ASSET_HUB = 5335`). `no_std`-compatible but requires
  an allocator.
- `scale` — SCALE codec and metadata support
  (`Encode`/`Decode`/`DecodeWithMemTracking`/`TypeInfo`/`MaxEncodedLen` for every public
  type), for direct use inside a Substrate runtime.
- `serde` — serde support (implies `ss58`): `AccountId32` (de)serializes as an SS58 string in
  the Trevo format, `AccountName` as a string, other public types via derived impls.
  Deserializing `BlockchainAccountInfo` re-validates that the embedded identifiers match
  `account_id` (SCALE `Decode` performs no such validation). Its serde shape is the externally
  tagged Rust enum — not interchangeable with the Python/TS info objects.

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
                     BlockchainAccountInfo, error types
  keyless.rs       — encode_account, encode_*, decode_*, is_keyless_account
  ss58.rs          — SS58 codec for AccountId32 + serde impls (features `ss58` / `serde`)
  tests_binary.rs  — byte-level structure tests (hardcoded expected byte values)
  tests_conformance.rs — verifies the committed cross-language fixture (see below)
  tests_encoding.rs — encode→decode round-trip tests
  tests_properties.rs — randomized property tests (deterministic inline PRNG, no extra deps)
  tests_ss58.rs    — SS58 codec tests (feature `ss58`)
  tests_serde.rs   — serde tests (feature `serde`)
examples/
  gen_conformance.rs — generates ../../conformance/vectors.json (requires feature `ss58`)
fuzz/
  fuzz_targets/ss58_decode.rs — cargo-fuzz target for the SS58 decoder (nightly; excluded
  from the published crate)
```

Contribution and release workflow (fixture regeneration rule, per-package tagging,
trusted publishing): see ../CONTRIBUTING.md.

All test modules additionally require the `std` feature; run the full suite with
`cargo test --all-features`.

## Cross-language conformance fixture

`conformance/vectors.json` (repository root) holds encoding, classification, and SS58
vectors derived from this crate — the source of truth — plus rejection and contract
groups: `ss58_invalid` (address strings every implementation must reject when decoding
with the accompanying expected format: non-canonical shapes such as one-byte checksums,
reserved formats, corrupted checksums, surrounding whitespace; each vector's
`error_kind` pins the failure classification — the Rust `Ss58Error` variant name,
matched by the `Ss58Error.kind` of the TypeScript and Python implementations),
`ss58_decode_only` (non-canonical two-byte-prefix spellings of formats below 64 that
every implementation must accept on decode while re-encoding canonically — the
documented malleability exception),
`named_invalid_encode` (names every implementation must reject at encode time),
`hex_invalid` (raw account id strings violating the "0x" + 64 hex contract of the
Rust `AccountId32` `FromStr` and the Python `decode_account_id` / TypeScript
`decodeAccountId` entry points), and
`hex_valid` (mixed-case raw account id strings those entry points must accept, with the
normalized lowercase `account_id`). The Rust, Python, and TypeScript test suites all
verify against the committed fixture, and CI checks that the fixture matches the
generator output. After an intentional change to the encoding semantics,
regenerate with:

```
cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json
```

and review the diff together with the corresponding Python/TypeScript changes.

## Public API

```rust
// Encoding (all parameters taken by value; AccountName is a 10-byte Copy type)
encode_app_agent_account(app_agent_id: AppAgentId) -> AccountId32
encode_transactional_account(app_agent_id: AppAgentId, ta_id: TransactionalId) -> AccountId32
encode_named_account(app_agent_id: AppAgentId, name: AccountName) -> AccountId32

// Decoding (all take &AccountId32)
decode_account_info(account: &AccountId32) -> BlockchainAccountInfo  // classifies any account
decode_app_agent_account(account: &AccountId32) -> Result<AppAgentId, KeylessDecodeError>
decode_transactional_account(account: &AccountId32) -> Result<(AppAgentId, TransactionalId), KeylessDecodeError>
decode_named_account(account: &AccountId32) -> Result<(AppAgentId, AccountName), KeylessDecodeError>
is_keyless_account(account: &AccountId32) -> bool

// Classification helpers
AccountType — Regular | AppAgent | Transactional | Named (mirrors the Python/TS AddressType)
BlockchainAccountInfo::{account_id, account_type, app_agent_id, ta_id, account_name, is_keyless_account}
// KeylessDecodeError variants each carry `found: AccountType` — the actual classification.

// AccountName
AccountName::{new, as_bytes, as_str} (+ TryFrom, FromStr, Display)

// AccountId32 hex interchange form ("0x" + 64 hex characters)
AccountId32: Display (renders lowercase) + FromStr/TryFrom<&str> (both digit cases;
Err = ParseAccountIdError) — the contract of the hex_valid/hex_invalid fixture groups

// SS58 (feature `ss58`)
AccountId32::to_ss58check(&self) -> String
AccountId32::to_ss58check_with_version(&self, format: u16) -> Result<String, Ss58Error>
AccountId32::from_ss58check(address: &str) -> Result<AccountId32, Ss58Error>
AccountId32::from_ss58check_with_version(address: &str) -> Result<(AccountId32, u16), Ss58Error>
SS58_FORMAT_TREVO_ASSET_HUB: u16 = 5335
SS58_MAX_ADDRESS_LENGTH: usize = 50  // decode rejects longer input before base58
```
