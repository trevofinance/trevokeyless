# Generated files and executable documentation

Do not hand-edit generated or lock files. Change the source of truth, run its writer, and
review the resulting diff.

| Derived file | Source of truth and writer | Verification |
|---|---|---|
| `conformance/vectors.json` | `keyless-rust/examples/gen_conformance.rs`; follow [conformance.md](conformance.md) | CI regenerates and compares it byte-for-byte; all suites pin groups and counts |
| `keyless-python/requirements-dev.txt` | Edit `requirements-dev.in` and run the exact `pip-compile` command in its header on the documented Python version | Python CI regenerates it and fails on drift |
| `keyless-rust/Cargo.lock` | Change `Cargo.toml`; Cargo updates the lock. Releases use `release_metadata.py --set` | Cargo CI uses `--locked` and audits it |
| `keyless-rust/fuzz/Cargo.lock` | Change `fuzz/Cargo.toml`; Cargo updates the standalone workspace lock. Releases use `release_metadata.py --set` | Fuzz CI and release metadata checks pin it |
| `keyless-ts/package-lock.json` | Change `package.json` through npm install/update commands; releases use `release_metadata.py --set` | `npm ci`, tests, and release metadata checks pin it |

## Executable package READMEs

Examples are tests, not pseudocode. Keep every example self-contained with its imports.

- `keyless-rust/README.md` compiles as a doctest through `include_str!`. Its installation
  version is rewritten by `.github/scripts/release_metadata.py --set`.
- `keyless-python/README.md` code blocks are executed and counted by
  `keyless-python/tests/test_readme_examples.py`.
- `keyless-ts/README.md` code blocks execute against the packed tarball and are counted by
  `keyless-ts/scripts/run-readme.mjs`.

Update the corresponding count pin when adding or removing a Python or TypeScript block.
