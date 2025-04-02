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

# Encode a Named address
encoded_address = trevokeyless.encode_app_agent_address(app_agent_id)
assert encoded_address == "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"

# Decode the Named address
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
