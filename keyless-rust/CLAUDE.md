# CLAUDE.md — trevo-keyless

Pure-Rust library for encoding and decoding Trevo keyless accounts. No Substrate/frame dependency;
only `parity-scale-codec`, `scale-info`, and `blake2b_simd`.

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

## AccountName

10-byte fixed array. Allowed characters: `0-9 a-z A-Z - #` (64 total).
`AccountName::new(&[u8])` validates length (exactly 10) and character set.

Well-known names used by the Trevo chain:
- `"hot-wallet"` — the default hot wallet of an App Agent.
- Names starting with `"cold"` — cold wallets (4-byte prefix, remaining 6 bytes distinguish them).

## File layout

```
src/
  lib.rs           — crate-level docs, module declarations
  types.rs         — AppAgentId, TransactionalId, AccountId32, AccountName, BlockchainAccountInfo
  keyless.rs       — encode_account, encode_*, decode_*, is_keyless_account
  tests_binary.rs  — byte-level structure tests (hardcoded expected byte values)
  tests_encoding.rs — encode→decode round-trip tests
```

## Public API

```rust
// Encoding (AppAgentId and TransactionalId taken by value; AccountName by reference)
encode_app_agent_account(app_agent_id: AppAgentId) -> AccountId32
encode_transactional_account(app_agent_id: AppAgentId, ta_id: TransactionalId) -> AccountId32
encode_named_account(app_agent_id: AppAgentId, name: &AccountName) -> AccountId32

// Decoding (all take &AccountId32)
decode_account_info(account: &AccountId32) -> BlockchainAccountInfo  // classifies any account
decode_app_agent_account(account: &AccountId32) -> Result<AppAgentId, KeylessDecodeError>
decode_transactional_account(account: &AccountId32) -> Result<(AppAgentId, TransactionalId), KeylessDecodeError>
decode_named_account(account: &AccountId32) -> Result<(AppAgentId, AccountName), KeylessDecodeError>
is_keyless_account(account: &AccountId32) -> bool
```
