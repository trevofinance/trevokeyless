# Committed fuzz seed corpora

One directory per fuzz target, one input per file, byte-exact and with **no trailing
newline** (libFuzzer passes the file contents to the target verbatim, so a stray byte
turns a seed into a rejected input). All are derived from `conformance/vectors.json`:

- `ss58_decode/` — address strings: valid addresses across account types and formats,
  non-canonical decode-only spellings, and structured rejections.
- `accountid_from_str/` — the `0x` + 64-hex contract cases (valid and invalid).
- `classify/` — raw **32-byte account ids** (not hex strings): valid AppAgent /
  Transactional / Named accounts including the identifier boundaries and the
  charset-boundary names, arbitrary regular accounts, single-byte corruptions and
  discriminant rewrites of keyless accounts, and accounts carrying a valid checksum over
  out-of-charset name bytes.

Without seeds, a 30 s/300 s libFuzzer run essentially never synthesizes a checksum-valid
base58 address — or a checksum-valid keyless account id — from scratch, so the accept-path
assertions in the targets (round-trip, re-encode, typed-decoder agreement) would never
execute.

CI uses `cargo-fuzz` 0.13.2 with the pinned nightly and passes these as a read-only
input directory. Reproduce the tool version locally with
`cargo install cargo-fuzz --version 0.13.2 --locked`, then run (from `keyless-rust/`;
the corpus directory is gitignored and absent in a fresh checkout, and cargo-fuzz
defaults to its own build triple, so the explicit `--target` matters — the same
command CONTRIBUTING.md and the workflows use):

```
mkdir -p fuzz/corpus/<target>
cargo +nightly-2026-07-01 fuzz run <target> fuzz/corpus/<target> fuzz/seeds/<target> \
    --target x86_64-unknown-linux-gnu
```

New coverage-increasing inputs land in `fuzz/corpus/` (gitignored, local); this
directory stays a small curated set. If the fixture gains materially new vector
shapes, regenerate by copying the relevant `ss58`/`input` fields into files here (for
`classify`, the `hex` fields decoded back to their 32 raw bytes).
