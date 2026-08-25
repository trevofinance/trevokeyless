# Conformance fixture schema

`conformance/vectors.json` is generated from the Rust source of truth and consumed by
the Rust, Python, and TypeScript suites. The generation and update procedure is in
[conformance.md](conformance.md).

This document defines the JSON shape of every group so another implementation can be
validated against the same fixture. Types below: `u32` is an integer in
`[0, 4294967295]`, `u16` in `[0, 65535]`, `hex64` is 64 lowercase hex characters (no
`0x`), and `ss58` is a base58 address string. A valid SS58 format is the narrower range
`[0, 16383]` excluding the reserved values 46 and 47; invalid-format vectors can contain
other `u16` values.

## Interchange entry points by language

The fixture is phrased in terms of these per-language entry points:

| Operation | Rust | Python | TypeScript |
|---|---|---|---|
| encode account id → address | `AccountId32::to_ss58check_with_version(fmt)` | `encode_address(hex, ss58_format=fmt)` | `encodeAddress(hex, fmt)` |
| decode address → account id + classify | `from_ss58check_expecting` + `decode_account_info` | `decode_address(addr, ss58_format=fmt)` | `decodeAddress(addr, fmt)` |
| decode address of unknown format → classify + format found | `from_ss58check_with_version` + `decode_account_info` | `decode_address_with_format(addr)` | `decodeAddressWithFormat(addr)` |
| classify raw account id | `decode_account_info` | `decode_account_id(hex)` | `decodeAccountId(hex)` |
| parse raw account id string | `AccountId32::from_str` | `decode_account_id` | `decodeAccountId` |

Python's `ss58_format` is **keyword-only** everywhere (encoders and decoders alike). The
TypeScript decoders and `encodeAddress` (the table above) take it positionally; the
keyless-type encoders (`encodeAppAgentAddress`, `encodeTransactionalAddress`,
`encodeNamedAddress`) take it as a member of their options object.

`type` values are `Regular | AppAgent | Transactional | Named` (numeric `0 | 1 | 2 | 3`).
In the SS58 groups (`ss58_invalid`) `error_kind` values are the
`Ss58Error` variant names
`InvalidBase58 | InvalidLength | InvalidChecksum | InvalidFormat | UnexpectedFormat`; in
`named_invalid_encode` they are the account-name validation kinds
`InvalidLength | InvalidCharacter` (see that group below).

A `message` field, where present, is the **rendered error message, shared verbatim** by all
three implementations ([cross-language-contracts.md](cross-language-contracts.md) pins
this wording). It is generated from the Rust `Display`
output, never hand-written, and every suite asserts it instead of a local literal — without
that, a one-language reword left all three suites green and silently broke the contract. It
appears wherever the classification is shared: `ss58_invalid`, `named_invalid_encode`, and
`hex_invalid`.

## Host-language boundaries

One class of hostile input can never be pinned by this fixture: **lone UTF-16
surrogates**. JSON cannot carry them and Rust strings cannot represent them, yet Python
and JavaScript host strings can be constructed with them. Their contract — a lone
surrogate counts as the three-byte replacement character and a valid surrogate pair as
one four-byte scalar when measuring UTF-8 byte length (JavaScript `TextEncoder`
semantics), for addresses and account names alike — is therefore pinned by the twin
per-language adversarial suites (`keyless-python/tests/test_adversarial.py` and
`keyless-ts/tests/adversarial.test.ts`), which mirror each other case for case, and by
the repository differential harness (`.github/scripts/check_cross_language.py`), which
compares the two host languages *directly* on lone-surrogate inputs — JSON escape
sequences can carry them between Python and JavaScript, but serde_json rejects them, so
the Rust oracle never sees those cases. Keep the twins in step when touching surrogate
handling.

A second unpinnable class is **host-number representations of integral values**. JSON
`123.0` parses to a Python `float` (rejected: wrong type) but to the JavaScript number
`123` (accepted — `123.0` *is* `123` in IEEE-754, so there is nothing left to reject);
JSON `-0` parses to an accepted Python `int` 0 but to the JavaScript double `-0`, which
the TypeScript implementation rejects (the one integral double it *can* tell apart from
its integer twin, and always the sign of a float-typed source — mirroring Python
rejecting the float `-0.0`). The same identifier or format value therefore classifies
differently depending on the host language's number model, and no shared vector can pin
either direction. Like the surrogate class, this parity is held by the twin adversarial
suites; keep the two sides mirrored when touching numeric validation.

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
Named name bytes, cross-discriminant rewrites, and valid accounts of every keyless type
(positive vectors, so the classify path is pinned independently of the encoding groups).

### `ss58_formats`
`{ "hex": hex64, "format": u16, "ss58": ss58 }` — encoding `hex` with `format` yields
`ss58`; decoding `ss58` returns the same bytes and `format`. Covers single-byte (`<64`)
and two-byte (`>=64`) prefixes, the reserved-format neighbors 45/48, and 0/16383.

