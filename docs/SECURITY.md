# Security Policy

## Reporting a vulnerability

Please report suspected security issues privately to **support@trevo.finance**, or use
GitHub's private vulnerability reporting on this repository (Security → Report a
vulnerability). Do not open a public issue for anything you believe has security
impact.

We will acknowledge your report, keep you informed of the progress, and credit you in
the fix release unless you prefer otherwise.

## Response expectations and coordinated disclosure

- **Acknowledgment** within **3 business days** of the report.
- **Initial assessment** (accepted, declined, or needs more information) within
  **7 days** of acknowledgment, and status updates at least **every 14 days** while a
  fix is in progress.
- **Coordinated disclosure**: we ask for up to **90 days** from the report before public
  disclosure, and we aim to release fixes well inside that window. If a fix ships
  earlier, disclosure follows the fix; if we need longer (for example a divergence that
  requires coordinated releases of all three packages), we will agree on a new date with
  you rather than let the deadline pass silently.
- **Advisories** are published as GitHub Security Advisories on this repository, and
  propagated to the per-ecosystem databases (RustSec, PyPA advisory database, GitHub
  Advisory Database for npm) where applicable, alongside the fix release.

## Supported versions

Security fixes are released for the **latest minor release line of each package**
(Rust crate, Python package, npm package — the three are versioned independently).
Older versions do not receive backports; upgrading is always safe with respect to the
encoding, which is frozen and identical across all versions.

## Scope

This repository contains the Rust, Python, and TypeScript implementations of the Trevo
keyless address encoding. Reports of particular interest:

- Inputs on which the implementations disagree (classification, acceptance, or
  rejection) — the cross-language conformance fixture is meant to make this impossible,
  so any divergence is a bug.
- Address strings accepted in more than one spelling (malleability), beyond the
  documented Substrate-compatible exception for two-byte-prefix encodings of SS58
  formats below 64.
- Panics, unbounded resource consumption, or crashes on untrusted input.

## Verifying release provenance

All three packages are published through registry **trusted publishing** (no long-lived
tokens) with build provenance from the `publish-*` workflows in this repository. To
verify that an artifact you install really was built here:

- **npm** — run `npm audit signatures` in a project that depends on
  `@trevofinance/trevokeyless`: it verifies the registry signatures *and* the provenance
  attestation of every installed package. The npm package page also shows the
  "Provenance" panel linking the version to its workflow run.
- **PyPI** — the `trevokeyless` file pages on pypi.org display the PEP 740 attestations
  (uploaded by the PyPA publish action and verified by PyPI itself on upload). To verify
  a downloaded sdist or wheel locally, use the PyPA
  [`pypi-attestations`](https://pypi.org/project/pypi-attestations/) tool against this
  repository's publisher identity.
- **crates.io** — crates.io has no provenance mechanism of its own, so the publish
  workflow signs a GitHub attestation over the exact `.crate` file. Download the archive
  and check it with the GitHub CLI:

  ```
  curl -sLO https://static.crates.io/crates/trevokeyless/trevokeyless-X.Y.Z.crate
  gh attestation verify trevokeyless-X.Y.Z.crate --repo trevo-finance/trevokeyless
  ```

## What this library does and does not guarantee

See the [technical security model](security-model.md): the keyless checksum is a
classification and integrity mechanism, **not** authentication. Anyone can construct a
valid keyless address embedding any AppAgent ID without any secret. Reports built on
treating decode results as proof of authorship are out of scope.

No secrets are processed anywhere in these libraries; there are no constant-time
requirements.
