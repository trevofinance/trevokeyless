# Trevo Keyless Account Encoding and Decoding Library

Encoding and decoding for Trevo keyless accounts: App Agent root accounts, transactional accounts,
and named accounts.

A **keyless account** is a 32-byte on-chain account identifier whose private key is deliberately
unknown. Its bytes are deterministically derived from a set of logical identifiers (AppAgentId and
optionally a TransactionalId or AccountName), so anyone can compute the address from the
identifiers and read back, by inspecting the bytes alone, which App Agent controls the account
under the Trevo chain's rules.

Because no private key is known, keyless accounts cannot sign transactions. Instead, an App Agent
acts on behalf of its keyless accounts by submitting App Transactions on Trevo blockchain.

**Classification is not authorization.** The keyless checksum involves no key material and no
secret of any kind — *anyone* can construct a valid keyless account embedding *any* AppAgentId,
for free. Never grant privileges (allow-listing, fee waivers, webhook trust, ...) to an account
merely because it decodes to a particular AppAgentId. See the
[security model](https://github.com/trevo-finance/trevokeyless/blob/main/docs/security-model.md) for the full
contract.

## Byte layout

```text
AppAgent root account:   [0..=3] AppAgentId | [4] type=1 | [5..=31]  checksum
Transactional account:   [0..=3] AppAgentId | [4] type=2 | [5..=8] TransactionalId | [9..=31]  checksum
Named account:           [0..=3] AppAgentId | [4] type=3 | [5..=14] AccountName     | [15..=31] checksum
```

The checksum bytes are `blake2_256(open_part)[N..]` where N is the length of the open part.

## Installation

The core account-id codec needs no features; the error types implement
`core::error::Error` unconditionally, and the default `std` feature only enables the
`std` features of the dependencies (disable it for `no_std`). Enable `ss58` for the
address methods shown below:

```toml
[dependencies]
trevokeyless = { version = "2.0.0", features = ["ss58"] }
```

## Example

```rust
use trevokeyless::*;

// Encode each of the three keyless account types.
let app_agent_id = AppAgentId::new(123);
let root = encode_app_agent_account(app_agent_id);
let transactional = encode_transactional_account(
    TransactionalAccountIdentifiers::new(app_agent_id, TransactionalId::new(7)),
);
// AccountName validates on construction: exactly 10 bytes from `0-9 a-z A-Z - #`.
// Names are case-sensitive — "Treasury-1" and "treasury-1" are different accounts.
let named = encode_named_account(
    NamedAccountIdentifiers::new(app_agent_id, AccountName::new(b"hot-wallet").unwrap()),
);

// The typed decoders return the identifiers of one expected type, or an error.
assert_eq!(decode_app_agent_account(&root).unwrap(), app_agent_id);

// decode_account_info classifies *any* account id — this is the runtime's entry point.
match decode_account_info(&transactional) {
    BlockchainAccountInfo::Transactional { app_agent_id, ta_id, .. } => {
        assert_eq!(app_agent_id, AppAgentId::new(123));
        assert_eq!(ta_id, TransactionalId::new(7));
    }
    _ => unreachable!(),
}

// An account id whose bytes match no valid keyless encoding is Regular.
let opaque = AccountId32::new([0u8; 32]);
assert!(matches!(
    decode_account_info(&opaque),
    BlockchainAccountInfo::Regular { .. },
));

// SS58 address encoding (feature `ss58`).
let address = named.to_ss58check();
assert_eq!(AccountId32::from_ss58check(&address).unwrap(), named);

// A valid address of an unexpected type decodes but fails the typed decoder.
let err = decode_app_agent_account(&transactional).unwrap_err();
assert!(matches!(
    err,
    KeylessDecodeError::WrongAccountType {
        expected: AccountType::AppAgent,
        found: AccountType::Transactional,
    },
));
```

## Cargo features

- `std` (default) — enables the `std` features of the dependencies. Disable for `no_std`
  builds. The error types implement `core::error::Error` unconditionally
  (`std::error::Error` is the same trait), so error handling works identically with and
  without this feature.
- `ss58` — SS58 address encoding/decoding for `AccountId32` (`to_ss58check*` /
  `from_ss58check*`) with the Trevo format `SS58_FORMAT_TREVO_ASSET_HUB` (5335). Works in
  `no_std` environments but requires an allocator.
- `scale` — SCALE codec and metadata support
  (`Encode`/`Decode`/`DecodeWithMemTracking`/`TypeInfo`/`MaxEncodedLen` for every public
  type), for direct use inside a Substrate runtime.
- `serde` — serde support (implies `ss58`): `AccountId32` follows the serializer's
  `is_human_readable()` — an SS58 address string in the Trevo format into human-readable
  formats (JSON, YAML, TOML, ...), the raw 32 account-id bytes into binary formats
  (bincode, postcard, MessagePack, ...). `AccountName` (de)serializes as a string.
  Deserializing `BlockchainAccountInfo` validates that the embedded identifiers match the
  account id. Its JSON shape is the Rust enum representation and is not interchangeable
  with the info objects of the Python and TypeScript libraries — use SS58 address strings
  or raw `0x` + 64-hex account ids as cross-language interchange formats.
- `test-accounts` — fixed-width `AccountId64` / `AccountId128` containers for Substrate runtime
  tests whose `AccountId` is wider than 32 bytes. A keyless `AccountId32` is stored in the leading
  bytes with canonical zero padding. No SS58 or cross-language counterpart; off by default.

## Design notes

- **Accounts, not addresses.** Unlike the Python and TypeScript trevokeyless libraries,
  whose entry points take and return SS58 address *strings*, this crate separates the two
  layers: `encode_*_account` returns the raw 32-byte `AccountId32`, and rendering it as an
  address is an explicit `to_ss58check*` call. The crate's primary consumer is the chain
  runtime, which works in account ids; address strings are an off-chain presentation.
- **SCALE support is opt-in.** The `scale` feature implements
  `Encode`/`Decode`/`DecodeWithMemTracking`/`TypeInfo`/`MaxEncodedLen` for every public
  type, so a Substrate runtime can use them directly; consumers that only encode, decode,
  and classify accounts never compile `parity-scale-codec`, `scale-info`, or their
  proc-macro stack. `BlockchainAccountInfo` encodes only its `AccountId32`; SCALE decoding
  re-derives the classification and identifiers, so inconsistent redundant claims cannot be
  decoded.
- **Hex round-trip.** `AccountId32`'s `Display` prints the cross-language raw interchange
  form `0x` + 64 lowercase hex characters, and its `FromStr` parses exactly that contract
  (both digit cases accepted) — the same strings the Python `decode_account_id` and
  TypeScript `decodeAccountId` entry points accept.

## License

Licensed under the [Apache License, Version 2.0](./LICENSE). Copyright 2025-2026 Trevo Ltd.
