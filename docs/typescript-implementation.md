# TypeScript implementation

The package reproduces Rust behavior for Node, browser, and React Native with only the
audited runtime dependencies `@noble/hashes` and `@scure/base`.

- `src/keyless.ts` is the public module.
- `src/ss58.ts` is internal except for explicitly re-exported contracts.
- `tests/public-surface.test.ts` and `tests/type-safety.ts` pin runtime and type surfaces.
- `tests/adversarial.test.ts` is paired with Python host-boundary coverage.
- `tests/cross-realm.test.ts` pins error branding across supported module/realm shapes.
- `scripts/run-readme.mjs` executes README examples from the packed artifact.
- `scripts/check-platforms.mjs` verifies supported platform outputs.

Keep source syntax erasable and preserve native ESM, CommonJS, browser, and React Native
conditions. Retain the `sideEffects` declarations needed for cross-realm error branding.
Use UTF-8 byte length for account names, reject non-primitive strings, and keep raw account
IDs separate from SS58 addresses. New public API starts in Rust and updates runtime/type
surface pins, executable README, conformance where observable, version, and changelog.
The bundled Hermes binaries target x64; another architecture supplies `HERMES_BINARY`.
