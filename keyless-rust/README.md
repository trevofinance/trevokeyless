# Trevo Keyless Account Encoding and Decoding Library

Encoding and decoding for Trevo keyless accounts: App Agent root accounts, transactional accounts,
and named accounts.

A **keyless account** is a 32-byte on-chain account identifier whose private key is deliberately
unknown. Its bytes are deterministically derived from a set of logical identifiers (AppAgentId and
optionally a TransactionalId or AccountName), so anyone can compute the address from the
identifiers and verify account ownership by inspecting the bytes alone.

Because no private key exists, keyless accounts cannot sign transactions. Instead, an App Agent
acts on behalf of its keyless accounts by submitting Clearing Transactions.

## Byte layout

```
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
let account = encode_app_agent_account(&app_agent_id);

// Decode it back
let decoded_id = decode_app_agent_account(account).unwrap();
assert_eq!(decoded_id, app_agent_id);
```
