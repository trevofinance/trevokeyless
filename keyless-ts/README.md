# Trevo Keyless Address Encoding and Decoding Library

This library provides functions for encoding and decoding keyless addresses used by the Trevo blockchain:
application agent addresses, transactional addresses, and named addresses.

A keyless address is a type of blockchain address that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless addresses convenient for use in off-chain applications.

## Usage

To use this library in your project, import the necessary functions and constants:

```javascript
const {
  encodeAppAgent,
  encodeTransactional,
  encodeNamed,
  AddressType,
  AddressTypeMismatchError,
  BlockchainAddressInfo,
  decodeAppAgent,
  decodeTransactional,
  decodeNamed,
  decodeAddress,
  isKeylessAddress,
} = require("@trevofinance/trevo-keyless");
```

Example for AppAgent addresses:

```javascript
const appAgentId = 123;
const encodedAppAgentAddress = encodeAppAgent(appAgentId);
console.log("Encoded AppAgent address:", encodedAppAgentAddress);

const decodedAppAgentId = decodeAppAgent(encodedAppAgentAddress);
console.log("Decoded AppAgent ID:", decodedAppAgentId);

const decodedAppAgentAddress = decodeAddress(encodedAppAgentAddress);
console.log("Decoded AppAgent address:", decodedAppAgentAddress);
```

Example for Transactional addresses

```javascript
const taId = 456;
const encodedTaAddress = encodeTransactional(appAgentId, taId);
console.log("Encoded Transactional address:", encodedTaAddress);

const decodedTaId = decodeTransactional(encodedTaAddress);
console.log("Decoded Transactional ID:", decodedTaId);

const decodedTransactionalAddress = decodeAddress(encodedTaAddress);
console.log("Decoded Transactional address:", decodedTransactionalAddress);
```

Example for Named addresses

```javascript
const addressName = "hot-wallet";
const encodedNamedAddress = encodeNamed(appAgentId, addressName);
console.log("Encoded Named address [123, 'hot-wallet']:", encodedNamedAddress);

const decodedAddressName = decodeNamed(encodedNamedAddress);
console.log("Decoded address name:", decodedAddressName);

const decodedNamedAddress = decodeAddress(encodedNamedAddress);
console.log("Decoded Named address:", decodedNamedAddress);
```

Example for Regular addresses

```javascript
const encodedRegularAddress =
  "ttowKp8AmRn9nxt8Gcout4stYQem7TZNkhpnVmtpS5KG39m2Y";
const decodedRegularAddress = decodeAddress(encodedRegularAddress);
console.log("Decoded Regular address:", decodedRegularAddress);
```

Check whether an address is keyless

```javascript
console.log(isKeylessAddress(encodeAppAgent(123))); // true
console.log(isKeylessAddress(encodedRegularAddress)); // false
```

## Accepted input and errors

The decoding functions accept an SS58 address string of a 32-byte account id in the
expected SS58 format (5335, the Trevo asset hub, by default). For backwards
compatibility, a raw account id in hex form (`0x` followed by exactly 64 hex characters)
is also accepted; raw account bytes carry no SS58 format, so the `ss58Format` argument
is not compared against such input.

Any other input — wrong SS58 format or checksum, account ids that are not exactly
32 bytes, malformed base58 or hex, surrounding whitespace — throws an `Error`; only the
canonical spelling of an address is accepted. The typed decoders
(`decodeAppAgent`, `decodeTransactional`, `decodeNamed`) throw
`AddressTypeMismatchError` (carrying the `expected` and `found` address types) when the
address decodes to a different type.

The `ss58Format` argument of every function must be an integer in `[0, 16383]` and not
a reserved format (46, 47); in particular `-1` is rejected rather than passed to the
underlying decoder, where it would act as an "accept any format" sentinel.
