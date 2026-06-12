# Security Policy

## Reporting a vulnerability

Please report suspected security issues privately to **support@trevo.finance**.
Do not open a public issue for anything you believe has security impact.

We will acknowledge your report, keep you informed of the progress, and credit you in
the fix release unless you prefer otherwise.

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

## What this library does and does not guarantee

See the *Security model* section of the [README](./README.md): the keyless checksum is
a classification and integrity mechanism, **not** authentication. Anyone can construct
a valid keyless address embedding any AppAgent ID without any secret. Reports built on
treating decode results as proof of authorship are out of scope.

No secrets are processed anywhere in these libraries; there are no constant-time
requirements.
