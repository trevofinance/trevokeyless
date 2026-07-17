# Contributing

This repository hosts three implementations of the Trevo keyless address encoding. The
**Rust crate (`keyless-rust/`) is the source of truth**; the Python and TypeScript
packages must reproduce its behavior exactly.

## The one rule that matters

**Every behavioral change lands together with the regenerated conformance fixture and
the matching Python/TypeScript changes, in the same pull request.**

The shared fixture [`conformance/vectors.json`](./conformance/vectors.json) is the
executable specification (its per-group JSON schema is documented in
[`conformance/README.md`](./conformance/README.md)). It is generated from the Rust crate
and verified by all three test suites; CI additionally checks that the committed fixture
matches the generator output byte for byte. Regenerate it with:

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

Adding a vector is more than regenerating: all three suites pin each group's exact
vector count (and the exact group set), so the pins must move together with the fixture.
The step-by-step path — generator self-checks, the three count pins, what a *new group*
additionally needs — is documented in
[`conformance/README.md`](./conformance/README.md#adding-a-vector-or-a-group).

## Development

Each package is self-contained; CI runs, at minimum, these commands (plus the
conformance-fixture diff, packaging/artifact-content checks for all three packages,
dependency audits, a `no_std` build, docs and publish dry-run gates for Rust, and the
executable README examples — the workflow files under `.github/workflows/` are
authoritative).

Every cargo invocation in CI passes `--locked`: a dependency change must land with the
regenerated `Cargo.lock` (both `keyless-rust/Cargo.lock` and `keyless-rust/fuzz/Cargo.lock`
are audited).

```
# Rust (keyless-rust/)
cargo fmt --all -- --check
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
python3 ../.github/scripts/check_rust_artifacts.py   # exact .crate file list

# Python (keyless-python/)
python -m pip install --require-hashes -r requirements-dev.txt
python -m pip install --no-deps --no-build-isolation .
ruff format --check && ruff check
mypy trevokeyless tests
python3 -m unittest discover

# TypeScript (keyless-ts/)
npm ci
npm run format-check && npm run lint-check
npm run type-check
npm run build && npm run test
npm run check-platform-resolvers
# After installing Chromium once with `npx playwright install chromium`:
npm run check-platforms

# Cross-language differential check (repository root, after the TS build). The Rust crate
# is the oracle; Python and TypeScript are compared against it.
python3 .github/scripts/check_cross_language.py --cases 256

# The CI helper scripts themselves (repository root)
ruff format --check .github/scripts && ruff check .github/scripts
mypy --config-file .github/scripts/mypy.ini .github/scripts
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
```

`keyless-python/requirements-dev.txt` is the hash-pinned lock CI installs (the `.txt`
name is what makes Dependabot see it); `requirements-dev.in` holds the direct tool pins
and the exact `pip-compile` command that regenerates the lock. CI regenerates the lock
from the `.in` and fails on any drift, so the two cannot diverge.

README examples are executed by CI, and the Python and TypeScript runners **pin the
number of code blocks** so a silently skipped example cannot pass: adding or removing an
example means updating the pinned count in `keyless-python/tests/test_readme_examples.py`
(Python) or `keyless-ts/scripts/run-readme.mjs` (TypeScript). The Rust README needs no
count — it compiles as one doctest via `include_str!`.

### Fuzzing (Rust)

The fuzz harness is a standalone workspace under `keyless-rust/fuzz/` with three targets
— `ss58_decode`, `accountid_from_str`, and `classify` — and committed seed corpora
(`fuzz/seeds/<target>/`, see the README there) without which a short run never reaches
the accept-path assertions. cargo-fuzz needs a nightly toolchain; CI pins
`nightly-2026-07-01` and `cargo-fuzz@0.13.2` (fuzz.yml is authoritative — keep local
runs on the same pins so a finding reproduces in CI). To run locally:

```
rustup toolchain install nightly-2026-07-01
cargo install cargo-fuzz --version 0.13.2 --locked
cd keyless-rust
mkdir -p fuzz/corpus/ss58_decode
RUSTUP_TOOLCHAIN=nightly-2026-07-01 cargo fuzz run ss58_decode \
    fuzz/corpus/ss58_decode fuzz/seeds/ss58_decode \
    --target x86_64-unknown-linux-gnu -- -max_total_time=300
```

(Substitute either other target name; `RUSTUP_TOOLCHAIN` beats the repository-root
`rust-toolchain.toml`, exactly as CI sets it.) Crash reproducers land in
`fuzz/artifacts/<target>/`; new corpus discoveries in the throwaway `fuzz/corpus/`.
`cargo fuzz run <target> fuzz/artifacts/<target>/<file>` replays a single reproducer.

Cross-language conventions:

- Error classification is part of the contract: every `ss58_invalid` vector pins its
  `error_kind` (the Rust `Ss58Error` variant name, matched by the `Ss58Error.kind` of
  the TypeScript and Python implementations), and all three suites assert it. Error wording is
  additionally contractual where the READMEs say so; the name-validation messages
  ("Account name must be exactly 10 characters long" / "Account name contains invalid
  characters") and the SS58 messages (mirroring the Rust `Display` output) are shared
  verbatim by all three implementations — change them in all three and their tests at
  once.
- The Python package keeps its SS58 codec private (`trevokeyless._ss58`, a strict port
  of the Rust decoder); only its error types (`Ss58Error`, `Ss58ErrorKind`) and the
  `ACCOUNT_ID_LENGTH` / `SS58_MAX_ADDRESS_LENGTH` constants are re-exported. (`Ss58FormatMismatchError` is defined in `keyless.py`, not there.)
  The TypeScript equivalent (`src/ss58.ts`) is likewise internal — only `src/keyless.ts`
  is the public module.
- New public API needs: Rust-first design, the Python `__all__` + the
  `test_public_surface` pin updated, and conformance vectors when the behavior is
  externally visible.

## Releasing

The three packages are versioned independently. Full commands are documented in
[`RELEASING.md`](./RELEASING.md). To release one:

1. Run `.github/scripts/release_metadata.py --set <rust|python|ts> X.Y.Z`; this
   synchronizes the package's manifest and derived version declarations. The helper
   accepts stable `X.Y.Z` releases only. Breaking API changes require a **major** bump —
   and that includes raising a runtime/toolchain floor (MSRV, Node `engines`, the
   minimum Python); see the root README's versioning section.
2. Update the package's `CHANGELOG.md`: retitle the unreleased section with the
   version and date, and make sure everything shipping in the release is listed.
3. Land that on `main` through a pull request (CI validates the publishable artifacts
   on every run).
4. Tag the **merge commit on `main`** `rust-vX.Y.Z`, `python-vX.Y.Z`, or `ts-vX.Y.Z` and
   push the tag. The matching `publish-*` workflow re-runs the full check suite on the
   tagged commit — the package's `check-*` workflow, `check-docs`, *and*
   `check-cross-language` — verifies that the tag matches the manifest version and that
   **the tagged commit is an ancestor of `origin/main`** (a tag on an unmerged commit is
   refused), and publishes via registry trusted publishing (no stored tokens).

The one-time registry-side Trusted Publisher configuration is documented in the header
of each `.github/workflows/publish-*.yml`.
