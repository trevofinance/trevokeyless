# Trevo Keyless Address Encoding and Decoding Library

This library provides functions for encoding and decoding keyless addresses used by the Trevo blockchain:
application agent addresses, transactional addresses, and named addresses.

A keyless address is a type of blockchain address that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless addresses convenient for use in off-chain applications.

For more detailed docs, visit the README of the specific library:

- Typescript
  - Sources: [GitHub](./keyless-ts/)
  - Package: [NPM](https://www.npmjs.com/package/@trevofinance/trevo-keyless)
- Python 
  - Sources: [GitHub](./keyless-python/)
  - Package: [PyPI](https://pypi.org/project/trevokeyless/)
- Rust
  - Sources: [GitHub](./keyless-rust/)
  - Package: [Crates.io](https://crates.io/crates/trevo-keyless)

## Common terms

- **AppAgent**: on-chain entity that serves as a blockchain representation of an off-chain application;
- **Keyless address**: an address with unknown private key controlled by an AppAgent;
- **Address**: an SS58-encoded address string (49 characters in the Trevo SS58 format, 5335);
- **Account**: a 32-byte public key of an address;
- **Address type**: one of the possible address types;
- **AppAgent ID**: an identifier of an AppAgent `uint32`;
- **TA ID**: an identifier of a Transactional address, `uint32`;
- **Address name**: a name of a Named address, 10 ASCII characters from the list `0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#`;
