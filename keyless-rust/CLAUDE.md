# CLAUDE.md — trevo-keyless

Pure-Rust library for encoding and decoding Trevo keyless accounts. No Substrate/frame dependency;
only `parity-scale-codec`, `scale-info`, and `blake2b_simd` (plus optional `bs58` and `serde`,
see Cargo features below).

## Cargo features

- `std` (default) — `std::error::Error` impls and `std` features of the dependencies. Disable
  for `no_std` builds (the crate builds for `wasm32v1-none`).
- `ss58` — SS58 address encoding/decoding for `AccountId32` (`to_ss58check*`,
  `from_ss58check*`, `SS58_FORMAT_TREVO_ASSET_HUB = 5335`). `no_std`-compatible but requires
  an allocator.
- `serde` — serde support (implies `ss58`): `AccountId32` (de)serializes as an SS58 string in
  the Trevo format, `AccountName` as a string, other public types via derived impls.
  Deserializing `BlockchainAccountInfo` re-validates that the embedded identifiers match
  `account_id` (SCALE `Decode` performs no such validation). Its serde shape is the externally
  tagged Rust enum — not interchangeable with the Python/TS info objects.

## What is a keyless account?

A **keyless account** is a 32-byte on-chain identifier whose private key is deliberately unknown.
Its bytes are derived deterministically from a set of logical identifiers, so:
- Anyone can compute the address from the identifiers (no key generation).
- Anyone can identify the controlling App Agent by inspecting the bytes.

Because no private key exists, keyless accounts cannot sign transactions. In the Trevo chain, an
App Agent acts on behalf of its keyless accounts by submitting **App Transactions**.

## Three account types

| Type | Constant (byte [4]) | Identifiers embedded |
|------|---------------------|----------------------|
| App Agent root | `APP_AGENT_ACCOUNT_IDENTIFIER = 1` | `AppAgentId` |
| Transactional  | `TRANSACTIONAL_ACCOUNT_IDENTIFIER = 2` | `AppAgentId`, `TransactionalId` |
| Named          | `NAMED_ACCOUNT_IDENTIFIER = 3` | `AppAgentId`, `AccountName` (10 bytes) |

## Encoding scheme

Every keyless account is 32 bytes, divided into an **open part** (plain-text identifiers) and a
**checksum** (trailing bytes of a Blake2-256 hash):

```
final_account[0..N]    = open_part              (N = open part length)
final_account[N..32]   = blake2_256(open_part)[N..32]
```

Concretely:

```
AppAgent root (N=5):
  [0..=3]   AppAgentId          (little-endian u32)
  [4]       type discriminant = 1
  [5..=31]  blake2_256(open_part)[5..=31]

Transactional (N=9):
  [0..=3]   AppAgentId          (little-endian u32)
  [4]       type discriminant = 2
  [5..=8]   TransactionalId     (little-endian u32)
  [9..=31]  blake2_256(open_part)[9..=31]

Named (N=15):
  [0..=3]   AppAgentId          (little-endian u32)
  [4]       type discriminant = 3
  [5..=14]  AccountName         (10 bytes, charset: 0-9 a-z A-Z - #)
  [15..=31] blake2_256(open_part)[15..=31]
```

Implementation: `encode_account(open_part)` computes `hash = blake2_256(open_part)` (32 bytes)
then overwrites `hash[0..N]` with the open part, leaving `hash[N..]` as the stored checksum.

## Decoding / validation

`decode_account_info(account: &AccountId32)` reads byte [4] as the type discriminant, reads the
open part for that type, recomputes the expected checksum, and compares it to the stored bytes. If
the checksum matches, the account is decoded into the appropriate `BlockchainAccountInfo` variant.
Any mismatch or unknown discriminant → `BlockchainAccountInfo::Regular`.

For Named accounts the name bytes must additionally pass charset validation. Anyone can construct
an account that carries a valid checksum over out-of-charset name bytes (no private key is
needed); such an account decodes as `Regular`. The Python and TypeScript implementations apply
the same rule.

## AccountName

10-byte fixed array. Allowed characters: `0-9 a-z A-Z - #` (64 total).
`AccountName::new(&[u8])` validates length (exactly 10) and character set; invalid input returns
`AccountNameError::{InvalidLength, InvalidCharacter}`.

## SS58 (feature `ss58`)

`AccountId32::to_ss58check()` / `from_ss58check(&str)` use the Trevo format
(`SS58_FORMAT_TREVO_ASSET_HUB = 5335`); the `*_with_version` variants take/return an explicit
format. Decoding failures return `Ss58Error`.

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
vectors derived from this crate — the source of truth — plus three rejection groups:
`ss58_invalid` (address strings every implementation must reject: non-canonical shapes
such as one-byte checksums, reserved formats, corrupted checksums, surrounding
whitespace), `named_invalid_encode` (names every implementation must reject at encode
time), and `hex_invalid` (raw account id strings violating the "0x" + 64 hex contract
of the Python `decode_account_id` / TypeScript `decodeAccountId` entry points; this
crate has no string form of an account id, so its suite skips that group). The Rust,
Python, and TypeScript test suites all verify against the committed fixture, and CI
checks that the fixture matches the generator output. After an intentional change to the encoding semantics,
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
AccountName::{new, as_bytes, as_str, starts_with} (+ TryFrom, FromStr, Display)

// SS58 (feature `ss58`)
AccountId32::to_ss58check(&self) -> String
AccountId32::to_ss58check_with_version(&self, format: u16) -> Result<String, Ss58Error>
AccountId32::from_ss58check(address: &str) -> Result<AccountId32, Ss58Error>
AccountId32::from_ss58check_with_version(address: &str) -> Result<(AccountId32, u16), Ss58Error>
SS58_FORMAT_TREVO_ASSET_HUB: u16 = 5335
SS58_MAX_ADDRESS_LENGTH: usize = 50  // decode rejects longer input before base58
```
