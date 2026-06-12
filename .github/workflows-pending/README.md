# Staged workflow updates

The credentials of the session that prepared this branch cannot write to
`.github/workflows/` (no `workflow` scope), so the workflow halves of the preceding
commits are staged here — the same mechanism used before the 0.9.0 line. Apply them by
replacing the live copies and deleting this directory:

```
cp .github/workflows-pending/*.yml .github/workflows/
git rm -r .github/workflows-pending
```

Until applied, the `check-ts` interop smoke test fails on this branch: it still
imports the old package name.

What changed, relative to the live copies:

- `check-ts.yml` — the interop smoke test imports the renamed
  `@trevofinance/trevokeyless`; a new step executes every TypeScript README example
  against the packed tarball (`keyless-ts/scripts/run-readme.mjs`).
- `publish-ts.yml` — header comment follows the package rename.
- `publish-rust.yml` — header comment follows the `trevokeyless` crate rename.
- `check-python.yml`, `publish-python.yml` — `pip install .[test]` becomes
  `pip install .[dev]` (the extra was renamed).