### `ss58_decode_only`
`{ "ss58": ss58, "hex": hex64, "format": u16, "canonical_ss58": ss58, "reason": string }`
— non-canonical spellings that every implementation must **accept on decode** (yielding
`hex` and `format`) while its encoder produces exactly `canonical_ss58` (which differs
from `ss58`, so re-encoding does **not** reproduce the input). The documented
Substrate-compatible malleability exception: the two-byte-prefix encoding of formats
below 64.

### `ss58_invalid`
`{ "ss58": string, "format": u16, "error_kind": Kind, "expected"?: u16, "found"?: u16, "message": string, "reason": string }`
Decoding `ss58` **with the expected `format`** must fail, classified as `error_kind`.
`UnexpectedFormat` vectors additionally carry `expected`/`found` (the requested and the
embedded format). Covers wrong-length checksums, reserved formats, prefix-tag tampering,
corrupted checksums, surrounding whitespace, out-of-alphabet characters, and the
UTF-8-byte length bound.

### `named_invalid_encode`
`{ "name": string, "error_kind": "InvalidLength" | "InvalidCharacter", "message": string, "reason": string }`
— names every implementation must reject at **encode** time, *and* the kind of failure it
must report. `error_kind` is the Rust `AccountNameError` variant name, taken from the real
validation while generating; the Python and TypeScript name-validation errors must classify
identically (`ValueError` / `Error` carrying the same kind).

The rule, in all three implementations, is Rust's: **the UTF-8 byte length must be exactly
10 (`InvalidLength`), and only then are the bytes checked against the charset `0-9 a-z A-Z
- #` (`InvalidCharacter`)**. A non-ASCII name is therefore judged by its byte length —
never by its code-point count (Python `len`) or its UTF-16 code-unit count (JavaScript
`String.length`). The three counts disagree, so the group separates them pairwise:

| name | UTF-8 bytes | code points | UTF-16 units | `error_kind` |
|---|---|---|---|---|
| `"héllo12345"` | 11 | 10 | 10 | `InvalidLength` |
| `"café12345"` | 10 | 9 | 9 | `InvalidCharacter` |
| `"café..."` (NFD) | 11 | 10 | 10 | `InvalidLength` |
| `"🙂aaaaaaaa"` | 12 | 9 | 10 | `InvalidLength` |
| `"🙂aaaaaa"` | 10 | 7 | 8 | `InvalidCharacter` |
| `"ααααα"` | 10 | 5 | 5 | `InvalidCharacter` |

Every name in the group is rejected by every implementation whatever unit it measures, so
only `error_kind` catches an implementation measuring the wrong one.

### `hex_invalid`
`{ "input": string, "message": string, "reason": string }` — raw account id strings that
violate the `0x` + 64-hex contract; `decode_account_id` / `encode_address` (and the Rust
`FromStr`) must reject them, with the shared `message`.

### `hex_valid`
`{ "input": string, "account_id": string, "expected": <classify expected> }` — accepted
spellings (plain lowercase, uppercase, and alternating-case digits) the entry points
must **accept**; `account_id` is the normalized `0x` + lowercase-hex form, and
`expected` is the classification (same shape as `classify`). Covers each keyless type
*and* a Regular account: mixed-case acceptance is a property of the hex contract, not of
the keyless classification, so it must hold for accounts that classify as nothing.

### `hex_not_an_address`
`{ "input": string, "reason": string }` — `0x`-prefixed raw account id strings fed to the
SS58 *address* entry points (`decode_address` / `is_keyless_address`), which must reject
them (raw account ids carry no SS58 format; use `decode_account_id`). Rejection is pinned;
the error classification is per-language.

### `address_not_an_account_id`
`{ "input": string, "reason": string }` — SS58 address strings fed to the *raw-account-id*
entry points (`decode_account_id` / `decodeAccountId`, `encode_address`, the typed
`decode_*_account_id` decoders, and the Rust `FromStr`), the mirror image of
`hex_not_an_address`: an address is never `0x` + 64 hex characters, so the hex contract
must reject it rather than silently base58-decoding it. Rejection is pinned; each
implementation reports its own `ParseAccountIdError`.

### `format_invalid_encode`
`{ "hex": hex64, "format": u16, "reason": string }` —
SS58 formats that every implementation must reject at **encode** time, fed to the
raw-account-id encoder (`hex` is a valid account id) and to every keyless encode entry
point. Both ways a format can be unusable are covered: the reserved formats (46, 47), which
no address may carry, and the formats outside the 14-bit range.

Only the **rejection** is pinned; the classification is deliberately per-language, because an
unusable format is a rejected *argument* and each language reports a bad argument its own way:
Rust `InvalidSs58Format`, Python `ValueError`, TypeScript `RangeError`. What *is* a
cross-language requirement is that the two cases are not told apart: an implementation must
reject a reserved format the same way it rejects an out-of-range one, and in particular must
not raise its `Ss58Error` — the type that reports on an *address* — for either. (Python and
TypeScript once let the reserved formats fall through to the SS58 layer, so the same mistake
produced two different error types depending only on *how* the format was unusable, and a
caller catching `RangeError` caught 16384 but not 46.)

