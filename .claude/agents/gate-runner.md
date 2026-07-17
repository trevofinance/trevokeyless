---
name: gate-runner
description: Runs one trevokeyless gate set (rust, python, ts, scripts, or cross) — the local CI-equivalent commands — and returns a compact pass/fail report. Use to validate changes without flooding the caller with build and test logs.
tools: Bash, Read, Grep, Glob
---

You are given one gate set: rust | python | ts | scripts | cross. Run its commands in
order from the stated directory. Do not stop at the first failure — run every command
in the set, then report.

**rust** (from keyless-rust/): `cargo fmt --all -- --check`;
`cargo clippy --locked --all-features --all-targets -- -D warnings`;
`cargo test --locked --all-features`; `cargo test --locked --no-default-features`;
`python3 ../.github/scripts/check_rust_artifacts.py`.

**python** (from keyless-python/; create/activate a virtualenv first if ruff/mypy are
missing): `python -m pip install --require-hashes -r requirements-dev.txt`;
`python -m pip install --no-deps --no-build-isolation .`;
`ruff format --check`; `ruff check`; `mypy trevokeyless tests`;
`python3 -m unittest discover` (unittest, NOT pytest).

**ts** (from keyless-ts/): `npm ci`; `npm run format-check`; `npm run lint-check`;
`npm run type-check`; `npm run build`; `npm test`; `npm run check-platform-resolvers`;
`npm run check-platforms` (needs Chromium via `npx playwright install chromium` once
per host; the bundled Hermes is x64-only — skip with a note, or set HERMES_BINARY,
on other architectures).

**scripts** (from the repository root):
`ruff format --check .github/scripts`; `ruff check .github/scripts`;
`mypy --config-file .github/scripts/mypy.ini .github/scripts`;
`python3 -m unittest discover -s .github/scripts -p 'test_*.py'`.

**cross** (from the repository root; requires an up-to-date keyless-ts build —
run `npm run build` there first if dist/ is stale):
`python3 .github/scripts/check_cross_language.py --cases 256`.

Report format: one line per command — PASS or FAIL. For each FAIL, quote only the
decisive error excerpt (a few lines, never the full log) and the file:line it points
at. End with a single overall verdict line: `<set>: N/M passed`.
