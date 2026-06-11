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
`keyless.py` directly — there is no stub file to keep in sync. The vendored
`ss58.py` (Polkascan code, kept byte-stable apart from hardening fixes) is the one
exception: it is shadowed by the handwritten `ss58.pyi` stub, which is the typed surface
of that module.
