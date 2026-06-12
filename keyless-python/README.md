# Trevo Keyless Address Encoding and Decoding Library

This library provides functions for encoding and decoding keyless addresses used by the Trevo blockchain:
application agent addresses, transactional addresses, and named addresses.

A keyless address is a type of blockchain address that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless addresses convenient for use in off-chain applications.

## Installation

You can install the library using pip:

``` python3
pip install trevokeyless
```

## Usage

### Encode and Decode AppAgent Addresses

``` python3
import trevokeyless

app_agent_id = 123

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

app_agent_id = 123
transactional_address_id = 456

# Encode a Transactional address
encoded_address = trevokeyless.encode_transactional_address(app_agent_id, transactional_address_id)
assert encoded_address == "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG"

# Decode the Transactional address
decoded_data = trevokeyless.decode_transactional_address(encoded_address)
assert decoded_data == (123, 456)
```

### Encode and Decode Named Addresses

``` python3
import trevokeyless

app_agent_id = 123
address_name = "hot-wallet"

# Encode a Named address
encoded_address = trevokeyless.encode_named_address(app_agent_id, address_name)
assert encoded_address == "ttowKp8Ams1q53N3APEt8PQi8hJ57WjQ92KQTtJrY574nomqv"

# Decode the Named address
decoded_data = trevokeyless.decode_named_address(encoded_address)
assert decoded_data == (123, "hot-wallet")
```

### Decode any address

``` python3
import trevokeyless

app_agent_id = 123

# Encode an AppAgent address
encoded_address = trevokeyless.encode_app_agent_address(app_agent_id)
assert encoded_address == "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"

# Decode the address and classify it
decoded_data = trevokeyless.decode_address(encoded_address)
expected_data = trevokeyless.BlockchainAddressInfo(
    address=encoded_address,
    account_id="0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2",
    address_type=trevokeyless.AddressType.AppAgent,
    app_agent_id=123,
    ta_id=None,
    address_name=None,
)
assert decoded_data == expected_data
```

### Decode a raw account id

``` python3
import trevokeyless

# Raw account bytes carry no SS58 format, so decode_account_id takes none.
info = trevokeyless.decode_account_id("0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2")
assert info.address_type is trevokeyless.AddressType.AppAgent
assert info.app_agent_id == 123
```

### Check whether an address is keyless

``` python3
import trevokeyless

assert trevokeyless.is_keyless_address(trevokeyless.encode_app_agent_address(123))

# Alice's well-known development account is a regular address
assert not trevokeyless.is_keyless_address("ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4")
```

Note: an address that is *keyless* is not an address that is *authorized* — anyone can
construct a valid keyless address embedding any AppAgent ID. See the
[security model](https://github.com/trevo-finance/trevo-keyless#security-model).

## Accepted input and errors

`decode_address` (and `is_keyless_address` and the typed decoders) accept an SS58
address string of a 32-byte account id in the expected SS58 format (5335, the Trevo
asset hub, by default). A raw account id (`0x` followed by exactly 64 hex characters)
is classified with `decode_account_id` instead; raw account bytes carry no SS58 format,
so that function takes no `ss58_format` argument, and `decode_address` rejects `0x`
input outright. The `address` field of the returned `BlockchainAddressInfo` always
holds the input verbatim; `account_id` is normalized `0x` + lowercase hex.

Only the canonical spelling of an address is accepted: wrong-length checksums,
surrounding whitespace, case variations, characters outside the base58 alphabet, and
account ids that are not exactly 32 bytes all raise `ValueError`. (One deliberate,
Substrate-compatible exception: the two-byte-prefix encoding of formats below 64 is
accepted on decode.) `is_keyless_address` likewise raises — rather than returning
`False` — when the input cannot be decoded at all.

Two `ValueError` subclasses carry structured details:

- `Ss58FormatMismatchError` (`expected`/`found` attributes) when the address is a valid
  SS58 string of a different format;
- `AddressTypeMismatchError` (`expected`/`found` attributes) from the typed decoders
  (`decode_app_agent_address`, `decode_transactional_address`, `decode_named_address`)
  when the address decodes to a different type.

The encoding functions validate their inputs: AppAgent and Transactional IDs must be
integers in the u32 range `[0, 4294967295]` (matching the `u32` identifiers of the
on-chain encoding), and address names must be exactly 10 characters from the set
`0-9 a-z A-Z - #`. The `ss58_format` argument of every function must be an integer in
`[0, 16383]` and not a reserved format (46, 47). Invalid input raises `ValueError`.
