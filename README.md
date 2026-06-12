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
  - Package: [NPM](https://www.npmjs.com/package/@trevofinance/trevokeyless)
- Python 
  - Sources: [GitHub](./keyless-python/)
  - Package: [PyPI](https://pypi.org/project/trevokeyless/)
- Rust
  - Sources: [GitHub](./keyless-rust/)
  - Package: [Crates.io](https://crates.io/crates/trevokeyless)

The Rust implementation is the source of truth for the encoding. The shared fixture
[`conformance/vectors.json`](./conformance/vectors.json) is generated from it
(`keyless-rust/examples/gen_conformance.rs`) and verified by the test suites of all
three libraries — covering the encoding, the classification, the accepted and rejected
SS58 spellings, and the error classification of every rejection (`error_kind`, the
Rust `Ss58Error` variant names) — so the implementations cannot drift apart unnoticed.

## Security model

**The keyless checksum is classification and integrity, not authentication.** A keyless
account id is 32 bytes: an open part (the embedded identifiers) followed by the trailing
bytes of `blake2b-256(open part)`. No key material and no secret of any kind is involved
— *anyone* can compute a valid keyless address embedding *any* AppAgent ID, for free.

What decoding guarantees:

- If an account id passes the keyless check, its embedded identifiers are read
  consistently by every implementation (pinned by the shared conformance fixture).
- If the Trevo chain's rules hold, only the App Agent with the embedded ID can operate
  that account on chain. That on-chain rule — not this library — is what "the address
  belongs to AppAgent X" means.

What decoding does **not** guarantee:

- It is *not* proof that AppAgent X created, requested, or authorized the address.
  Never grant privileges (allow-listing, fee waivers, webhook trust, ...) to an address
  merely because it decodes to a particular AppAgent ID.
- A regular account misclassifying as keyless requires its bytes to match a truncated
  blake2b-256 checksum: probability ≈ 2⁻¹³⁶ in the worst case (Named accounts,
  17 checksum bytes). Corrupting any byte of a keyless account demotes it to Regular.

Additional properties:

- **No secrets, no constant time.** Every input and output of these libraries is a
  public identifier; checksum comparisons are deliberately not constant-time because
  there is nothing to hide.
- **Canonical spellings only.** Decoders reject non-canonical address spellings
  (wrong-length checksums, surrounding whitespace, case variations, out-of-alphabet
  characters) and bound input length before base58 work, so the decoding cost of
  untrusted input stays small. One deliberate, Substrate-compatible exception: the
  two-byte-prefix encoding of SS58 formats below 64 is accepted on decode (the Trevo
  format 5335 is unaffected — its two-byte prefix *is* the canonical form).
- **Non-goals.** These libraries do not manage keys, sign anything, talk to a chain, or
  prove authorship. They encode, decode, and classify addresses.

See [SECURITY.md](./SECURITY.md) for how to report a vulnerability.

## Interchange formats

Only two representations are cross-language interchange formats:

- the SS58 address string (e.g. `ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp`);
- the raw 32-byte account id, rendered as `0x` + 64 lowercase hex characters by all
  three libraries (the Rust `AccountId32` `Display`, the Python `account_id` field,
  the TypeScript `accountId` field).

Each library converts between the two in both directions: Rust with
`AccountId32::to_ss58check*` / `from_ss58check*`, Python with `encode_address` /
`decode_address`, TypeScript with `encodeAddress` / `decodeAddress`.

The decoded info objects are *per-language* shapes: the Rust serde representation of
`BlockchainAccountInfo` is an externally tagged enum and is **not** interchangeable
with the flat Python/TypeScript `BlockchainAddressInfo` objects. Do not serialize one
language's info object and parse it in another; exchange addresses or account ids.

## Versioning and compatibility

- **The byte encoding is frozen.** The layout described above and the SS58 format 5335
  are stable; [`conformance/vectors.json`](./conformance/vectors.json) is the executable
  specification, and a change to any committed vector is a breaking change of the
  encoding itself.
- The three packages are versioned **independently** (their APIs evolve at different
  speeds), each with its own git tag scheme (`rust-vX.Y.Z`,
  `python-vX.Y.Z`, `ts-vX.Y.Z`). While the versions are below 1.0.0, minor releases may
  contain breaking API changes; the encoding itself never changes.

## Common terms

- **AppAgent**: on-chain entity that serves as a blockchain representation of an off-chain application;
- **Keyless address**: an address with unknown private key controlled by an AppAgent;
- **Address**: an SS58-encoded address string (49 characters in the Trevo SS58 format, 5335);
- **Account**: the 32-byte account identifier behind an address — an opaque public key
  or hash for regular accounts, deterministically derived bytes (no key exists) for
  keyless accounts;
- **Address type**: the classification of an address — Regular, AppAgent,
  Transactional, or Named;
- **AppAgent ID**: an identifier of an AppAgent `uint32`;
- **TA ID**: an identifier of a Transactional address, `uint32`;
- **Address name**: a name of a Named address, 10 ASCII characters from the list `0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#`;
