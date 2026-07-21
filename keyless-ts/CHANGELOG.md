# Changelog — @trevofinance/trevokeyless (TypeScript)

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the
package follows [semantic versioning](https://semver.org). The byte encoding itself is
frozen and never changes at any version.

## [2.0.0] — 2026-07-21

Everything below is the delta against 1.0.0, the previous — and only other — release of
this package on npm.

### Added

- Native ESM and CommonJS distributions with explicit browser and React Native export
  conditions; CI resolver-smoke-tests both conditions from the packed artifact without
  a global `TextEncoder`.
- `decodeAddressWithFormat` and its `DecodedAddressWithFormat` result type: decode an
  SS58 address of _any_ (non-reserved) format and return the classification together
  with the format found in the address — the counterpart of the Rust
  `AccountId32::from_ss58check_with_version` for addresses whose format is not known
  up front. Prefer `decodeAddress` whenever the expected format is known.
- `accountName()` and the branded `AccountName` type: a Named account name is now validated at
  construction, mirroring `appAgentId()` / `AppAgentId`.
- The type-only readability aliases `BlockchainAddress`, `BlockchainAccountId`, and
  `Ss58Format` (plain `string` / `string` / `number`, deliberately unbranded), used by
  the primary entry-point signatures and matching the aliases the Python package
  exports.
- `decodeAppAgentAccountId`, `decodeTransactionalAccountId` and `decodeNamedAccountId`:
  the raw-account-id counterparts of the typed address decoders (mirroring the Rust
  `decode_app_agent_account` / `decode_transactional_account` / `decode_named_account`),
  throwing `AccountTypeMismatchError` on a type mismatch.
- Exported error classes for the two failure classes that previously threw a bare
  `Error`, so a caller can discriminate them programmatically instead of matching on the
  message: `ParseAccountIdError` (input is not `0x` + exactly 64 hex characters; mirrors
  the Rust `ParseAccountIdError`) and `NotAnAddressError` (a raw `0x` account id was
  passed to an SS58 address entry point). The messages are unchanged.
- `Ss58FormatMismatchError extends Ss58Error`, the `"UnexpectedFormat"` case of the SS58
  taxonomy, carrying `expected`/`found` (the Python implementation exposes the same
  class). `Ss58Error.kind` keeps working exactly as before, and catching `Ss58Error`
  still sees it.
- `isAppAgentId`, `isTransactionalId` and `isAccountName`: non-throwing type guards
  that narrow to the branded identifier types — the query counterparts of the
  validating `appAgentId()` / `transactionalId()` / `accountName()` factories.
- Exported constants `ACCOUNT_NAME_LENGTH` (10) and the account-id byte-`[4]`
  discriminants `APP_AGENT_ACCOUNT_IDENTIFIER` (1), `TRANSACTIONAL_ACCOUNT_IDENTIFIER`
  (2) and `NAMED_ACCOUNT_IDENTIFIER` (3), under the same names as the Rust constants.
- The `exports` map exposes `./package.json`.

### Changed

- **Breaking:** Node 20 and Node 22.0–22.12 are not supported: the supported and
  exactly tested floor is Node 22.13.0 (`engines: >=22.13.0`). 1.0.0 declared `>=22`,
  which admitted 22.0–22.11 — versions on which this CommonJS package could not load
  at all: it `require()`s the ESM-only `@noble/hashes`, which works unflagged only on
  Node 22.12+. (22.12 itself loads, but sits below the exactly tested floor, which is
  also what the ESLint 10 development toolchain requires.)
- **Breaking:** the public `Ss58Error` constructor rejects `kind: "UnexpectedFormat"`
  (with a `RangeError`) unless invoked through `Ss58FormatMismatchError` or a subclass
  of it. A bare error of that kind carried no `expected`/`found` pair and failed the
  `instanceof Ss58FormatMismatchError` the documentation promises for it; the Python
  constructor already rejected the same misuse. The library itself always threw the
  subclass, so only direct construction is affected.
- **Breaking:** `AppAgentId` and `TransactionalId` are distinct branded types created
  by validated factories (`appAgentId()`, `transactionalId()`). Transactional and
  Named encoders accept object-shaped arguments, preventing positional identifier
  transposition.
- **Breaking:** decoded account info is a readonly four-variant discriminated union;
  impossible nullable-field combinations are gone, and returned values are frozen. The
  union is parameterized by whether the result carries the SS58 address it was decoded
  from: `decodeAddress` / `decodeAddressWithFormat` return
  `BlockchainAccountInfo<string>`, `decodeAccountId` returns
  `BlockchainAccountInfo<null>`, and the bare `BlockchainAccountInfo` still means
  "either".
- **Breaking:** `encodeAppAgentAddress` takes an options object,
  `encodeAppAgentAddress({ appAgentId, ss58Format })`, like the Transactional and Named
  encoders; the exported `AppAgentAddressOptions` type describes it.
- **Breaking:** `encodeAppAgentAccountId` takes an options object,
  `encodeAppAgentAccountId({ appAgentId })`, like every other encoder; the exported
  `AppAgentAccountIdentifiers` type describes it.
- **Breaking:** `encodeNamedAddress` and `encodeNamedAccountId` require an `AccountName`; build
  one with `accountName()`. Decoded names are `AccountName`. The brand is compile-time only —
  the runtime behaviour, the wire format, and JavaScript callers are unaffected.
- **Breaking:** account-name validation now measures the **UTF-8 byte length** of the
  name — the ten bytes that go on the wire — instead of its UTF-16 code-unit count,
  matching the Rust `AccountName::new`. The set of accepted names is unchanged; only the
  reported `AccountNameError.kind` changes for some non-ASCII names (e.g. `"héllo12345"`
  and `"🙂aaaaaaaa"` are now `"InvalidLength"` rather than `"InvalidCharacter"`). The
  kind of every rejected name is now pinned per vector by the conformance fixture.
- **Breaking:** the encoders validate `ss58Format` before the payload, exactly like the
  decode path: the format is a caller _parameter_ (a programmer error) while the
  identifiers and account id are _data_, so when both are invalid the caller's own bug
  is reported first. Previously `encodeAddress` validated the payload first and
  disagreed with `decodeAddress` about which of two bad arguments to report.
- **Breaking:** the encoding functions reject the reserved SS58 formats 46 and 47 with
  the same `RangeError` as any other unusable format. Previously they fell through to
  the SS58 layer and threw `Ss58Error` kind `"InvalidFormat"` — the type that reports
  on an _address_ — for what is a rejected _argument_. Decoders still accept 46 and 47
  as expected-format inputs (no valid address can carry them, so the result is the
  address's own validation error or an `"UnexpectedFormat"` mismatch).
- **Breaking:** the exported constant `NAMED_ADDRESS_LENGTH` is renamed to
  `ACCOUNT_NAME_LENGTH`: it is the length of an account _name_, not of an address.
- **Breaking:** non-string parameters (addresses, account ids, account names —
  including boxed `String` objects) are rejected with a `TypeError` instead of being
  coerced.
- Named-account validation now throws `AccountNameError` with a stable
  `InvalidLength`/`InvalidCharacter` kind.

### Fixed

- `instanceof` now works across the ESM/CommonJS boundary — and across genuine realm
  boundaries (a `vm` context, an iframe, a worker reached through a non-cloning channel). A
  dual package is instantiated twice whenever an application `import`s it while a transitive
  dependency `require`s it, so `e instanceof Ss58Error` silently returned `false` for an error
  thrown by the other copy — defeating the only error-discrimination mechanism the README
  documents. Every error prototype now carries a `Symbol.for` brand, so prototype identity is
  no longer load-bearing. The brand additionally requires a real `Error` — recognized from any
  realm via the `[[ErrorData]]` internal slot, not just `instanceof Error` — so a bare branded
  object literal does not pass; the registry key carries the package major version (two
  coexisting majors do not cross-brand each other's errors); and a consumer's own
  `class MyError extends Ss58Error` answers `instanceof` the ordinary way instead of matching
  every branded error in the process.
- The branded identifier types (`AppAgentId`, `TransactionalId`, `AccountName`) are now
  assignable across the ESM and CommonJS builds. The brands were `unique symbol`s, and each
  emitted declaration file gets its own distinct symbol, so the two builds' views of the same
  type were mutually unassignable (**TS2322**) whenever a branded value crossed a mixed
  `import`/`require` dependency graph — the type-level twin of the `instanceof` hazard above.
  The brands are now namespaced string-literal keys (`"@trevofinance/trevokeyless.AppAgentId"`
  and friends), structurally identical in both declaration files and still absent at runtime;
  the id types remain mutually non-assignable and unforgeable in practice.
- The package ships CommonJS type declarations, and `import` / `require` have their own `types`
  conditions. Previously the `types` condition pointed `require` consumers at an ESM
  declaration file, so any consumer on `module: node16`/`nodenext` was rejected with **TS1479**.
- `src/` is `isolatedModules`-safe: types are re-exported with `export type`, so a file-by-file
  transpiler (esbuild, swc, babel) no longer emits a runtime re-export of a type that does not
  exist. Enforced by `isolatedModules` in tsconfig.
- The option-object entry points reject a non-object argument with the package's own `TypeError`
  instead of a raw destructuring error.
- The root lockfile now carries the same Node engine range as the manifest:
  `>=22.13.0`.
- The build cleans `dist` through Node's filesystem API, so it works on Windows as well
  as POSIX systems.
- Development-tool vulnerabilities are overridden to patched versions and enforced by a
  full-tree audit gate covering production and development dependencies at once (a
  separate production-only audit run could never fail without the full-tree one failing
  too).
- UTF-8 input length is computed without a platform `TextEncoder`, with surrogate
  pairs and lone surrogates matching the cross-language error taxonomy.
- Negative zero is rejected wherever an identifier or `ss58Format` is expected
  (`RangeError`, message `got -0`), and the non-throwing guards return `false` for it.
  `-0` is the one integral double JavaScript can tell apart from its integer twin
  (`123.0` _is_ `123`), and it always signals a float-typed source — the Python
  implementation rejects the float `-0.0` the same way. Plain `0` is unaffected.

### Internal

- Packed browser and React Native exports now execute in real Chromium and Hermes
  runtimes in CI, in addition to resolver checks; Windows independently builds and
  inspects the package.
- The test suite pins the per-group vector counts of the conformance fixture, the
  `error_kind` of every invalid-name vector, the `AccountType` numeric values, and the
  concrete error class and `.kind` of every adversarial rejection (previously seven
  assertions accepted any throw at all).
- Added exact npm archive checks and a seeded Rust/Python/TypeScript differential
  repository check.

## [1.0.0] — 2026-07-05

### Added

- Initial stable release: keyless address encoding/decoding/classification
  (`encode*Address`, `decodeAddress`, typed `decode*Address`, `isKeylessAddress`),
  the raw account-id entry points (`encode*AccountId`, `decodeAccountId`,
  `encodeAddress`, `isKeylessAccountId`), the strict internal SS58 codec on
  `@noble/hashes` + `@scure/base`, and verification against the cross-language
  conformance fixture.
