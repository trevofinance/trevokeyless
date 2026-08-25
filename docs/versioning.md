# Versioning and compatibility

The Rust, Python, and TypeScript packages use semantic versioning independently because
their language-level APIs evolve independently. Their tag schemes are `rust-vX.Y.Z`,
`python-vX.Y.Z`, and `ts-vX.Y.Z`.

The encoded bytes and SS58 format 5335 are frozen across package versions. Changing a
value of an existing conformance vector is a breaking change to the on-chain format and
requires explicit confirmation before implementation.

For an individual package:

- additive public API requires at least a minor version bump before release;
- breaking public API requires a major version bump;
- raising the Rust MSRV or Node engine floor, or dropping a supported Python minor, is a
  breaking change requiring a major version bump;
- adding support for a newer runtime version is not breaking.

The declared floors are authoritative in `keyless-rust/Cargo.toml`,
`keyless-python/pyproject.toml`, and `keyless-ts/package.json`; CI must test each exact
floor. The current supported ranges are Rust 1.85+, Python 3.10–3.14, and Node.js
22.13+ (with Node 24 also tested). Use [RELEASING.md](RELEASING.md) for preparation and
publishing.
