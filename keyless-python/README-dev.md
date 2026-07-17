# Keyless Addresses Python Library

## Development

Install the package with its development tools and run the checks that CI runs:

```
python -m pip install --require-hashes -r requirements-dev.txt
python -m pip install --no-deps --no-build-isolation .
ruff format --check
ruff check
mypy trevokeyless tests
python3 -m unittest discover
```

`requirements-dev.txt` pins every development/audit tool and transitive dependency
with hashes. Regeneration instructions are in `requirements-dev.in`; the test suite
itself is plain stdlib `unittest` and the published package has no runtime dependencies.

## Typing

The package ships a `py.typed` marker, so type checkers read the inline annotations of
every module directly — there are no stub files to keep in sync. (`_ss58.py` is a fully
typed port of the Rust reference SS58 decoder; the historically vendored Polkascan
codec and its handwritten stub are gone.) `tests/type_safety.py` is a compile-only
consumer fixture that pins the distinction between `AppAgentId` and `TransactionalId`.
