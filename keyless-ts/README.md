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
  Ss58Error,
  decodeAppAgentAddress,
  decodeTransactionalAddress,
  decodeNamedAddress,
  decodeAddress,
  decodeAccountId,
  encodeAddress,
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

Decode a raw account id (raw account bytes carry no SS58 format, so `decodeAccountId`
takes none)

```javascript
const info = decodeAccountId(
  "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2",
);
console.log(info.addressType === AddressType.AppAgent); // true
console.log(info.appAgentId); // 123
```

Encode a raw account id as an address (the reverse direction: render an account id,
e.g. from chain state, as its canonical SS58 address, in the Trevo format by default)

```javascript
const address = encodeAddress(
  "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2",
);
console.log(address); // ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp
```

Check whether an address is keyless

```javascript
const regularAddress = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";
console.log(isKeylessAddress(encodeAppAgentAddress(123))); // true
console.log(isKeylessAddress(regularAddress)); // false
```

Note: an address that is _keyless_ is not an address that is _authorized_ — anyone can
construct a valid keyless address embedding any AppAgent ID. See the
[security model](https://github.com/trevo-finance/trevo-keyless#security-model).

## Accepted input and errors

`decodeAddress` (and `isKeylessAddress` and the typed decoders) accept an SS58 address
string of a 32-byte account id in the expected SS58 format (5335, the Trevo asset hub,
by default). A raw account id (`0x` followed by exactly 64 hex characters) is
classified with `decodeAccountId` instead; raw account bytes carry no SS58 format, so
that function takes no `ss58Format` argument, and `decodeAddress` rejects `0x` input
outright. `encodeAddress` is the reverse direction: it takes the same raw account id
form and renders the SS58 address for the given format. The `address` field of the
returned `BlockchainAddressInfo` always holds the input verbatim; `accountId` is
normalized `0x` + lowercase hex.

Only the canonical spelling of an address is accepted: wrong-length checksums,
surrounding whitespace, case variations, characters outside the base58 alphabet, and
account ids that are not exactly 32 bytes all throw. (One deliberate,
Substrate-compatible exception: the two-byte-prefix encoding of formats below 64 is
accepted on decode.) `isKeylessAddress` likewise throws — rather than returning
`false` — when the input cannot be decoded at all.

Two error classes carry structured details:

- `Ss58Error` for every SS58 decoding failure: its `kind` mirrors the variants of the
  Rust `Ss58Error` enum (`"InvalidBase58" | "InvalidLength" | "InvalidChecksum" |
"InvalidFormat" | "UnexpectedFormat"`), and for `"UnexpectedFormat"` the `expected`
  and `found` fields carry the two formats;
- `AddressTypeMismatchError` (carrying the `expected` and `found` address types) from
  the typed decoders (`decodeAppAgentAddress`, `decodeTransactionalAddress`,
  `decodeNamedAddress`) when the address decodes to a different type.

The `ss58Format` argument of every function must be an integer in `[0, 16383]` and not
a reserved format (46, 47); in particular `-1` is rejected rather than treated as an
"accept any format" sentinel.
