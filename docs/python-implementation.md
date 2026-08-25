# Python implementation

The package is a zero-runtime-dependency Python reproduction of Rust-defined behavior.

- `trevokeyless/__init__.py` owns exports, `__all__`, and `__version__`.
- `trevokeyless/keyless.py` owns public address/account-ID APIs and errors.
- `trevokeyless/_ss58.py` is private except for explicitly re-exported contracts.
- `tests/test_public_surface.py` pins exports.
- `tests/test_readme_examples.py` executes package README examples.
- `tests/test_adversarial.py` is paired with TypeScript host-boundary coverage.
- `tests/type_safety.py` is the compile-only consumer typing fixture.

Use UTF-8 byte length for account names. Keep `ss58_format` keyword-only. Reject wrong
types—including `bool` used as an integer—with `TypeError`; invalid values use documented
`ValueError` subclasses. Raw account IDs and SS58 addresses retain separate entry points.
New public API starts in Rust and updates exports, public-surface tests, executable README,
conformance where observable, version, and changelog. Preserve strict mypy, configured
ruff style, and Google-style docstrings.
