# Trevo Keyless Account Encoding and Decoding Library

This library provides functions for encoding and decoding keyless accounts used by the Trevo blockchain:
application agent accounts, transactional accounts, and named accounts.

A keyless account is a type of blockchain account that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless accounts convenient for use in off-chain applications.

## Example

```rust
use trevo_keyless::*;

// Encode an AppAgent keyless account
let app_agent_id = 123;
let app_agent_account = encode_app_agent_account(&app_agent_id);

// Decode the keyless account
let decoded_app_agent_id = decode_app_agent_account(&app_agent_account).unwrap();
assert_eq!(decoded_app_agent_id, app_agent_id);
```
