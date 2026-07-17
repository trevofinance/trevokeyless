# CLAUDE.md — @trevofinance/trevokeyless (TypeScript)

TypeScript implementation of the Trevo keyless address encoding. Two audited runtime
dependencies only: `@noble/hashes` (blake2b) and `@scure/base` (base58) — do not add
more without a very good reason.

The Rust crate (`../keyless-rust/`) is the source of truth for the encoding; this
package must reproduce its behavior exactly (see `../CONTRIBUTING.md` and the shared
fixture `../conformance/vectors.json`).

## File layout

```
src/
  keyless.ts   — THE public module (package main): encode*/decode* address and
                 account-id functions, the validating factories (appAgentId,
                 transactionalId, accountName) and their non-throwing is* type-guard
                 counterparts, AccountType, BlockchainAccountInfo, error classes
                 (AccountNameError, AccountTypeMismatchError, ParseAccountIdError,
                 NotAnAddressError), the public constants
  ss58.ts      — internal strict SS58 codec (port of keyless-rust/src/ss58.rs); only
                 Ss58Error / Ss58FormatMismatchError / Ss58ErrorKind /
                 SS58_MAX_ADDRESS_LENGTH / ACCOUNT_ID_LENGTH are re-exported
                 (utf8Length is package-internal: it also backs the UTF-8-byte
                 account-name length rule)
tests/
  conformance.test.ts    — verifies ../../conformance/vectors.json (all groups, with the
                           per-group vector counts pinned)
  public-surface.test.ts — pins the exported surface; update when API changes
  adversarial.test.ts    — malformed/hostile input
  cross-realm.test.ts    — cross-realm/dual-instantiation instanceof branding (ESM+CJS
                           loaded in one process, plus a genuine vm-context realm; see
                           defineCrossRealmBrand and isRealError)
  properties.test.ts     — randomized round-trips (deterministic inline PRNG, same
                           xorshift64* logic as the Python twin)
  encoding.test.ts / names.test.ts / ss58.test.ts
  version.test.ts        — pins package.json against package-lock.json (version, engines)
  type-safety.ts         — compile-only assertions (brands, discriminated union,
                           @ts-expect-error); checked by `tsc -p tests/tsconfig.json`
  tsconfig.json          — the compile config for the tests and type-safety.ts
scripts/
  run-readme.mjs — executes the README's javascript code blocks against the packed
                   tarball (CI); it counts the blocks — update when adding an example
  check-platforms.mjs — resolver checks plus real Chromium and Hermes execution
  check-package.mjs — enforces the exact npm archive allowlist
  clean.mjs — removes dist/ through Node's fs API (build step; Windows-safe)
  write-build-metadata.mjs — writes the per-format package.json type marker into dist/
  differential.mjs — runs encode/decode cases from a JSON file against dist/esm and
                     prints the normalized results (the repo's cross-language
                     differential check)
```

## Module format (deliberate constraints)

- Ships native ESM (`dist/esm`) and CommonJS (`dist/cjs`) entry points with explicit
  browser and React Native export conditions. CI tests `require`, `import`, and both
  non-Node conditions against the packed artifact, then executes the bundles in current
  Chromium and a pinned Hermes runtime. A separate Windows job builds and inspects the
  package and runs the Hermes bundle there too.
- **No non-erasable TS syntax** (no `enum`, no namespaces): `AccountType` is a `const`
  object plus a union type. Keep new code erasable.
- `engines`: `>=22.13.0` — the supported and exactly tested floor. It supports this
  CommonJS package's unflagged loading of ESM-only `@noble/hashes` and the current
  ESLint 10 development toolchain. CI also tests Node 24.
- `sideEffects` deliberately lists `keyless.js` and `ss58.js` instead of `false`: both
  modules install the cross-realm `instanceof` brands on their error prototypes at load
  time (`defineCrossRealmBrand`), which _is_ a module side effect. `false` is probably
  safe today — each module's effects only touch its own exports — but a bundler dropping
  a "side-effect-free" module would silently un-brand the error classes it exports, so
  the conservative annotation stays. Don't flip it for a bundle-size win without proving
  the brands survive tree-shaking in the target bundlers.

## Commands (the core CI gates)

```
npm ci
npm run format-check && npm run lint-check    # prettier; eslint over the package
npm run type-check                            # source, tests, and compile-only API assertions
npm run build && npm test                     # tsc; jest via ts-jest
npm run check-platform-resolvers              # cheap browser + React Native resolver smoke
npx playwright install chromium               # once per host
npm run check-platforms                       # real Chromium + Hermes execution
```

These are the core gates only: `check-ts.yml` runs a superset (`npm audit`, the
package-content check, ESM/CJS interop and consumer type-resolution smokes, README
execution, a Windows job) and invokes the platform checks directly
(`node scripts/check-platforms.mjs --resolver-only|--runtime`); the workflow is
authoritative.

The bundled `hermes-engine-cli` binaries target x64 hosts. On another architecture, set
`HERMES_BINARY` to a compatible Hermes executable before `npm run check-platforms`.

## Conventions

- **Error taxonomy** (documented in the README, asserted by tests): parameter misuse
  (out-of-range/non-integer id or ss58Format, or a reserved ss58Format handed to an
  _encoder_) → `RangeError`; address-string validation
  → `Ss58Error` with `.kind` (the five cross-language kinds; `"UnexpectedFormat"` is the
  `Ss58FormatMismatchError` subclass carrying `.expected`/`.found`); typed-decoder
  mismatch → `AccountTypeMismatchError` with `(expected, found)`; a malformed raw account
  id → `ParseAccountIdError`; a raw `0x` account id passed to an address entry point →
  `NotAnAddressError`; account-name validation → `AccountNameError` with `.kind`;
  non-primitive string arguments are explicitly rejected with `TypeError` rather than
  coerced. Tests assert the concrete class _and_ the `.kind` — never a bare
  `expect(...).toThrow()`, which would also pass on a leaked `RangeError`.
- **Account names are validated by UTF-8 byte length** (`utf8Length`, not
  `String.length`), like the Rust `AccountName::new(&[u8])`: 10 bytes → charset check,
  otherwise `AccountNameError("InvalidLength")`. The `named_invalid_encode` fixture group
  pins the kind of every rejected name.
- Validation order is contractual and mirrored across languages. In the **decoders and
  `encodeAddress`**, the `ss58Format` parameter is checked before the address/account-id
  payload (a caller parameter is a programmer error; the address or account id is data,
  often untrusted — report the caller's own bug first), and full address validation
  (structure, length, checksum) precedes the expected-format comparison. The three
  **keyless encoders** check their identifier/name options _before_ `ss58Format`, in
  declaration order (the open part is built before `encodeKeylessAccount` examines the
  format) — matching Python, and pinned by the both-invalid ordering tests in
  `adversarial.test.ts` / `test_adversarial.py`.
- The name-validation and SS58 error messages are shared verbatim with Rust/Python —
  change them everywhere at once (see root CLAUDE.md).
- Raw `0x` account ids are rejected by the SS58 address entry points (`decodeAddress`,
  `decodeAddressWithFormat`); `decodeAccountId` / `encodeAddress` are the
  raw-account-id entry points.
- New public API needs: Rust-first design, the export list + `public-surface.test.ts`
  updated, README import list + example (executed in CI), conformance vectors when
  externally visible, and a semantic version update via the root release-metadata
  helper (major for breaking changes).
