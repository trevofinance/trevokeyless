# Rust fuzzing

The standalone workspace in `keyless-rust/fuzz/` has three targets:

- `ss58_decode` for SS58 parsing;
- `accountid_from_str` for raw account-ID parsing; and
- `classify` for classification, re-encoding, typed-decoder agreement, and checksum
  fragility.

Committed byte-exact seeds live under `keyless-rust/fuzz/seeds/<target>/`. New live
discoveries stay in ignored `fuzz/corpus/`; crash artifacts land in
`fuzz/artifacts/<target>/`.

`.github/scripts/run_fuzz.py` owns the supported target list, nightly and cargo-fuzz
pins, host target triple, lock check, corpus arguments, replay shape, and default duration.
Inspect those values with:

```console
python3 .github/scripts/run_fuzz.py --list
```

Run a target or replay one artifact:

```console
python3 .github/scripts/run_fuzz.py --target ss58_decode --ensure-tools
python3 .github/scripts/run_fuzz.py --target classify --reproducer <artifact>
```

Use `--duration` only when a task or CI schedule requires a different time budget. The
script validates the target against checked-in sources and verifies the fuzz lock before
execution.

Do not silently fix a finding or promote a corpus file. Remediation follows the Rust-first
behavioral-change workflow in [CONTRIBUTING.md](CONTRIBUTING.md), with a conformance vector
when the case is fixture-representable.
