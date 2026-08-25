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

CI passes these seeds as a read-only input directory. The canonical local procedure and
artifact-replay workflow are in [`docs/fuzzing.md`](../../../docs/fuzzing.md); the current
tool and target pins remain authoritative in `.github/workflows/fuzz.yml`.

New coverage-increasing inputs land in `fuzz/corpus/` (gitignored, local); this
directory stays a small curated set. If the fixture gains materially new vector
shapes, regenerate by copying the relevant `ss58`/`input` fields into files here (for
`classify`, the `hex` fields decoded back to their 32 raw bytes).
