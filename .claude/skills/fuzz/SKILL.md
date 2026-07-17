---
name: fuzz
description: Run or reproduce the Rust cargo-fuzz targets (ss58_decode, accountid_from_str, classify) with the CI-pinned nightly toolchain and the committed seed corpora.
argument-hint: "[ss58_decode|accountid_from_str|classify] [artifact-path]"
---

# Fuzzing

Keep local runs on CI's exact pins so a finding reproduces in CI (fuzz.yml is
authoritative):

```
rustup toolchain install nightly-2026-07-01
cargo install cargo-fuzz --version 0.13.2 --locked
```

Run a target (default ss58_decode) from keyless-rust/:

```
mkdir -p fuzz/corpus/<target>
RUSTUP_TOOLCHAIN=nightly-2026-07-01 cargo fuzz run <target> \
    fuzz/corpus/<target> fuzz/seeds/<target> \
    --target x86_64-unknown-linux-gnu -- -max_total_time=300
```

- The committed seed corpora (fuzz/seeds/<target>/) are required — without them a short
  run never reaches the accept-path assertions. fuzz/corpus/ is throwaway and
  gitignored. `RUSTUP_TOOLCHAIN` beats the repository-root rust-toolchain.toml, exactly
  as CI sets it.
- Crash reproducers land in fuzz/artifacts/<target>/. Replay one:
  ```
  RUSTUP_TOOLCHAIN=nightly-2026-07-01 cargo fuzz run <target> \
      fuzz/artifacts/<target>/<file> --target x86_64-unknown-linux-gnu
  ```
- A finding is a behavioral bug: fix it in Rust first, then pin the case as a
  conformance vector (the add-conformance-vector skill) so Python and TypeScript are
  checked against it too.
