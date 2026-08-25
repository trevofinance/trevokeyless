# Security model

**The keyless checksum provides classification and accidental-corruption detection, not
authentication.** No key material or secret is involved. Anyone can construct a valid
keyless account embedding any AppAgent ID.

## What decoding guarantees

- All conforming implementations read the same identifiers from an account that passes
  the keyless checks.
- Under the Trevo chain's rules, only the App Agent with the embedded identifier can
  operate that account on chain. The runtime rule—not this library—establishes control.

## What decoding does not guarantee

- It does not prove that the embedded App Agent created, requested, or authorized the
  account. Never grant privileges solely because an address decodes to a particular ID.
- The checksum is not a cryptographic proof of authorship. Anyone can calculate a valid
  checksum for chosen identifiers.

A random regular account falsely matching a known keyless layout requires a truncated
Blake2b-256 match. The worst case is a Named account with 17 checksum bytes, approximately
2⁻¹³⁶. Accidental corruption similarly demotes an account to `Regular` with overwhelming
probability.

## Additional properties

- Inputs and outputs are public identifiers; constant-time comparison is not required.
- Rust SCALE decoding re-derives `BlockchainAccountInfo` from `AccountId32`, so a payload
  cannot inject contradictory identifiers. This compact representation changed the 1.x
  wire contract; consumers with persisted 1.x values need the one-time migration described
  in the [Rust changelog](../keyless-rust/CHANGELOG.md).
- Decoders bound address length and reject malformed spellings. The deliberate exception
  is the Substrate-compatible two-byte-prefix representation of formats below 64, which
  is accepted on decode but never emitted by encoders.
- Named account names are case-sensitive exact bytes. Never trim, case-fold, or normalize
  them; visually similar names can identify different accounts.
- These libraries do not manage keys, sign transactions, contact a chain, or prove
  authorship.

See [SECURITY.md](SECURITY.md) to report a vulnerability privately.
