# Trevo Keyless Address Encoding and Decoding Library

This library provides functions for encoding and decoding keyless addresses used by the Trevo blockchain:
application agent addresses, transactional addresses, and named addresses.

A keyless address is a type of blockchain address that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless addresses convenient for use in off-chain applications.

## Example

```rust
use trevo_keyless::*;

// Encode an AppAgent keyless address
let app_agent_id = 123;
let app_agent_address = encode_app_agent_account(&app_agent_id);

// Decode the keyless address
let decoded_app_agent_id = decode_app_agent_account(&app_agent_address).unwrap();
assert_eq!(decoded_app_agent_id, app_agent_id);
```
