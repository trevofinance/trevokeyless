# Releasing

The Rust, Python, and TypeScript packages are versioned independently. Update every
derived declaration for one package with the repository helper:

```console
python3 .github/scripts/release_metadata.py --set rust 2.1.0
python3 .github/scripts/release_metadata.py --set python 2.1.0
python3 .github/scripts/release_metadata.py --set ts 2.1.0
```

The helper accepts stable release versions in `X.Y.Z` form only; it deliberately rejects
prerelease and build metadata.

For `rust` this rewrites four declarations — `Cargo.toml`, `Cargo.lock`,
`fuzz/Cargo.lock` (the fuzz crate pins the library by path and is audited separately), and
the install snippet in `keyless-rust/README.md` (the README ships inside the `.crate`).
All four are verified by the checker below and by its unit tests.

Between releases, notes accumulate under the `## [Unreleased]` heading of the package's
`CHANGELOG.md`; the checker accepts a changelog whose newest release heading lags the
manifests only while that section exists. In the release pull request, replace
`## [Unreleased]` with `## [X.Y.Z] — YYYY-MM-DD` (make sure everything shipping in the
release is listed) — a publish tag fails its gates while the release heading is missing.
Review the generated lockfile diff, and run the checker:

```console
python3 .github/scripts/release_metadata.py
```

Publish tags are package-specific: `rust-vX.Y.Z`, `python-vX.Y.Z`, and `ts-vX.Y.Z`. Tag
the merge commit on `main`: before publishing, each publish workflow

- fails unless the tagged commit is an **ancestor of `origin/main`** (a tag can otherwise
  point at any commit, and a tag push is all it takes to publish);
- fails unless the tag version equals the package manifest version;
- re-runs the full check suite on the tagged commit: the package's `check-*` workflow,
  `check-docs`, and `check-cross-language` (the differential check against the Rust source
  of truth);
- asserts the exact contents of the artifact it is about to upload;
- publishes with **build provenance**: npm via `--provenance`, PyPI via the PyPA action's
  PEP 740 attestations, and crates.io — which has no provenance mechanism of its own — via a
  signed `actions/attest-build-provenance` attestation over the exact `.crate` file, with a
  digest check afterwards that the archive `cargo publish` uploaded is the one attested.

Publishes are serialized per package (a `concurrency` group with no cancellation), so two
tags pushed back-to-back queue instead of racing at the registry. The queue is only **one
deep**: GitHub keeps one running plus at most one pending run per group, so a *third* tag
pushed while two are already in flight silently replaces (cancels) the pending middle run
— that tag is never published. `publish-watch.yml` files an issue whenever a publish run
ends cancelled; recover by re-running the cancelled workflow run from the Actions tab once
the queue drains (safe at any time: every gate re-runs from scratch, and the registries
accept publishing a version lower than the latest). Pushing release tags one at a time,
waiting for each publish to finish, avoids the situation entirely.
