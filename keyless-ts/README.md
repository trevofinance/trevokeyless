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
[security model](https://github.com/trevo-finance/trevokeyless/blob/main/docs/security-model.md) for the full
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

```
npm install @trevofinance/trevokeyless
```

Node consumers require Node.js `>=22.13.0`. CI tests that exact floor and Node 24; this
also covers the runtime's unflagged CommonJS loading of ESM-only dependencies.

The package ships native ESM and CommonJS entry points plus explicit `browser` and
`react-native` export conditions. CI bundles both conditions from the packed npm
artifact, executes the browser build in the current Playwright-managed Chromium, and
executes the React Native build in Hermes. The resolver checks also run at both supported
Node versions, and Windows CI builds and inspects the package independently. Browser
output targets ES2022 and does not require a global `TextEncoder`.

## Usage

To use this library in your project, import the necessary functions and constants:

```javascript
const {
  accountName,
  appAgentId,
  transactionalId,
  isAccountName,
  isAppAgentId,
  isTransactionalId,
  encodeAppAgentAddress,
  encodeTransactionalAddress,
  encodeNamedAddress,
  AccountType,
  AccountNameError,
  AccountTypeMismatchError,
  NotAnAddressError,
  ParseAccountIdError,
  Ss58Error,
  Ss58FormatMismatchError,
  decodeAppAgentAddress,
  decodeTransactionalAddress,
  decodeNamedAddress,
  decodeAddress,
  decodeAddressWithFormat,
  decodeAccountId,
  decodeAppAgentAccountId,
  decodeTransactionalAccountId,
  decodeNamedAccountId,
  encodeAddress,
  encodeAppAgentAccountId,
  encodeTransactionalAccountId,
  encodeNamedAccountId,
  isKeylessAddress,
  isKeylessAccountId,
  ACCOUNT_ID_LENGTH,
  ACCOUNT_NAME_LENGTH,
  APP_AGENT_ACCOUNT_IDENTIFIER,
  TRANSACTIONAL_ACCOUNT_IDENTIFIER,
  NAMED_ACCOUNT_IDENTIFIER,
  SS58_FORMAT_TREVO_ASSET_HUB,
  SS58_MAX_ADDRESS_LENGTH,
  U32_MAX,
} = require("@trevofinance/trevokeyless");
```

The exported constants are `ACCOUNT_ID_LENGTH` (32) and `ACCOUNT_NAME_LENGTH` (10),
the account-id byte-`[4]` discriminants `APP_AGENT_ACCOUNT_IDENTIFIER` (1),
`TRANSACTIONAL_ACCOUNT_IDENTIFIER` (2) and `NAMED_ACCOUNT_IDENTIFIER` (3) — equal to the
matching `AccountType` values — plus `SS58_FORMAT_TREVO_ASSET_HUB` (5335),
`SS58_MAX_ADDRESS_LENGTH` (50), and `U32_MAX`.

TypeScript users can additionally import the type-only declarations `AccountName`,
`AppAgentId`, `TransactionalId`, `AppAgentAddressOptions`, `AppAgentAccountIdentifiers`,
`TransactionalAddressOptions`,
`TransactionalAccountIdentifiers`, `NamedAddressOptions`, `NamedAccountIdentifiers`,
`BlockchainAccountInfo` and its `RegularAccountInfo`, `AppAgentAccountInfo`,
`TransactionalAccountInfo`, and `NamedAccountInfo` variants,
`DecodedAddressWithFormat`, `DecodedTransactionalAddress`, `DecodedNamedAddress`,
`Ss58ErrorKind`, and `AccountNameErrorKind` (types are erased at runtime, so they are
not available from `require`), plus the readability aliases `BlockchainAddress` /
`BlockchainAccountId` / `Ss58Format` (plain `string` / `string` / `number` — the
Python package exports the same aliases). The branded types prevent App Agent and Transactional
identifiers from being transposed, and keep an unvalidated account name out of the
encoders: `appAgentId()`, `transactionalId()` and `accountName()` validate a value once
and brand it. Their non-throwing counterparts `isAppAgentId()`, `isTransactionalId()`
and `isAccountName()` answer whether a value would validate — narrowing it to the
branded type on success — for code that asks instead of catching:

```javascript
const candidate = 123; // e.g. parsed from user input
if (!isAppAgentId(candidate)) {
  throw new RangeError(`not a valid AppAgent id: ${candidate}`);
}
// candidate now satisfies AppAgentId — usable without a throwing factory call.
console.log(encodeAppAgentAccountId({ appAgentId: candidate }));

console.log(isTransactionalId(-1)); // false: outside the u32 range
console.log(isAccountName("hot-wallet")); // true
console.log(isAccountName("Hot Wallet")); // false: the space is outside the charset
```

Example for AppAgent addresses:

```javascript
const aaId = appAgentId(123);
const encodedAppAgentAddress = encodeAppAgentAddress({ appAgentId: aaId });
console.log("Encoded AppAgent address:", encodedAppAgentAddress);

const decodedAppAgentId = decodeAppAgentAddress(encodedAppAgentAddress);
console.log("Decoded AppAgent ID:", decodedAppAgentId);

const decodedAppAgentAddress = decodeAddress(encodedAppAgentAddress);
console.log("Decoded AppAgent address:", decodedAppAgentAddress);
```

Example for Transactional addresses

```javascript
const aaId = appAgentId(123);
const txId = transactionalId(456);
const encodedTaAddress = encodeTransactionalAddress({
  appAgentId: aaId,
  taId: txId,
});
console.log("Encoded Transactional address:", encodedTaAddress);

const decodedTaId = decodeTransactionalAddress(encodedTaAddress);
console.log("Decoded Transactional ID:", decodedTaId);

const decodedTransactionalAddress = decodeAddress(encodedTaAddress);
console.log("Decoded Transactional address:", decodedTransactionalAddress);
```

Example for Named addresses

```javascript
const aaId = appAgentId(123);
const name = accountName("hot-wallet");
const encodedNamedAddress = encodeNamedAddress({
  appAgentId: aaId,
  accountName: name,
});
console.log("Encoded Named address [123, 'hot-wallet']:", encodedNamedAddress);

const decodedAddressName = decodeNamedAddress(encodedNamedAddress);
console.log("Decoded address name:", decodedAddressName);

const decodedNamedAddress = decodeAddress(encodedNamedAddress);
console.log("Decoded Named address:", decodedNamedAddress);
```

Account names are case-sensitive: `Treasury-1` and `treasury-1` are two different accounts
of the same App Agent, with different account ids and addresses. Never case-fold or trim a
name before encoding, comparing, or looking one up. This matters wherever a name drives
payment routing or an access decision.

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
console.log(info.accountType === AccountType.AppAgent); // true
console.log(info.appAgentId); // 123
```

Decode an address of unknown SS58 format (an address of a foreign network:
`decodeAddress` would throw `UnexpectedFormat` for it; `decodeAddressWithFormat`
returns the format found in the address instead — prefer `decodeAddress` whenever the
expected format is known, since accepting any format silently admits addresses of
other networks)

```javascript
const { info, ss58Format } = decodeAddressWithFormat(
  "5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7",
);
console.log(ss58Format); // 42
console.log(info.accountType === AccountType.AppAgent); // true
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

Encode identifiers straight to a raw account id (the account-id counterparts of the
`encode*Address` functions return the raw `0x` + 64 hex interchange form directly,
without going through an SS58 address)

```javascript
const accountId = encodeAppAgentAccountId({ appAgentId: appAgentId(123) });
console.log(accountId); // 0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2

// The transactional and named identifiers have the same counterpart.
console.log(
  encodeTransactionalAccountId({
    appAgentId: appAgentId(123),
    taId: transactionalId(7),
  }),
); // 0x7b00000002070000002e8d5c5d2e5deb6e449c8ee5b098bb459957db4b467634
console.log(
  encodeNamedAccountId({
    appAgentId: appAgentId(123),
    accountName: accountName("hot-wallet"),
  }),
); // 0x7b00000003686f742d77616c6c65746f13294d746d30576c61b409e943f757a4

// The typed decoders have account-id counterparts too: they take the raw account id
// (which carries no SS58 format) and throw AccountTypeMismatchError on a mismatch.
console.log(decodeAppAgentAccountId(accountId)); // 123
console.log(
  decodeTransactionalAccountId(
    encodeTransactionalAccountId({
      appAgentId: appAgentId(123),
      taId: transactionalId(7),
    }),
  ),
); // { appAgentId: 123, taId: 7 }
console.log(
  decodeNamedAccountId(
    encodeNamedAccountId({
      appAgentId: appAgentId(123),
      accountName: accountName("hot-wallet"),
    }),
  ),
); // { appAgentId: 123, accountName: 'hot-wallet' }

// isKeylessAccountId classifies a raw account id (isKeylessAddress takes an SS58
// address); it is total over well-formed account ids.
console.log(isKeylessAccountId(accountId)); // true
console.log(isKeylessAccountId(`0x${"11".repeat(32)}`)); // false
```

Check whether an address is keyless

```javascript
const regularAddress = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";
console.log(
  isKeylessAddress(encodeAppAgentAddress({ appAgentId: appAgentId(123) })),
); // true
console.log(isKeylessAddress(regularAddress)); // false
```

When you decode input you do not control, `decodeAddress` throws rather than returning a
sentinel. `Ss58Error` covers every decoding failure and carries a machine-readable `kind`;
its `Ss58FormatMismatchError` subclass is the `"UnexpectedFormat"` case, thrown only for a
fully valid SS58 address of a different network, and it additionally carries `expected` and
`found` formats. Because the subclass is checked first with `instanceof`, a foreign-network
address is distinguished from malformed input.

```javascript
const assert = require("node:assert");

// A valid address of a foreign network (SS58 format 42, not Trevo's 5335) is reported as
// a format mismatch, never silently accepted.
try {
  decodeAddress("5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7");
  throw new Error("a foreign-format address must not decode");
} catch (mismatch) {
  assert.ok(mismatch instanceof Ss58FormatMismatchError);
  assert.strictEqual(mismatch.kind, "UnexpectedFormat");
  assert.strictEqual(mismatch.expected, SS58_FORMAT_TREVO_ASSET_HUB);
  assert.strictEqual(mismatch.found, 42);
}

// Any other malformed input throws the base Ss58Error, whose kind names the failure.
// Check the subclass first, since Ss58FormatMismatchError is an Ss58Error.
try {
  decodeAddress("this is not an address");
  throw new Error("malformed input must not decode");
} catch (error) {
  assert.ok(!(error instanceof Ss58FormatMismatchError));
  assert.ok(error instanceof Ss58Error);
  assert.strictEqual(error.kind, "InvalidBase58");
}
```

## Accepted input and errors

`decodeAddress` (and `isKeylessAddress` and the typed decoders) accept an SS58 address
string of a 32-byte account id in the expected SS58 format (5335, the Trevo asset hub,
by default). `decodeAddressWithFormat` accepts an address of _any_ (non-reserved)
format and returns the format found in the address alongside the classification — the
entry point for addresses whose format is not known up front (prefer `decodeAddress`
otherwise). A raw account id (`0x` followed by exactly 64 hex characters) is
classified with `decodeAccountId` instead; raw account bytes carry no SS58 format, so
that function takes no `ss58Format` argument, and `decodeAddress` (like
`decodeAddressWithFormat`) rejects `0x` input outright. `encodeAddress` is the reverse
direction: it takes the same raw account id form and renders the SS58 address for the
given format. `BlockchainAccountInfo` is a
readonly discriminated union with one variant per `accountType`; each variant contains
only the identifiers valid for that classification. Returned objects are frozen at
runtime. These interfaces describe decoder **output values**, not validation tokens:
do not trust a caller-constructed object as proof of a classification; decode its
original address or account ID instead. The `address` field holds the SS58 address for
`decodeAddress` and is `null` for `decodeAccountId` (raw account bytes carry no address;
render one with `encodeAddress`); `accountId` is normalized `0x` + lowercase hex.
`address` is the
input spelling verbatim — returned exactly as you passed it, canonical or not, never
re-encoded or normalized: for the one accepted non-canonical spelling (the
two-byte-prefix encoding of formats below 64, see below) it differs from the canonical
address of the same account — key caches and lookups on `accountId`, and render the
canonical address with `encodeAddress`.

Only the canonical spelling of an address is accepted: wrong-length checksums,
surrounding whitespace, case variations, characters outside the base58 alphabet, and
account ids that are not exactly 32 bytes all throw. (One deliberate,
Substrate-compatible exception: the two-byte-prefix encoding of formats below 64 is
accepted on decode.) `isKeylessAddress` likewise throws — rather than returning
`false` — when the input cannot be decoded at all.

Every failure has its own exported error class, so no caller has to match on a message:

- `Ss58Error` for every SS58 decoding failure: its `kind` mirrors the Rust enum variants
  `"InvalidBase58"`, `"InvalidLength"`, `"InvalidChecksum"`, `"InvalidFormat"`, and
  `"UnexpectedFormat"`. The `"UnexpectedFormat"` case is the `Ss58FormatMismatchError`
  subclass, whose `expected` and `found` fields carry the two formats (catching the
  `Ss58Error` base class still sees it).
- `AccountTypeMismatchError` (carrying the `expected` and `found` account types) from
  the typed decoders (`decodeAppAgentAddress`, `decodeTransactionalAddress`,
  `decodeNamedAddress` and their `decode*AccountId` counterparts) when the input decodes
  to a different type.
- `AccountNameError` from the Named encoders, whose `kind` is `"InvalidLength"` or
  `"InvalidCharacter"`. The length rule is defined on the **UTF-8 byte length** of the
  name — the ten bytes that go on the wire — exactly like the Rust implementation: a
  name must encode to exactly `ACCOUNT_NAME_LENGTH` (10) UTF-8 bytes, and those bytes
  must all be in the allowed charset (`0-9 a-z A-Z - #`). So `"héllo12345"` is an
  `"InvalidLength"` failure (11 UTF-8 bytes, even though it is 10 JavaScript characters)
  while `"café12345"` is an `"InvalidCharacter"` failure (10 bytes, but `é` is out of
  charset).
- `ParseAccountIdError` from the raw-account-id entry points (`decodeAccountId`,
  `encodeAddress`, `isKeylessAccountId`, `decode*AccountId`) when the input is not `0x`
  followed by exactly 64 hex characters.
- `NotAnAddressError` from the SS58 address entry points (`decodeAddress`,
  `decodeAddressWithFormat`, `isKeylessAddress`, `decode*Address`) when the input is a
  raw `0x` account id — use `decodeAccountId` for those.

Every `ss58Format` argument must be an integer in `[0, 16383]`; in particular `-1` is
rejected rather than treated as an "accept any format" sentinel. Formats 46 and 47 are
valid expected-format inputs to the decoding functions: because no valid address can
carry either reserved format, decoding will report either the address's validation
error or `Ss58Error` kind `"UnexpectedFormat"`. The encoding functions reject 46 and
47 with the same `RangeError` as an out-of-range format — an unusable format is a
rejected _argument_, never an `Ss58Error`, which reports on an _address_.

Parameter-range violations — an out-of-range or non-integer `ss58Format`, or an id
outside the u32 range — throw a `RangeError`, not an `Ss58Error`, so a handler that
catches only `Ss58Error` does not see parameter misuse. All string parameters require
primitive JavaScript strings at runtime; non-strings and boxed `String` objects throw a
`TypeError` instead of being coerced.

## License

Licensed under the [Apache License, Version 2.0](./LICENSE). Copyright 2025-2026 Trevo Ltd.
