# CLAUDE.md — trevokeyless (Python)

Pure-Python implementation of the Trevo keyless address encoding. **Zero runtime
dependencies** — the strict base58 codec is vendored in `trevokeyless/_ss58.py` (SS58
payloads are at most 36 bytes, so the big-int arithmetic is trivial). Do not add a
runtime dependency without a very good reason.

The Rust crate (`../keyless-rust/`) is the source of truth for the encoding; this
package must reproduce its behavior exactly (see `../CONTRIBUTING.md` and the shared
fixture `../conformance/vectors.json`).

## File layout

```
trevokeyless/
  __init__.py    — re-exports, __all__ (the public API pin), hardcoded __version__
                   (kept equal to pyproject.toml by tests/test_version.py)
  keyless.py     — all public entry points: encode_*/decode_* address and account-id
                   functions, BlockchainAccountInfo, AccountType, the validating
                   identifier types (AppAgentId/TransactionalId are int subclasses that
                   enforce the u32 range in __new__), the public constants, error types
  _ss58.py       — PRIVATE strict SS58 codec (port of keyless-rust/src/ss58.rs) with
                   vendored base58; only its error types (Ss58Error, Ss58ErrorKind) and
                   the ACCOUNT_ID_LENGTH / SS58_MAX_ADDRESS_LENGTH constants are
                   re-exported as public API. keyless.py also
                   reuses its _text_encoder_utf8_length helper for the account-name rule
tests/
  test_conformance.py     — verifies ../../conformance/vectors.json (all groups)
  test_public_surface.py  — pins __all__ / the public surface; update when API changes
  test_readme_examples.py — executes the README's python3 code blocks (README must stay
                            executable; it counts the blocks — update when adding one)
  test_adversarial.py     — malformed/hostile input (whitespace, unicode, surrogates,
                            type confusion, checksum corruption)
  type_safety.py          — compile-only consumer assertions for nominal identifier types
  test_properties.py      — randomized round-trips (deterministic inline PRNG, same
                            xorshift64* logic as the TS twin)
  test_encoding.py / test_names.py / test_ss58.py / test_version.py
```

## Commands (the core CI gates)

```
python -m pip install --require-hashes -r requirements-dev.txt
python -m pip install --no-deps --no-build-isolation .
ruff format --check && ruff check
mypy trevokeyless tests
python3 -m unittest discover        # unittest, NOT pytest
```

CI installs the package **non-editable** and then re-runs the whole suite a second time
against the *installed* copy in site-packages, which catches packaging mistakes an editable
install hides (a module missing from the wheel, say). These are the core gates only:
`check-python.yml` runs a superset (packaging build + artifact-content checks, the
lock-and-scripts job, a Windows packaging smoke run) and is authoritative.

`README-dev.md` (this directory) is the human-facing copy of these commands plus the
typing notes (`py.typed`, no stub files, the `type_safety.py` consumer fixture); keep
the command lists in step when either changes.

Supported Pythons: 3.10–3.14 (CI matrix); `requires-python = ">=3.10"`.

## Conventions

- **Error taxonomy** (documented in the README, asserted by tests): wrong *type* →
  `TypeError` (`bool` is explicitly rejected where an int is expected); wrong *value*
  (out-of-range id or format, a reserved `ss58_format` handed to an *encoder*, malformed
  name/hex) → `ValueError`. `Ss58Error` subclasses
  `ValueError` and carries `.kind` (one of the five cross-language kinds);
  `Ss58FormatMismatchError` is its `"UnexpectedFormat"` case with `.expected`/`.found`;
  `AccountTypeMismatchError` (also a `ValueError`) carries the typed-decoder
  `(expected, found)` pair; `ParseAccountIdError` (malformed raw account id) and
  `NotAnAddressError` (raw account id handed to an address entry point) are the other two
  `ValueError` subclasses.
- **Account names** are validated by their **UTF-8 byte length** (10), not their
  code-point count — the Rust rule; the classification of a rejected non-ASCII name
  (`InvalidLength` vs `InvalidCharacter`) is pinned per vector by the
  `named_invalid_encode` fixture group. Never "simplify" this back to `len(name)`.
- Validation order is contractual and mirrored across languages. In the **decoders and
  `encode_address`**, the `ss58_format` parameter is checked before the address/account-id
  payload (a caller parameter is a programmer error; the address or account id is data,
  often untrusted — report the caller's own bug first), and full address validation
  (structure, length, checksum) precedes the expected-format comparison. The three
  **keyless encoders** check their identifier/name arguments *before* `ss58_format`, in
  declaration order (the open part is built before the format is examined) — matching
  TypeScript, and pinned by the both-invalid ordering tests in `test_adversarial.py` /
  `adversarial.test.ts`.
- The name-validation and SS58 error messages are shared verbatim with Rust/TS — change
  them everywhere at once (see root CLAUDE.md).
- `ss58_format` is keyword-only everywhere (encoders and decoders alike).
- Raw `0x` account ids are rejected by the SS58 address entry points
  (`decode_address`, `decode_address_with_format`, `is_keyless_address`, the typed
  address decoders); every one of them has a raw-account-id counterpart
  (`decode_account_id` / `encode_address` / `is_keyless_account_id` /
  `decode_*_account_id`).
- New public API needs: Rust-first design, `__all__` + `test_public_surface` updated,
  README section + example (executed in CI), conformance vectors when externally
  visible, and a semantic version update via the root release-metadata helper (major
  for breaking changes).
- Style: ruff (strict select list, line length 120), mypy `strict = true` on the
  package. Google-style docstrings with Args/Returns/Raises.
