# Keyless account model

A keyless account is a 32-byte blockchain account identifier whose private key is
deliberately unknown. Its bytes are derived from public identifiers and a checksum, and
the Trevo runtime lets the embedded App Agent control it through chain-side rules.

## Account types

- **AppAgent root** — the one account derived from an AppAgent ID alone.
- **Transactional** — one of up to 2³² numbered sub-accounts per App Agent, intended for
  programmatic fan-out such as users, payment flows, or jobs.
- **Named** — a long-lived sub-account identified by an exact ten-byte name, useful for
  roles such as a treasury, fee sink, or escrow pool.

All three types are publicly computable. Their intended controller comes from the Trevo
runtime, not from a secret in this encoding; see the [security model](security-model.md).

## Byte layout

Identifiers are little-endian. The open part is followed by the trailing bytes of
`blake2b-256(open_part)` until the 32-byte account id is full.

```text
AppAgent root:  [0..=3] AppAgentId | [4]=1 | [5..=31] checksum
Transactional:  [0..=3] AppAgentId | [4]=2 | [5..=8] TransactionalId | [9..=31] checksum
Named:          [0..=3] AppAgentId | [4]=3 | [5..=14] AccountName | [15..=31] checksum
```

Classification recomputes the checksum selected by byte `[4]`. A mismatch, unknown
discriminant, or out-of-charset Named name classifies as `Regular`.

The Rust crate documentation in `keyless-rust/src/lib.rs` is authoritative for encoding
semantics and the Rust public interface. The generated fixture is the executable
cross-language specification.

## Terms

- **App Agent**: on-chain representation of an off-chain application.
- **Keyless address**: an SS58 address for an account whose private key is deliberately
  unknown and whose control is assigned by the chain to an App Agent.
- **Address**: an SS58 string; Trevo uses format 5335.
- **Account id**: the underlying 32 public bytes.
- **Account type**: `Regular`, `AppAgent`, `Transactional`, or `Named`.
- **AppAgent ID** and **Transactional ID**: unsigned 32-bit identifiers.
- **Account name**: exactly ten UTF-8 bytes from `0-9 a-z A-Z - #`.
