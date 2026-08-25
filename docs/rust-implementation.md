# Rust implementation

The Rust crate is the behavioral source of truth and has no Substrate FRAME dependency.

- `src/lib.rs` owns crate documentation and the module surface.
- `src/types.rs` owns identifiers, account types, account IDs, and errors.
- `src/keyless.rs` owns encoding, classification, and typed decoding.
- `src/ss58.rs` owns feature-gated SS58 and related serde behavior.
- `src/test_accounts.rs` owns feature-gated wide test-account wrappers.
- `examples/gen_conformance.rs` is the only shared-fixture writer.
- `examples/differential.rs` is the differential oracle.
- `fuzz/` is a separate nightly-only workspace.

Preserve `no_std`, `wasm32v1-none`, and the declared MSRV. The `serde` feature continues
to imply `ss58`; feature unification must not change human-readable wire behavior. SCALE
and serde decoding re-derive or validate embedded identifiers. Wide test account IDs keep
canonical right-zero padding and have no SS58 or cross-language counterpart. Preserve
both array and slice `AsRef` implementations and use inherent byte accessors where type
inference is ambiguous. Never hand-edit `conformance/vectors.json`.
