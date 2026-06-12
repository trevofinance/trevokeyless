# Contributing

This repository hosts three implementations of the Trevo keyless address encoding. The
**Rust crate (`keyless-rust/`) is the source of truth**; the Python and TypeScript
packages must reproduce its behavior exactly.

## The one rule that matters

**Every behavioral change lands together with the regenerated conformance fixture and
the matching Python/TypeScript changes, in the same pull request.**

The shared fixture [`conformance/vectors.json`](./conformance/vectors.json) is the
executable specification. It is generated from the Rust crate and verified by all three
test suites; CI additionally checks that the committed fixture matches the generator
output byte for byte. Regenerate it with:

```
cd keyless-rust
cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json
```

and **review the diff**. A change to the *value* fields of an existing vector (`hex`,
`ss58`, `expected`, `account_id`) means the encoding itself changed — that is a
breaking change of the on-chain format and needs a very good reason. Additive changes —
new vector groups, new vectors in a group, new metadata fields such as `error_kind` —
are the normal way to pin newly specified behavior (rejection cases, boundary cases,
error classifications).

## Development

Each package is self-contained; CI runs exactly these commands.

```
# Rust (keyless-rust/)
cargo fmt --all -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo test --no-default-features

# Python (keyless-python/)
pip install -e .[test]
ruff format --check && ruff check
mypy trevokeyless
python3 -m unittest discover

# TypeScript (keyless-ts/)
npm ci
npm run format-check && npm run lint-check
npm run build && npm run test
```

Cross-language conventions:

- Error wording is part of the contract where the READMEs say so (name validation, the
  SS58 format mismatch); change it in all three implementations and their tests at once.
- The Python package keeps the vendored generic SS58 codec private
  (`trevokeyless._ss58`); do not re-export it. The TypeScript equivalent
  (`src/ss58.ts`) is likewise internal — only `src/keyless.ts` is the public module.
- New public API needs: Rust-first design, the Python `__all__` + the
  `test_public_surface` pin updated, and conformance vectors when the behavior is
  externally visible.

## Releasing

The three packages are versioned independently. To release one:

1. Bump the version in its manifest (`Cargo.toml` / `pyproject.toml` / `package.json`,
   and `Cargo.lock` / `package-lock.json` where applicable) and update its
   `CHANGELOG.md`.
2. Land that on `main` through a pull request (CI validates the publishable artifacts
   on every run).
3. Tag the merge commit `rust-vX.Y.Z`, `python-vX.Y.Z`, or `ts-vX.Y.Z` and push the
   tag. The matching `publish-*` workflow re-runs the tests, verifies the tag matches
   the manifest version, and publishes via registry trusted publishing (no tokens).

The one-time registry-side Trusted Publisher configuration is documented in the header
of each `.github/workflows/publish-*.yml`.
