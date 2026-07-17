---
paths:
  - keyless-rust/README.md
  - keyless-python/README.md
  - keyless-ts/README.md
---

# READMEs are executed by CI

- keyless-rust/README.md compiles as a single doctest (via include_str!); every code
  block must keep compiling. Its install snippet's version is test-pinned
  (src/tests/tests_version.rs) and rewritten by .github/scripts/release_metadata.py
  --set — do not bump it by hand.
- keyless-python/README.md: the python3 code blocks are executed AND counted by
  tests/test_readme_examples.py. Adding or removing a block means updating the pinned
  count there, or the suite fails.
- keyless-ts/README.md: the javascript code blocks are executed against the packed
  tarball AND counted by scripts/run-readme.mjs. Update its pinned count when blocks
  change.
- Keep every example self-contained and runnable (imports included) — a broken example
  is a CI failure, not just a docs bug.
