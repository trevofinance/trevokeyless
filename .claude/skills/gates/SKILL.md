---
name: gates
description: Run the local CI-equivalent gate suite for one trevokeyless package (rust, python, ts), the CI helper scripts, the cross-language differential check, or everything.
argument-hint: "[rust|python|ts|scripts|cross|all]"
---

# Local CI gates

Run the requested gate set (default: `all`). For `all`, delegate each set to the
gate-runner subagent — in parallel where possible — and present one consolidated
pass/fail table with failure excerpts only. Run every command in a set even after a
failure, then report.

## rust — from keyless-rust/

```
cargo fmt --all -- --check
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
python3 ../.github/scripts/check_rust_artifacts.py
```

## python — from keyless-python/ (activate/create a virtualenv if the tools are missing)

```
python -m pip install --require-hashes -r requirements-dev.txt
python -m pip install --no-deps --no-build-isolation .
ruff format --check && ruff check
mypy trevokeyless tests
python3 -m unittest discover        # unittest, NOT pytest
```

## ts — from keyless-ts/

```
npm ci
npm run format-check && npm run lint-check
npm run type-check
npm run build && npm test
npm run check-platform-resolvers
npm run check-platforms             # needs `npx playwright install chromium` once per
                                    # host; bundled Hermes is x64-only (else HERMES_BINARY)
```

## scripts — from the repository root (the CI helper scripts themselves)

```
ruff format --check .github/scripts && ruff check .github/scripts
mypy --config-file .github/scripts/mypy.ini .github/scripts
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
```

## cross — from the repository root (requires the keyless-ts build to exist)

```
python3 .github/scripts/check_cross_language.py --cases 256
```

These are the core gates; the workflows under .github/workflows/ are authoritative and
run a superset (version matrices, packaging/artifact checks, audits, platform jobs).