### `format_invalid_decode`
`{ "ss58": ss58, "format": u16, "reason": string }` — the **decode** side of the format
contract: an expected format outside the 14-bit range, against an *otherwise valid* address.
Every implementation must reject it, and the rejection must come from the parameter, not the
address (which decodes fine without an expected format). Only the rejection is cross-language —
Rust reports `Ss58ExpectingError::InvalidExpectedFormat`, Python a `ValueError`, TypeScript a
`RangeError` — so
no `error_kind` is pinned. The reserved formats 46/47 are deliberately absent from *this* group:
unlike at encode time, they are well-formed things to *expect*. No address can carry one, so the
answer is simply that the address does not match — an `UnexpectedFormat` mismatch, not a
rejected parameter. `ss58_invalid` pins that, with a vector per reserved format decoding a valid
Trevo address; a decoder must not borrow the encoder's rejection and report an invalid-format
error about a perfectly valid address.

### `format_invalid_host`
`{ "hex": hex64, "ss58": ss58, "value": number | boolean, "reason": string }` — format
values that exist only at the **host-language boundary**: JSON-representable values that
are not u16-typed (boolean `true`, negative, fractional, and beyond-u16 numbers), fed both
to the encode entry points (as the SS58 format, with `hex` a valid account id) and to the
decode entry points (as the expected format, against the otherwise valid address `ss58`).
Rust's `u16` makes these unrepresentable, so the group exists for the Python and
TypeScript runtime format validation — exactly as `u32_invalid_encode` below does for the
identifiers. Only the rejection is pinned (Python `TypeError`/`ValueError`, TypeScript
`RangeError`); it must never be an `Ss58Error`, which reports on an *address*. One
host-value class is deliberately absent: negative zero — JSON `-0` parses to an accepted
`int` 0 in Python but to a rejected `-0` double in JavaScript, so no shared vector can pin
it; the per-language adversarial suites cover it.

### `u32_invalid_encode`
`{ "value": number | boolean, "reason": string }` — identifier values every implementation must
reject at **encode** time (negative, above `u32::MAX`, non-integer, beyond the IEEE-754
exact-integer range, and boolean `true` — Python's `bool` subclasses `int` and must still be
rejected). Rust enforces the u32 range through its type system, so these are unrepresentable there
and the group exists for the Python `AppAgentId`/`TransactionalId` constructors and the
TypeScript `appAgentId()`/`transactionalId()` factories. Only the rejection is cross-language:
Python splits the failure along its own convention (`TypeError` for a wrong type, `ValueError`
for a wrong value) while TypeScript raises `RangeError` for both — so no `error_kind`.

### `constants`
`{ "name": string, "value": number }` — the exported constant *values*, read from the Rust
crate's own constants: `ACCOUNT_ID_LENGTH` (32), `ACCOUNT_NAME_LENGTH` (10), the byte-`[4]`
discriminants `APP_AGENT_ACCOUNT_IDENTIFIER` (1) / `TRANSACTIONAL_ACCOUNT_IDENTIFIER` (2) /
`NAMED_ACCOUNT_IDENTIFIER` (3), `SS58_FORMAT_TREVO_ASSET_HUB` (5335),
`SS58_MAX_ADDRESS_LENGTH` (50), and `U32_MAX` (4294967295). The behavior at the bounds is
pinned by other groups (the exact-50-byte address, the 2³²−1/2³² identifiers, format
16383); this group pins the constants themselves, which every implementation exports under
the same name — except `U32_MAX`, which Python and TypeScript export while Rust callers
use the native `u32::MAX`. Consumers should assert the *name set* matches their exports
exactly, so an added constant cannot go silently unconsumed.

### `type_mismatch`
`{ "hex": hex64, "ss58": ss58, "decode_as": Type, "found": Type, "address_message": string }` — decoding the account
with the typed decoder for `decode_as` (always a keyless type different from `found`) must
fail carrying the pair `(expected = decode_as, found)`: Rust
`KeylessDecodeError::WrongAccountType`, Python `AccountTypeMismatchError`, TypeScript
`AccountTypeMismatchError` (Python/TS take the `ss58` address; Rust takes the `hex` bytes).
The structured `(expected, found)` pair is the cross-language contract. `address_message` pins
the rendered message of the **Python and TypeScript** errors, which each document themselves as
sharing it with the other; it is the one field not read back from a Rust `Display`, because
Rust's `KeylessDecodeError` deliberately says *account* where the two address-level libraries
say *address* (see the naming contract in
[cross-language-contracts.md](cross-language-contracts.md)). Covers every ordered pair of
keyless types plus each decoder on a `Regular` account.
