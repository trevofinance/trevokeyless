# Trevo Keyless Address Encoding and Decoding Library

This library provides functions for encoding and decoding keyless addresses used by the Trevo blockchain:
application agent addresses, transactional addresses, and named addresses.

A keyless address is a type of blockchain address that does not depend on a pair of
cryptographic keys for identification.
Instead, it is derived from a combination of identifiers and checksums and is controlled by an AppAgent.
This makes the keyless addresses convenient for use in off-chain applications.

## Installation

```
npm install @trevofinance/trevo-keyless
```

The package ships a single CommonJS module with TypeScript declarations; it works from
both `require` and `import`.

## Usage

To use this library in your project, import the necessary functions and constants:

```javascript
const {
  encodeAppAgentAddress,
  encodeTransactionalAddress,
  encodeNamedAddress,
  AddressType,
  AddressTypeMismatchError,
  decodeAppAgentAddress,
  decodeTransactionalAddress,
  decodeNamedAddress,
  decodeAddress,
  decodeAccountId,
  isKeylessAddress,
} = require("@trevofinance/trevo-keyless");
```

TypeScript users can additionally import the type-only declarations
`BlockchainAddressInfo`, `DecodedTransactionalAddress`, and `DecodedNamedAddress`
(interfaces are erased at runtime, so they are not available from `require`).

Example for AppAgent addresses:

```javascript
const appAgentId = 123;
const encodedAppAgentAddress = encodeAppAgentAddress(appAgentId);
console.log("Encoded AppAgent address:", encodedAppAgentAddress);

const decodedAppAgentId = decodeAppAgentAddress(encodedAppAgentAddress);
console.log("Decoded AppAgent ID:", decodedAppAgentId);

const decodedAppAgentAddress = decodeAddress(encodedAppAgentAddress);
console.log("Decoded AppAgent address:", decodedAppAgentAddress);
```

Example for Transactional addresses

```javascript
const appAgentId = 123;
const taId = 456;
const encodedTaAddress = encodeTransactionalAddress(appAgentId, taId);
console.log("Encoded Transactional address:", encodedTaAddress);

const decodedTaId = decodeTransactionalAddress(encodedTaAddress);
console.log("Decoded Transactional ID:", decodedTaId);

const decodedTransactionalAddress = decodeAddress(encodedTaAddress);
console.log("Decoded Transactional address:", decodedTransactionalAddress);
```

Example for Named addresses

```javascript
const appAgentId = 123;
const addressName = "hot-wallet";
const encodedNamedAddress = encodeNamedAddress(appAgentId, addressName);
console.log("Encoded Named address [123, 'hot-wallet']:", encodedNamedAddress);

const decodedAddressName = decodeNamedAddress(encodedNamedAddress);
console.log("Decoded address name:", decodedAddressName);

const decodedNamedAddress = decodeAddress(encodedNamedAddress);
console.log("Decoded Named address:", decodedNamedAddress);
```

Example for Regular addresses (Alice's well-known development account)

```javascript
const encodedRegularAddress =
  "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";
const decodedRegularAddress = decodeAddress(encodedRegularAddress);
console.log("Decoded Regular address:", decodedRegularAddress);
```

Check whether an address is keyless

```javascript
const regularAddress = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";
console.log(isKeylessAddress(encodeAppAgentAddress(123))); // true
console.log(isKeylessAddress(regularAddress)); // false
```

## Accepted input and errors

The decoding functions accept an SS58 address string of a 32-byte account id in the
expected SS58 format (5335, the Trevo asset hub, by default). A raw account id in hex
form (`0x` followed by exactly 64 hex characters) is classified with `decodeAccountId`
instead; raw account bytes carry no SS58 format, so that function takes no `ss58Format`
argument.

Any other input — wrong SS58 format or checksum, account ids that are not exactly
32 bytes, malformed base58 or hex, surrounding whitespace — throws an `Error`; only the
canonical spelling of an address is accepted. The typed decoders
(`decodeAppAgentAddress`, `decodeTransactionalAddress`, `decodeNamedAddress`) throw
`AddressTypeMismatchError` (carrying the `expected` and `found` address types) when the
address decodes to a different type.

The `ss58Format` argument of every function must be an integer in `[0, 16383]` and not
a reserved format (46, 47); in particular `-1` is rejected rather than passed to the
underlying decoder, where it would act as an "accept any format" sentinel.
