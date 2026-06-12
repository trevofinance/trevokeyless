# Trevo Keyless Account Encoding and Decoding Library

Encoding and decoding for Trevo keyless accounts: App Agent root accounts, transactional accounts,
and named accounts.

A **keyless account** is a 32-byte on-chain account identifier whose private key is deliberately
unknown. Its bytes are deterministically derived from a set of logical identifiers (AppAgentId and
optionally a TransactionalId or AccountName), so anyone can compute the address from the
identifiers and read back, by inspecting the bytes alone, which App Agent controls the account
under the Trevo chain's rules.

Because no private key exists, keyless accounts cannot sign transactions. Instead, an App Agent
acts on behalf of its keyless accounts by submitting App Transactions on Trevo blockchain.

**Classification is not authorization.** The keyless checksum involves no key material and no
secret of any kind — *anyone* can construct a valid keyless account embedding *any* AppAgentId,
for free. Never grant privileges (allow-listing, fee waivers, webhook trust, ...) to an account
merely because it decodes to a particular AppAgentId. See the
[security model](https://github.com/trevo-finance/trevo-keyless#security-model) for the full
contract.

## Byte layout

```text
AppAgent root account:   [0..=3] AppAgentId | [4] type=1 | [5..=31]  checksum
Transactional account:   [0..=3] AppAgentId | [4] type=2 | [5..=8] TransactionalId | [9..=31]  checksum
Named account:           [0..=3] AppAgentId | [4] type=3 | [5..=14] AccountName     | [15..=31] checksum
```

The checksum bytes are `blake2_256(open_part)[N..]` where N is the length of the open part.

## Example

```rust
use trevo_keyless::*;

// Encode an AppAgent root account
let app_agent_id: AppAgentId = 123;
let account = encode_app_agent_account(app_agent_id);

// Decode it back
let decoded_id = decode_app_agent_account(&account).unwrap();
assert_eq!(decoded_id, app_agent_id);

// SS58 address encoding (feature `ss58`)
let address = account.to_ss58check();
assert_eq!(address, "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp");
assert_eq!(AccountId32::from_ss58check(&address).unwrap(), account);
```

## Cargo features

- `std` (default) — implements `std::error::Error` for the error types and enables the `std`
  features of the dependencies. Disable for `no_std` builds.
- `ss58` — SS58 address encoding/decoding for `AccountId32` (`to_ss58check*` /
  `from_ss58check*`) with the Trevo format `SS58_FORMAT_TREVO_ASSET_HUB` (5335). Works in
  `no_std` environments but requires an allocator.
- `scale` — SCALE codec and metadata support
  (`Encode`/`Decode`/`DecodeWithMemTracking`/`TypeInfo`/`MaxEncodedLen` for every public
  type), for direct use inside a Substrate runtime.
- `serde` — serde support (implies `ss58`): `AccountId32` (de)serializes as an SS58 address
  string in the Trevo format, `AccountName` as a string. Deserializing `BlockchainAccountInfo`
  validates that the embedded identifiers match the account id. Its JSON shape is the Rust
  enum representation and is not interchangeable with the info objects of the Python and
  TypeScript libraries — SS58 address strings are the cross-language interchange format.

## Design notes

- **Accounts, not addresses.** Unlike the Python and TypeScript trevo-keyless libraries,
  whose entry points take and return SS58 address *strings*, this crate separates the two
  layers: `encode_*_account` returns the raw 32-byte `AccountId32`, and rendering it as an
  address is an explicit `to_ss58check*` call. The crate's primary consumer is the chain
  runtime, which works in account ids; address strings are an off-chain presentation.
- **SCALE support is opt-in.** The `scale` feature derives
  `Encode`/`Decode`/`DecodeWithMemTracking`/`TypeInfo`/`MaxEncodedLen` for every public
  type, so a Substrate runtime can use them directly; consumers that only encode, decode,
  and classify accounts never compile `parity-scale-codec`, `scale-info`, or their
  proc-macro stack. Note that SCALE `Decode` of `BlockchainAccountInfo` performs no
  consistency validation — see the type documentation.
- **Hex rendering.** `AccountId32`'s `Display` prints the cross-language raw interchange
  form `0x` + 64 lowercase hex characters — the exact string the Python
  `decode_account_id` and TypeScript `decodeAccountId` entry points accept.
