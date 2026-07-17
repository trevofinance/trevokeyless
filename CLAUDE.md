# CLAUDE.md — trevokeyless (repository root)

Three implementations of the Trevo keyless address encoding — Rust, Python, TypeScript —
kept behaviorally identical through a shared conformance fixture. Used by the Trevo
blockchain runtime (Rust) and by off-chain apps (all three).

## Repository map

```
keyless-rust/      Rust crate `trevokeyless` — THE SOURCE OF TRUTH for the encoding
keyless-python/    Python package `trevokeyless` (zero runtime dependencies)
keyless-ts/        npm package `@trevofinance/trevokeyless` (native ESM + CommonJS, plus
                   browser/react-native export conditions; Node 22.13+)
conformance/       vectors.json — the executable cross-language specification,
                   generated from the Rust crate; schema documented in conformance/README.md
CONTRIBUTING.md    the one rule that matters + release process (read before changing behavior)
SECURITY.md        security policy; the security model itself is in README.md
```

Each package directory has its own CLAUDE.md with layout, commands, and conventions.

## The one rule that matters

**Every behavioral change lands together with the regenerated conformance fixture and
the matching changes in all three implementations, in the same pull request.**

Regenerate the fixture with:

```
cd keyless-rust
cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json
```

CI diffs the committed fixture against the generator output byte-for-byte, and all three
test suites verify against it. A change to a *value* field of an existing vector means
the frozen on-chain encoding changed — that needs a very good reason. New vectors/groups
are the normal way to pin new behavior.

## Cross-language contracts (do not break unilaterally)

- **Interchange formats**: only the SS58 address string and the raw account id
  (`0x` + 64 lowercase hex) cross language boundaries. Decoded info objects and error
  *messages* are per-language shapes (exceptions below).
- **Error taxonomy**: the five `Ss58Error` kinds (`InvalidBase58 | InvalidLength |
  InvalidChecksum | InvalidFormat | UnexpectedFormat`) are shared and pinned per vector
  (`error_kind`); adding a kind is a breaking cross-language change.
- **Shared error wording** (verbatim in all three, per CONTRIBUTING): the SS58 messages
  (mirroring the Rust `Display` output) and the account-name validation messages
  ("Account name must be exactly 10 characters long" / "Account name contains invalid
  characters"). Change them in all three implementations and their tests at once.
- **Typed-decoder mismatch pair**: the `(expected, found)` account types carried by the
  Rust `KeylessDecodeError::WrongAccountType` and the Python/TS
  `AccountTypeMismatchError`, pinned by the `type_mismatch` fixture group.
- **`AccountType` numeric values**: Regular=0, AppAgent=1, Transactional=2, Named=3 in
  all three; the keyless values equal the discriminant at byte `[4]` of the account id.
- Naming convention: Rust says *account* (works on `AccountId32` bytes), Python/TS say
  *address* (work on SS58 strings). This split is deliberate.

## Versioning and releases

The three packages are versioned independently (semver; the byte encoding itself is
frozen and never changes). Release = version bump PR to `main`, then tag
`rust-vX.Y.Z` / `python-vX.Y.Z` / `ts-vX.Y.Z`; the matching
`.github/workflows/publish-*.yml` re-runs the gates, verifies tag == manifest version,
and publishes via registry trusted publishing (no stored tokens), with build provenance on all three
artifacts (npm `--provenance`, PyPI PEP 740 attestations, and an
`actions/attest-build-provenance` attestation over the `.crate`). Adding public API to a
package requires bumping its **minor** version before releasing it.

A tag only publishes if its commit is an ancestor of `origin/main` and the tag version
equals the manifest version.

Version sync points (each is test-pinned, and all are rewritten by
`.github/scripts/release_metadata.py --set`): `keyless-rust/Cargo.toml` + `Cargo.lock` +
`fuzz/Cargo.lock` + the install snippet in `keyless-rust/README.md`;
`keyless-python/pyproject.toml` + `trevokeyless/__init__.py::__version__`;
`keyless-ts/package.json` + `package-lock.json`.

## CI overview

`check-rust` / `check-python` / `check-ts` run per-package format+lint+type checks, tests
on version matrices, dependency audits, artifact-content checks (an exact allowlist per
package: `.github/scripts/check_rust_artifacts.py`, `check_python_artifacts.py`,
`keyless-ts/scripts/check-package.mjs`), and execute the README examples (the Rust README
is a doctest; Python/TS run their READMEs' code blocks, so README examples must stay
executable). Rust cargo invocations pass `--locked`, and every Rust job exports
`RUSTUP_TOOLCHAIN` so the root `rust-toolchain.toml` cannot override the toolchain the job
installs (that is what makes the MSRV job real). `check-rust` also runs the full suite on
big-endian s390x under QEMU — the one job that would catch an endianness regression in the
byte format. `check-python` additionally verifies that
`keyless-python/requirements-dev.txt` still regenerates from `requirements-dev.in`, and
lints/type-checks the `.github/scripts/` helpers.

`check-cross-language` runs the seeded differential harness
(`.github/scripts/check_cross_language.py`): Rust is the oracle, Python and TypeScript are
compared against it, on a fixed seed for PRs and pushes and a run-id seed on the weekly
schedule (a scheduled failure files an issue, like the other scheduled workflows).

`check-docs` greps for pre-1.0 renamed symbols, verifies the release metadata
(`release_metadata.py`, including the conformance-fixture freshness), runs the
release-helper unit tests, and link-checks the Markdown with lychee.

`fuzz.yml` runs scheduled cargo-fuzz on the Rust fuzz targets (`ss58_decode`,
`accountid_from_str`, `classify`); `check-rust` smoke-runs the same targets for 30s each.

`audit.yml` re-runs the dependency audits of all three ecosystems on a weekly schedule
(the `check-*` workflows already audit on every push/PR); like the other scheduled
workflows, a failure files an issue via `report_scheduled_failure.sh`.

The three `publish-*` workflows call the package's `check-*`, `check-docs`, and
`check-cross-language` before publishing. `publish-watch.yml` files an issue when a
publish run ends cancelled (a third rapid tag silently replaces the pending queue slot —
see RELEASING.md for the recovery steps). Exact commands per package: see the package
CLAUDE.md files or CONTRIBUTING.md.
