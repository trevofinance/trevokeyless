# Development and verification

`.github/scripts/run_checks.py` is the executable owner for reusable local check
selection, command bodies, and ordering. Workflows own matrix-only platform, packaging,
audit, and publication gates; they consume the executable owner for shared sets.

## Named sets

| Set | Purpose | Prerequisites and interpretation |
|---|---|---|
| `rust` | Formatting, lint, feature-complete and no-default tests, crate allowlist | Stable Rust with rustfmt and clippy |
| `python` | Locked environment, formatting, lint, strict types, unit tests | A supported Python interpreter |
| `scripts` | Formatting, lint, types, and unit tests for repository helpers | The Python development lock installed |
| `ts` | Locked install, formatting, lint, types, build, tests, resolver checks | A supported Node/npm pair |
| `cross` | Seeded Rust/Python/ESM/CommonJS differential | Builds TypeScript itself when `ts` was not selected |
| `all` | Every set in dependency-safe order | Runs `cross` only after the TypeScript set |

Inspect the current inventory without executing it:

```console
python3 .github/scripts/run_checks.py --list
```

Run one or more sets by name, or use `all`:

```console
python3 .github/scripts/run_checks.py rust
python3 .github/scripts/run_checks.py python scripts
python3 .github/scripts/run_checks.py all
```

Each command is an independent leaf. The owner continues after a leaf failure and exits
nonzero after reporting per-set totals. It never repairs failures. Compare repository
status before and after; untracked build output is expected, but a check-created tracked
change must be reported and not silently reverted.

## Selecting verification

- Package-internal change: run that package set and focused tests.
- Shared encode/decode, validation, error, fixture, or shared-path refactor: run `all`.
- Fixture-affecting change: regenerate to a temporary path, compare byte-for-byte, then
  run all consumers.
- Error-message, validation-order, account-name, surrogate, or host-number change: also
  compare the paired Python/TypeScript adversarial coverage.
- Documentation or configuration change: run repository-script and relevant structural
  checks plus any package whose published README or artifact changed.

Chromium runtime checks require a once-per-host Playwright installation. The bundled
Hermes executable targets x64; another architecture supplies `HERMES_BINARY`. CI adds
platform and artifact checks beyond the reusable local sets.

## AI configuration

The deterministic neutral owner is:

```console
python3 .github/scripts/check_ai_config.py --scope core
```

Use `--scope adapters` for installed adapter contracts and `--scope all` for the combined
remote verdict. The combined verdict scans tracked and pending AI configuration for secret
or transcript material, overlays the complete working patch into an isolated Git worktree,
removes every installed adapter and its external shims, and reruns core validation. Run unit
tests through the `scripts` set. Provider adapters own independent static, mutation, and
live-smoke entry points; static repository validation never claims live client behavior.
Live records are required even when a probe cannot run: use `NOT_RUN` with a concise reason.
Each record's SHA-256 input binding covers canonical routers and skills plus the adapter's
manifest, mechanics, smoke procedure, external shims, and workflow; changing any input makes
the record stale until it is deliberately refreshed.

## CI overview

- Package workflows consume their matching shared set and add version/platform matrices,
  audits, packaging, README, and publication-dry-run coverage.
- `check-cross-language.yml` supplies scheduled versus pull-request seed/case values to
  the shared cross set.
- `fuzz.yml` consumes `.github/scripts/run_fuzz.py` for pinned fuzz mechanics.
- `check-docs.yml` covers generated freshness, links, release metadata, skills, and AI
  configuration.
- Publish workflows re-run package, docs, and differential gates before external action.
