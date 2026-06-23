# Conformance fixture schema (`vectors.json`)

`vectors.json` is the executable cross-language specification of the Trevo keyless
encoding. It is **generated from the Rust crate** (the source of truth) by
`keyless-rust/examples/gen_conformance.rs` and consumed by the Rust, Python, and
TypeScript test suites. CI regenerates it and diffs against the committed file, so it
cannot drift from the implementation.

Regenerate after an intentional change to the encoding semantics, and review the diff:

```
cd keyless-rust
cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json
```

A change to any *value* field (`hex`, `ss58`, `account_id`, `expected`, …) of an
existing vector is a breaking change of the on-chain format. Adding vectors or groups is
the normal way to pin newly specified behavior.

This document describes the JSON shape of each group so a fourth implementation (Go,
Java, …) can be validated against the same fixture. Types below: `u32` is an integer in
`[0, 4294967295]`, `u16` in `[0, 16383]`, `hex64` is 64 lowercase hex characters (no
`0x`), `ss58` is a base58 address string.

## Interchange entry points by language

The fixture is phrased in terms of these per-language entry points:

| Operation | Rust | Python | TypeScript |
|---|---|---|---|
| encode account id → address | `AccountId32::to_ss58check_with_version(fmt)` | `encode_address(hex, fmt)` | `encodeAddress(hex, fmt)` |
| decode address → account id + classify | `from_ss58check_expecting` + `decode_account_info` | `decode_address(addr, fmt)` | `decodeAddress(addr, fmt)` |
| classify raw account id | `decode_account_info` | `decode_account_id(hex)` | `decodeAccountId(hex)` |
| parse raw account id string | `AccountId32::from_str` | `decode_account_id` | `decodeAccountId` |

`type` values are `Regular | AppAgent | Transactional | Named` (numeric `0 | 1 | 2 | 3`).
`error_kind` values are the `Ss58Error` variant names
`InvalidBase58 | InvalidLength | InvalidChecksum | InvalidFormat | UnexpectedFormat`.

## Groups

### `app_agent`
Encoding vectors for App Agent root accounts.
`{ "id": u32, "hex": hex64, "ss58": ss58 }` — encoding `id` yields account bytes `hex`;
the Trevo-format (5335) address of those bytes is `ss58`. Decoding `hex` classifies as
`AppAgent` with `app_agent_id == id`.

### `transactional`
`{ "aa": u32, "ta": u32, "hex": hex64, "ss58": ss58 }` — as above for Transactional
accounts; decoding yields `app_agent_id == aa`, `ta_id == ta`.

### `named`
`{ "aa": u32, "name": string, "hex": hex64, "ss58": ss58 }` — `name` is exactly 10
characters from `0-9 a-z A-Z - #`; decoding yields `app_agent_id == aa`, `name`.

### `classify`
`{ "hex": hex64, "expected": { "type": Type, "app_agent_id"?: u32, "ta_id"?: u32, "name"?: string } }`
Classifying the raw account bytes `hex` must produce exactly `expected`. Optional fields
are present only for the corresponding type (`app_agent_id` for AppAgent/Transactional/
Named, `ta_id` for Transactional, `name` for Named). Covers arbitrary bytes, every
single-byte corruption of each keyless type (all demote to `Regular`), out-of-charset
Named name bytes, and cross-discriminant rewrites.

### `ss58_formats`
`{ "hex": hex64, "format": u16, "ss58": ss58 }` — encoding `hex` with `format` yields
`ss58`; decoding `ss58` returns the same bytes and `format`. Covers single-byte (`<64`)
and two-byte (`>=64`) prefixes, the reserved-format neighbors 45/48, and 0/16383.

### `ss58_decode_only`
`{ "ss58": ss58, "hex": hex64, "format": u16, "reason": string }` — non-canonical
spellings that every implementation must **accept on decode** (yielding `hex` and
`format`) while its encoder keeps producing the canonical form (so re-encoding `hex` at
`format` does **not** reproduce `ss58`). The documented Substrate-compatible malleability
exception: the two-byte-prefix encoding of formats below 64.

### `ss58_invalid`
`{ "ss58": string, "format": u16, "error_kind": Kind, "expected"?: u16, "found"?: u16, "reason": string }`
Decoding `ss58` **with the expected `format`** must fail, classified as `error_kind`.
`UnexpectedFormat` vectors additionally carry `expected`/`found` (the requested and the
embedded format). Covers wrong-length checksums, reserved formats, prefix-tag tampering,
corrupted checksums, surrounding whitespace, out-of-alphabet characters, and the
UTF-8-byte length bound.

### `named_invalid_encode`
`{ "name": string, "reason": string }` — names every implementation must reject at
**encode** time (wrong length or out-of-charset). Note `"héllo12345"` is 10 code points
but 11 UTF-8 bytes: Rust reports a length error, Python/TypeScript a charset error — all
three reject it (rejection is pinned, the error category is per-language).

### `hex_invalid`
`{ "input": string, "reason": string }` — raw account id strings that violate the
`0x` + 64-hex contract; `decode_account_id` / `encode_address` (and the Rust `FromStr`)
must reject them.

### `hex_valid`
`{ "input": string, "account_id": string, "expected": <classify expected> }` — mixed-case
spellings the entry points must **accept**; `account_id` is the normalized `0x` +
lowercase-hex form, and `expected` is the classification (same shape as `classify`).

### `hex_not_an_address`
`{ "input": string, "reason": string }` — `0x`-prefixed raw account id strings fed to the
SS58 *address* entry points (`decode_address` / `is_keyless_address`), which must reject
them (raw account ids carry no SS58 format; use `decode_account_id`). Rejection is pinned;
the error classification is per-language.

### `type_mismatch`
`{ "hex": hex64, "ss58": ss58, "decode_as": Type, "found": Type }` — decoding the account
with the typed decoder for `decode_as` (always a keyless type different from `found`) must
fail carrying the pair `(expected = decode_as, found)`: Rust
`KeylessDecodeError::WrongAccountType`, Python `AccountTypeMismatchError`, TypeScript
`AccountTypeMismatchError` (Python/TS take the `ss58` address; Rust takes the `hex` bytes).
The structured `(expected, found)` pair is the cross-language contract; the rendered error
*message* is per-language. Covers every ordered pair of keyless types plus each decoder on
a `Regular` account.
