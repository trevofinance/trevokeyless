# Interoperability and decoding compatibility

Only two representations are cross-language interchange formats:

- an SS58 address string;
- a raw 32-byte account id rendered as `0x` plus 64 lowercase hexadecimal characters.

All three packages convert in both directions. Rust returns `AccountId32`; Python and
TypeScript return their own classified information objects. Those objects are deliberately
language-specific: Rust uses an enum, Python frozen dataclasses, and TypeScript a readonly
discriminated union. Exchange the address or raw account id rather than serialized decoded
objects.

SS58 failures share the five kinds `InvalidBase58`, `InvalidLength`, `InvalidChecksum`,
`InvalidFormat`, and `UnexpectedFormat`. Shared rendered messages and typed-decoder
`(expected, found)` pairs are defined by the fixture and
[cross-language contracts](cross-language-contracts.md).

## Canonicalization and forward compatibility

Encoders emit one canonical address spelling. Decoders reject whitespace, case changes,
invalid alphabet characters, invalid checksum lengths, and oversized input. The sole
accepted non-canonical form is the two-byte SS58 prefix for formats below 64; use the raw
account id as a cache or lookup key and re-encode when a canonical address is required.

Unknown account-type discriminants classify as `Regular`. Consequently, `Regular` means
"not a keyless type known to this library version," not proof that no later Trevo version
  recognizes it. Consumers requiring exhaustive classification must keep the library current.
