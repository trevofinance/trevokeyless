# Keyless Addresses Python Library

## Development

Install the package with its development tools and run the checks that CI runs:

```
pip install -e .[test]
ruff format --check
ruff check
mypy trevokeyless
python3 -m unittest discover
```

## Typing

The package ships a `py.typed` marker, so type checkers read the inline annotations of
every module directly — there are no stub files to keep in sync. (`_ss58.py` is a fully
typed port of the Rust reference SS58 decoder; the historically vendored Polkascan
codec and its handwritten stub are gone.)
