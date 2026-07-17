# Trevo Keyless Address Encoding and Decoding Library

This library provides functions for encoding and decoding keyless addresses used by the Trevo blockchain:
application agent addresses, transactional addresses, and named addresses.

A keyless address is a type of blockchain address that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless addresses convenient for use in off-chain applications.

For more detailed docs, visit the README of the specific library:

- TypeScript (requires Node.js 22.13+)
  - Sources: [GitHub](./keyless-ts/)
  - Package: [NPM](https://www.npmjs.com/package/@trevofinance/trevokeyless)
- Python (requires Python 3.10+)
  - Sources: [GitHub](./keyless-python/)
  - Package: [PyPI](https://pypi.org/project/trevokeyless/)
- Rust (MSRV 1.85)
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
  17 checksum bytes). Corrupting any byte of a keyless account demotes it to Regular with
  the same overwhelming probability — the checksum detects accidental corruption, it does
  not *prove* integrity against an attacker who can grind (nobody can: no secret is
  involved, so anyone can compute a *valid* keyless account for any AppAgent ID at will).

Additional properties:

- **No secrets, no constant time.** Every input and output of these libraries is a
  public identifier; checksum comparisons are deliberately not constant-time because
  there is nothing to hide.
- **SCALE decoding re-derives classification** (Rust, feature `scale`).
  `BlockchainAccountInfo` encodes only its `AccountId32`; decoding classifies those
  bytes again and derives the identifiers, so a SCALE payload cannot inject redundant
  or contradictory claims. This compact representation is a wire-contract change from
  the 1.x enum encoding; runtime consumers holding persisted 1.x values need the
  one-time storage migration described in the
  [Rust changelog](keyless-rust/CHANGELOG.md).
- **Canonical spellings only.** Decoders reject non-canonical address spellings
  (wrong-length checksums, surrounding whitespace, case variations, out-of-alphabet
  characters) and bound input length before base58 work, so the decoding cost of
  untrusted input stays small. One deliberate, Substrate-compatible exception: the
  two-byte-prefix encoding of SS58 formats below 64 is accepted on decode (the Trevo
  format 5335 is unaffected — its two-byte prefix *is* the canonical form).
- **Named account names are case-sensitive, exact byte strings.** The 10 name bytes are
  embedded and compared verbatim, so `Treasury-1`, `treasury-1`, and `TREASURY-1` are
  three *different* accounts of the same App Agent. Treat a name as an opaque identifier,
  not a display label: never case-fold, trim, or otherwise normalize it before encoding,
  and compare decoded names exactly. Two names that look alike to a human can be distinct
  accounts — relevant wherever a name drives payment routing or access decisions.
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
`decode_address`, TypeScript with `encodeAddress` / `decodeAddress`. Addresses whose
SS58 format is not known up front decode with Rust
`AccountId32::from_ss58check_with_version`, Python `decode_address_with_format`, or
TypeScript `decodeAddressWithFormat`, each returning the format found in the address
alongside the decoded account — the raw `AccountId32` in Rust, the classified info object in
Python and TypeScript.

The decoded info objects are *per-language* shapes: Rust uses an enum, Python a union of
four frozen dataclasses, and TypeScript a readonly discriminated union. Each variant
contains only its valid identifiers, but their serialized object shapes are not a
cross-language contract. Exchange addresses or account ids instead.

SS58 failures share five `kind`/variant names and exact rendered messages across the
three implementations. Account-name failures likewise expose length/character kinds
and the same two messages, although a particular non-ASCII JavaScript/Python string can
reach a different kind than its UTF-8 bytes do in Rust. Typed-decoder mismatches share
the requested type (`expected`) and actual classification (`found`), pinned by the
`type_mismatch` fixture group; their rendered message is not an interchange contract.

## Versioning and compatibility

- **The byte encoding is frozen.** The layout described above and the SS58 format 5335
  are stable; [`conformance/vectors.json`](./conformance/vectors.json) is the executable
  specification, and a change to any committed vector is a breaking change of the
  encoding itself.
- **Forward compatibility of classification.** Decoding treats any unknown account type
  discriminant (byte `[4]` outside 1–3) as `Regular`. If the Trevo chain ever adds a
  keyless account type, it will be additive — and library versions released before it
  will silently classify those accounts as `Regular`, not raise an error. `Regular`
  therefore means "not a keyless account *that this library version knows*"; consumers
  that need exhaustive classification must track library updates.
- The three packages are versioned **independently** (their APIs evolve at different
  speeds), each with its own git tag scheme (`rust-vX.Y.Z`, `python-vX.Y.Z`,
  `ts-vX.Y.Z`), and follow [semantic versioning](https://semver.org): breaking changes
  to a package's public API only happen in its major releases. The encoding itself
  never changes at any version.
- **Raising a runtime or toolchain floor is a breaking change.** Bumping the Rust MSRV,
  raising the Node `engines` floor, or dropping a supported Python minor removes
  environments a package previously worked in, so it lands in that package's **major**
  release — never a minor or patch. (Adding support for a *new* runtime version is not
  breaking.) The floors are: `rust-version` in `keyless-rust/Cargo.toml`, `engines` in
  `keyless-ts/package.json`, and `requires-python` in `keyless-python/pyproject.toml`;
  CI tests each package at exactly its declared floor.

## What the three keyless types are for

All three keyless types are controlled by the same App Agent; they differ in what
derives their identity, which is what to pick by:

- **AppAgent root** — the single account derived from the AppAgentId alone: the
  application's own on-chain identity, one per App Agent.
- **Transactional** — up to 2³² numbered sub-accounts per App Agent, indexed by the
  TA ID. Built for programmatic fan-out: give each user, payment flow, or job its own
  isolated on-chain account without managing a key for any of them, and reconstruct
  the account later from the two numbers alone.
- **Named** — sub-accounts labeled by a 10-character name embedded verbatim in the
  account bytes. Built for the small set of long-lived, human-recognizable roles an
  application points at repeatedly — a treasury, a fee sink, an escrow pool — where a
  stable label (`Treasury-1`, `fees#pool1`), not a counter, is the natural identity.

Anyone can *compute* any of these addresses from the identifiers (see the security
model above — classification is not authorization); which operations the Trevo runtime
actually permits each type is chain-side policy, outside these libraries.

## Common terms

- **AppAgent**: on-chain entity that serves as a blockchain representation of an off-chain application;
- **Keyless address**: an address whose private key is deliberately unknown (nobody can
  sign for it), controlled by an AppAgent;
- **Address**: an SS58-encoded address string (49 characters in the Trevo SS58 format, 5335);
- **Account**: the 32-byte account identifier behind an address — an opaque public key
  or hash for regular accounts, deterministically derived bytes (no key exists) for
  keyless accounts;
- **Account type** (`AccountType`): the classification of an account — Regular, AppAgent,
  Transactional, or Named;
- **AppAgent ID**: an identifier of an AppAgent `uint32`;
- **TA ID**: an identifier of a Transactional address, `uint32`;
- **Account name** (`AccountName`): the name embedded in a Named account, 10 ASCII characters from the list `0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#`;

## License

Licensed under the [Apache License, Version 2.0](./LICENSE). Copyright 2025-2026 Trevo Ltd.
