---
paths:
  - keyless-rust/src/ss58.rs
  - keyless-rust/src/types.rs
  - keyless-python/trevokeyless/_ss58.py
  - keyless-python/trevokeyless/keyless.py
  - keyless-ts/src/ss58.ts
  - keyless-ts/src/keyless.ts
---

# Shared error wording and taxonomy

- The SS58 error messages (the Rust `Display` output) and the account-name validation
  messages ("Account name must be exactly 10 characters long" / "Account name contains
  invalid characters") are shared VERBATIM by all three implementations. The conformance
  fixture pins them in its `message` fields, generated from Rust — never reword in one
  language. A rewording is a three-implementation change plus a fixture regeneration and
  test updates, in one PR.
- The five `Ss58Error` kinds (InvalidBase58 | InvalidLength | InvalidChecksum |
  InvalidFormat | UnexpectedFormat) are a pinned cross-language taxonomy; adding a kind
  is a breaking cross-language change.
- Validation order is contractual: in the decoders and `encode_address`/`encodeAddress`,
  the format parameter is checked before the address/account-id payload, and full
  address validation precedes the expected-format comparison; the three keyless encoders
  check identifier/name arguments before ss58 format, in declaration order. Pinned by
  the both-invalid ordering tests in keyless-python/tests/test_adversarial.py and
  keyless-ts/tests/adversarial.test.ts.
- Account names are validated by UTF-8 BYTE length (exactly 10) — never code-point count
  (Python `len`) or UTF-16 units (JS `String.length`).
