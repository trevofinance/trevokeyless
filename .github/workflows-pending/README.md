# Pending workflow updates

The automation environment that produced this branch cannot push changes under
`.github/workflows/` (its credential lacks the `workflow` scope), so the updated and
new workflow files live here instead. To apply them, run from the repository root:

```
git mv -f .github/workflows-pending/check-rust.yml \
          .github/workflows-pending/check-python.yml \
          .github/workflows-pending/check-ts.yml \
          .github/workflows-pending/publish-rust.yml \
          .github/workflows-pending/publish-python.yml \
          .github/workflows-pending/publish-ts.yml \
          .github/workflows/
git rm .github/workflows-pending/README.md
git commit -m "CI: apply pending workflow updates"
```

What changes relative to the current `.github/workflows/`:

- `check-rust.yml`: adds a `cargo publish --dry-run` step, a nightly cargo-fuzz smoke
  job for the SS58 decoder, and a `cargo audit` job.
- `check-python.yml`: matrix 3.10/3.12/3.13/3.14, a `pip-audit` step, and
  build + `twine check --strict` + wheel/sdist content assertions.
- `check-ts.yml`: matrix 22/24, `npm audit --omit=dev --audit-level=high`, and an
  `npm pack --dry-run` package-contents allowlist check.
- `publish-rust.yml` / `publish-python.yml` / `publish-ts.yml` (new): tag-triggered
  releases via registry trusted publishing; see the file headers for the one-time
  registry-side setup.
