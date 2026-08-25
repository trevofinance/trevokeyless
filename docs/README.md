# Documentation

The root README is the short project entry point. These documents contain the canonical
repository-wide details:

- [Keyless model](keyless-model.md) — account types, byte layout, and terminology.
- [Interoperability](interoperability.md) — cross-language representations and decoding
  compatibility.
- [Security model](security-model.md) — guarantees, non-guarantees, and safe use.
- [Versioning](versioning.md) — independent package versions and compatibility policy.
- [Contributing](CONTRIBUTING.md) — contribution and review workflow.
- [Cross-language contracts](cross-language-contracts.md) — observable behavior shared by
  Rust, Python, and TypeScript.
- [Development](development.md) — local gates, verification selection, and CI overview.
- [AI configuration architecture](ai-configuration.md) — provider-neutral ownership,
  adapter boundaries, and validation.
- [AI agent guide](AI-AGENT-GUIDE.md), [configuration policy](AI-CONFIG.md), and
  [repository decisions](DECISIONS.md).
- Package implementation ownership for [Rust](rust-implementation.md),
  [Python](python-implementation.md), and [TypeScript](typescript-implementation.md).
- [Generated files](generated-files.md) — writers, lockfiles, and executable READMEs.
- [Fuzzing](fuzzing.md) — Rust fuzz targets, corpora, and reproduction.
- [Conformance fixture](conformance.md) and [schema](conformance-schema.md).
- [Releasing](RELEASING.md), [security policy](SECURITY.md), and
  [code of conduct](CODE_OF_CONDUCT.md).

Package-specific installation, APIs, and executable examples remain in each package's
`README.md` because those files ship to crates.io, PyPI, and npm.
