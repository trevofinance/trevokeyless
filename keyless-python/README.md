# Trevo Keyless Address Encoding and Decoding Library

This library provides functions for encoding and decoding keyless addresses used by the Trevo blockchain:
application agent addresses, transactional addresses, and named addresses.

A keyless address is a type of blockchain address that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless addresses convenient for use in off-chain applications.

**Classification is not authorization.** The keyless checksum involves no key material and
no secret of any kind — anyone can construct a valid keyless address embedding any
AppAgent ID, for free. Never grant privileges (allow-listing, fee waivers, webhook trust,
and the like) to an address merely because it decodes to a particular AppAgent ID. See the
[security model](https://github.com/trevo-finance/trevokeyless#security-model) for the full
contract.

## Byte layout

A keyless account id is 32 bytes: an open part carrying the identifiers, followed by a
checksum that fills the remainder.

```text
AppAgent root account:   [0..=3] AppAgentId | [4] type=1 | [5..=31]  checksum
Transactional account:   [0..=3] AppAgentId | [4] type=2 | [5..=8] TransactionalId | [9..=31]  checksum
Named account:           [0..=3] AppAgentId | [4] type=3 | [5..=14] AccountName     | [15..=31] checksum
```

The identifiers are stored little-endian, and the checksum bytes are
`blake2_256(open_part)[N..]`, where `N` is the length of the open part.

Decoding recomputes the checksum for the discriminant at byte `[4]`; any mismatch, an
unknown discriminant, or out-of-charset Named name bytes classify the account as
`Regular`. This is a forward-compatibility rule with a real upgrade obligation: any byte
`[4]` value this library version does not recognize classifies as `Regular`, so `Regular`
means "not a keyless account that this library version knows", not "provably not keyless".
A keyless account type added in a newer version therefore reads as `Regular` on an older
library. Keep the library current before relying on a `Regular` classification.

## Installation

You can install the library using pip:

```shell
pip install trevokeyless
```

## Usage

### Encode and Decode AppAgent Addresses

``` python3
import trevokeyless

app_agent_id = trevokeyless.AppAgentId(123)

# Encode an AppAgent address
encoded_address = trevokeyless.encode_app_agent_address(app_agent_id)
assert encoded_address == "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"

# Decode the AppAgent address
decoded_app_agent_id = trevokeyless.decode_app_agent_address(encoded_address)
assert decoded_app_agent_id == 123
```

### Encode and Decode Transactional Addresses

``` python3
import trevokeyless

app_agent_id = trevokeyless.AppAgentId(123)
transactional_address_id = trevokeyless.TransactionalId(456)

# Encode a Transactional address
encoded_address = trevokeyless.encode_transactional_address(
    app_agent_id=app_agent_id,
    ta_id=transactional_address_id,
)
assert encoded_address == "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG"

# Decode the Transactional address
decoded_data = trevokeyless.decode_transactional_address(encoded_address)
assert decoded_data == (123, 456)
```

### Encode and Decode Named Addresses

``` python3
import trevokeyless

app_agent_id = trevokeyless.AppAgentId(123)
# AccountName validates on construction (10 UTF-8 bytes from 0-9 a-z A-Z - #), just like
# AppAgentId — a bad name fails here, not inside the encoder.
account_name = trevokeyless.AccountName("hot-wallet")

# Encode a Named address
encoded_address = trevokeyless.encode_named_address(
    app_agent_id=app_agent_id,
    account_name=account_name,
)
assert encoded_address == "ttowKp8Ams1q53N3APEt8PQi8hJ57WjQ92KQTtJrY574nomqv"

# Decode the Named address
decoded_data = trevokeyless.decode_named_address(encoded_address)
assert decoded_data == (123, "hot-wallet")
```

Account names are case-sensitive: `Treasury-1` and `treasury-1` are two different accounts
of the same App Agent, with different account ids and addresses. Never case-fold or trim a
name before encoding, comparing, or looking one up. This matters wherever a name drives
payment routing or an access decision.

### Decode any address

``` python3
import trevokeyless

app_agent_id = trevokeyless.AppAgentId(123)

# Encode an AppAgent address
encoded_address = trevokeyless.encode_app_agent_address(app_agent_id)
assert encoded_address == "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"

# Decode the address and classify it
decoded_data = trevokeyless.decode_address(encoded_address)
expected_data = trevokeyless.AppAgentAccountInfo(
    address=encoded_address,
    account_id="0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2",
    app_agent_id=trevokeyless.AppAgentId(123),
)
assert decoded_data == expected_data
```

### Decode a raw account id

``` python3
import trevokeyless

# Raw account bytes carry no SS58 format, so decode_account_id takes none.
info = trevokeyless.decode_account_id("0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2")
assert info.account_type is trevokeyless.AccountType.AppAgent
assert isinstance(info, trevokeyless.AppAgentAccountInfo)
assert info.app_agent_id == 123
```

### Decode an address of unknown SS58 format

``` python3
import trevokeyless

# An address of a foreign (non-Trevo) network: the same account id under SS58
# format 42. decode_address would raise Ss58FormatMismatchError for it;
# decode_address_with_format returns the format found in the address instead.
info, ss58_format = trevokeyless.decode_address_with_format(
    "5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7"
)
assert ss58_format == 42
assert info.account_type is trevokeyless.AccountType.AppAgent
assert isinstance(info, trevokeyless.AppAgentAccountInfo)
assert info.app_agent_id == 123

# Prefer decode_address whenever the expected format is known: accepting any
# format silently admits addresses of other networks.
```

### Encode a raw account id as an address

``` python3
import trevokeyless

# The reverse direction: render an account id (e.g. from chain state) as its
# canonical SS58 address, in the Trevo format by default.
address = trevokeyless.encode_address("0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2")
assert address == "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"
```

### Encode identifiers straight to a raw account id

``` python3
import trevokeyless

# The account-id counterparts of the encode_* functions return the raw 0x + 64 hex
# interchange form directly, without going through an SS58 address.
account_id = trevokeyless.encode_app_agent_account_id(trevokeyless.AppAgentId(123))
assert account_id == "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2"

# The transactional and named identifiers have the same counterpart.
transactional_account_id = trevokeyless.encode_transactional_account_id(
    app_agent_id=trevokeyless.AppAgentId(123),
    ta_id=trevokeyless.TransactionalId(7),
)
assert transactional_account_id == "0x7b00000002070000002e8d5c5d2e5deb6e449c8ee5b098bb459957db4b467634"
assert (
    trevokeyless.encode_named_account_id(
        app_agent_id=trevokeyless.AppAgentId(123),
        account_name=trevokeyless.AccountName("hot-wallet"),
    )
    == "0x7b00000003686f742d77616c6c65746f13294d746d30576c61b409e943f757a4"
)

# Each typed address decoder has an account-id counterpart taking the same raw form.
assert trevokeyless.decode_app_agent_account_id(account_id) == 123
assert trevokeyless.decode_transactional_account_id(transactional_account_id) == (123, 7)

# is_keyless_account_id classifies a raw account id (is_keyless_address takes an SS58
# address); it is total over well-formed account ids.
assert trevokeyless.is_keyless_account_id(account_id)
assert not trevokeyless.is_keyless_account_id("0x" + "11" * 32)
```

### Check whether an address is keyless

``` python3
import trevokeyless

assert trevokeyless.is_keyless_address(
    trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123))
)

# Alice's well-known development account is a regular address
assert not trevokeyless.is_keyless_address("ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4")
```

### Validate identifiers and names without exceptions

`is_app_agent_id`, `is_transactional_id`, and `is_account_name` are the non-throwing
counterparts of the validating constructors — the twins of the TypeScript
`isAppAgentId` / `isTransactionalId` / `isAccountName` type guards. Each returns `True`
exactly when the constructor would accept the value, and narrows it for a type checker
(`typing.TypeGuard`), so a guarded value flows into the typed encoders without a cast.

``` python3
import trevokeyless

assert trevokeyless.is_app_agent_id(123)
assert not trevokeyless.is_app_agent_id(-1)
assert trevokeyless.is_transactional_id(trevokeyless.U32_MAX)
assert not trevokeyless.is_transactional_id(2**32)
assert trevokeyless.is_account_name("Treasury-1")
assert not trevokeyless.is_account_name("too-short")   # 9 UTF-8 bytes, not 10
assert not trevokeyless.is_account_name("invalid@#!")  # 10 bytes, but '@' and '!' are outside the charset

user_input = 123
if trevokeyless.is_app_agent_id(user_input):
    address = trevokeyless.encode_app_agent_address(user_input)
```

### Handle decoding errors

When you decode input you do not control, `decode_address` raises rather than returning a
sentinel. `Ss58Error` covers every decoding failure and carries a machine-readable `kind`;
its `Ss58FormatMismatchError` subclass is the `"UnexpectedFormat"` case, raised only for a
fully valid SS58 address of a different network, and it additionally carries `expected` and
`found` formats. Because the subclass is caught first, a foreign-network address is
distinguished from malformed input.

``` python3
import trevokeyless

# A valid address of a foreign network (SS58 format 42, not Trevo's 5335) is reported as a
# format mismatch, never silently accepted.
try:
    trevokeyless.decode_address("5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7")
    raise AssertionError("a foreign-format address must not decode")
except trevokeyless.Ss58FormatMismatchError as mismatch:
    assert mismatch.kind == "UnexpectedFormat"
    assert mismatch.expected == trevokeyless.SS58_FORMAT_TREVO_ASSET_HUB
    assert mismatch.found == 42

# Any other malformed input raises the base Ss58Error, whose kind names the failure.
# Catch the subclass first, since Ss58FormatMismatchError is an Ss58Error.
try:
    trevokeyless.decode_address("this is not an address")
    raise AssertionError("malformed input must not decode")
except trevokeyless.Ss58FormatMismatchError as mismatch:
    raise AssertionError("malformed input is not a format mismatch") from mismatch
except trevokeyless.Ss58Error as error:
    assert error.kind == "InvalidBase58"
```

### Exported constants

``` python3
import trevokeyless

# The shape of the wire format, exported under the same names by all three
# implementations.
assert trevokeyless.ACCOUNT_ID_LENGTH == 32       # bytes in an account id
assert trevokeyless.ACCOUNT_NAME_LENGTH == 10     # UTF-8 bytes in a Named account name
assert trevokeyless.U32_MAX == 4294967295         # largest AppAgent / Transactional ID
assert trevokeyless.SS58_FORMAT_TREVO_ASSET_HUB == 5335  # the default SS58 format
assert trevokeyless.SS58_MAX_ADDRESS_LENGTH == 50 # longest accepted SS58 address

# The account type discriminant stored at byte [4] of a keyless account id; equal to the
# corresponding AccountType value.
assert trevokeyless.APP_AGENT_ACCOUNT_IDENTIFIER == trevokeyless.AccountType.AppAgent.value == 1
assert trevokeyless.TRANSACTIONAL_ACCOUNT_IDENTIFIER == trevokeyless.AccountType.Transactional.value == 2
assert trevokeyless.NAMED_ACCOUNT_IDENTIFIER == trevokeyless.AccountType.Named.value == 3
assert trevokeyless.AccountType.Regular.value == 0
```

### Exported types

Every name in `trevokeyless.__all__` is public API. Besides the functions and constants above:

| Name | What it is |
|---|---|
| `AppAgentId`, `TransactionalId` | validating `int` subclasses; a bad identifier fails where it is written, not inside an encoder |
| `AccountName` | a validating `str` subclass: exactly 10 UTF-8 bytes from `0-9 a-z A-Z - #` |
| `AccountType` | the `Regular \| AppAgent \| Transactional \| Named` enum (values `0..3`) |
| `BlockchainAccountInfo` | the union of the four decoded-info dataclasses below |
| `RegularAccountInfo`, `AppAgentAccountInfo`, `TransactionalAccountInfo`, `NamedAccountInfo` | frozen dataclasses; each carries only the identifiers its variant actually has |
| `DecodedAddressWithFormat` | the `(info, ss58_format)` pair returned by `decode_address_with_format` |
| `DecodedTransactionalAddress`, `DecodedNamedAddress` | the named tuples the typed decoders return |
| `Ss58Error`, `Ss58FormatMismatchError`, `AccountNameError`, `AccountTypeMismatchError`, `ParseAccountIdError`, `NotAnAddressError` | the error taxonomy (see below) |
| `Ss58ErrorKind`, `AccountNameErrorKind` | the `Literal` unions of the cross-language `kind` values |
| `BlockchainAddress`, `BlockchainAccountId`, `SS58Format` | readability aliases for `str`, `str`, `int` |

## Accepted input and errors

`AppAgentId` and `TransactionalId` are distinct, validating `int` subclasses: the
constructor rejects anything that is not an integer in the u32 range `[0, 4294967295]`
(`bool` included), so a bad identifier fails where it is written rather than deep inside
an encoder. They remain usable wherever an `int` is (`AppAgentId(1) + 1 == 2`). Use them
at typed call sites so a type checker also catches accidentally transposed IDs; encoders
with multiple logical identifiers are keyword-only for the same reason. Every
`ss58_format` argument is keyword-only, across encoders and decoders alike.

`decode_address` (and `is_keyless_address` and the typed decoders) accept an SS58
address string of a 32-byte account id in the expected SS58 format (5335, the Trevo
asset hub, by default). `decode_address_with_format` accepts an address of *any*
(non-reserved) format and returns the format found in the address alongside the
classification — the entry point for addresses whose format is not known up front
(prefer `decode_address` otherwise). A raw account id (`0x` followed by exactly 64 hex
characters) is classified with `decode_account_id` instead; raw account bytes carry no
SS58 format, so that function takes no `ss58_format` argument, and `decode_address`
(like `decode_address_with_format`) rejects `0x` input outright. Each typed address
decoder has an account-id counterpart on the same raw form
(`decode_app_agent_account_id`, `decode_transactional_account_id`,
`decode_named_account_id`), as does `is_keyless_address` (`is_keyless_account_id`).
`encode_address` is the reverse direction: it takes the same raw account id form and
renders the SS58 address for the given format. `BlockchainAccountInfo` is a union of four frozen,
slotted, keyword-only dataclasses: `RegularAccountInfo`, `AppAgentAccountInfo`,
`TransactionalAccountInfo`, and `NamedAccountInfo`. Each variant contains only the
identifiers valid for that classification; checking `isinstance` or `account_type`
determines which fields exist. These dataclasses are decoder **output values**, not
validation tokens: do not trust a caller-constructed or deserialized instance as proof
of a classification; decode its original address or account ID instead. The `address`
field holds the SS58 address for `decode_address` and is `None` for
`decode_account_id` (raw account bytes carry no address; render one with
`encode_address`); `account_id` is normalized `0x` + lowercase hex. `address` is
the input spelling verbatim — returned exactly as you passed it, canonical or not, never
re-encoded or normalized: for the one accepted non-canonical spelling (the
two-byte-prefix encoding of formats below 64, see below) it differs from the canonical
address of the same account — key caches and lookups on `account_id`, and render the
canonical address with `encode_address`.

Only the canonical spelling of an address is accepted: wrong-length checksums,
surrounding whitespace, case variations, characters outside the base58 alphabet, and
account ids that are not exactly 32 bytes all raise `ValueError`. (One deliberate,
Substrate-compatible exception: the two-byte-prefix encoding of formats below 64 is
accepted on decode.) `is_keyless_address` likewise raises — rather than returning
`False` — when the input cannot be decoded at all.

Five `ValueError` subclasses carry structured details:

- `Ss58Error` for every SS58 decoding failure: its `kind` attribute mirrors the
  variants of the Rust `Ss58Error` enum (`"InvalidBase58" | "InvalidLength" |
  "InvalidChecksum" | "InvalidFormat" | "UnexpectedFormat"`). The `"UnexpectedFormat"`
  case is raised as the subclass `Ss58FormatMismatchError` (`expected`/`found`
  attributes) — raised only when the address is a fully valid SS58 string of a
  different format; an invalid address always raises its validation kind instead.
- `AccountTypeMismatchError` (`expected`/`found` attributes) from the typed decoders
  (`decode_app_agent_address`, `decode_transactional_address`, `decode_named_address`
  and their `*_account_id` counterparts) when the input decodes to a different type.
- `AccountNameError` from the Named encoders, whose `kind` is `"InvalidLength"` or
  `"InvalidCharacter"`.
- `ParseAccountIdError` from the raw-account-id entry points (`decode_account_id`,
  `encode_address`, `is_keyless_account_id`, `decode_*_account_id`) when the input is
  not `0x` followed by exactly 64 hex characters. Mirrors the Rust
  `ParseAccountIdError`.
- `NotAnAddressError` from the SS58 address entry points (`decode_address`,
  `decode_address_with_format`, `is_keyless_address`, `decode_*_address`) when given a
  raw `0x` account id — the guidance to use the account-id entry point instead.

The encoding functions validate their inputs: AppAgent and Transactional IDs must be
integers in the u32 range `[0, 4294967295]` (matching the `u32` identifiers of the
on-chain encoding), and account names must be exactly `ACCOUNT_NAME_LENGTH` (10) **UTF-8
bytes** from the set `0-9 a-z A-Z - #`. The length rule is the *byte* length, exactly as
in the Rust implementation, which validates the raw name bytes; every allowed character
is one-byte ASCII, so the accepted set is unchanged, but the rule decides which of the
two `AccountNameError` kinds a rejected non-ASCII name gets: `"héllo12345"` (10
characters, 11 bytes) is an `"InvalidLength"`, while `"café12345"` (9 characters, 10
bytes) is an `"InvalidCharacter"`. Every `ss58_format` argument must be an integer in `[0, 16383]`.
Formats 46 and 47 are valid expected-format inputs to the decoding functions: because
no valid address can carry either reserved format, decoding will report either the
address's validation error or `Ss58FormatMismatchError`. The encoding functions reject
46 and 47 with the same `ValueError` as an out-of-range format — an unusable format is
a rejected *argument*, never an `Ss58Error`, which reports on an *address*. Wrong types
raise `TypeError`: a
non-string where an address, account id, or name is expected, and a non-integer
(`bool` included) where an ID or SS58 format is expected. Out-of-range values and other
invalid input raise `ValueError`.

## License

Licensed under the [Apache License, Version 2.0](./LICENSE). Copyright 2025-2026 Trevo Ltd.
