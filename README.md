# Trevo Keyless Address Encoding and Decoding Library

Rust, Python, and TypeScript libraries for encoding, decoding, and classifying Trevo
AppAgent root, transactional, and named keyless accounts.

The Rust crate is the source of truth. Its generated
[`conformance/vectors.json`](conformance/vectors.json) fixture is verified by all three
implementations so their observable behavior stays aligned.

## Packages

- [Rust crate](keyless-rust/) — MSRV 1.85; published as
  [`trevokeyless`](https://crates.io/crates/trevokeyless).
- [Python package](keyless-python/) — Python 3.10+; published as
  [`trevokeyless`](https://pypi.org/project/trevokeyless/).
- [TypeScript package](keyless-ts/) — Node.js 22.13+; published as
  [`@trevofinance/trevokeyless`](https://www.npmjs.com/package/@trevofinance/trevokeyless).

Start with the [documentation index](docs/README.md), especially the
[keyless model](docs/keyless-model.md), [interoperability contract](docs/interoperability.md),
and [security model](docs/security-model.md). Contributors should read
[`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md).

Report vulnerabilities privately according to the [security policy](docs/SECURITY.md).

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Copyright 2025-2026 Trevo Ltd.
