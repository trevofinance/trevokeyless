---
name: fuzz
description: Run or reproduce the Rust cargo-fuzz targets ss58_decode, accountid_from_str, or classify with the CI-pinned nightly toolchain and committed seed corpora. Use for fuzz runs, smoke tests, or artifact replays; accepts an optional target and reproducer path.
metadata: {"authority":"non-authoring","review_guard":"required-at-top-level"}
---

# Fuzzing

This is non-authoring diagnostic work. Establish one `review-guard` when invoked at the
top level, or reuse the explicitly declared caller-owned guard. Verify it after the run
and before reporting success. Corpus promotion or finding remediation requires separate
authoring authority.

Resolve paths from the repository root. Derive an optional target and reproducer path
from the user's request; default the target to `ss58_decode`. Before installing tools or
running a target, read `docs/fuzzing.md`, `keyless-rust/AGENTS.md`, and the committed
seed-corpus README. `.github/scripts/run_fuzz.py` is authoritative for supported targets,
pins, lock checks, corpus arguments, and replay mechanics.

Use the executable owner for a normal committed-seed run or a single-artifact replay.
Install its pinned prerequisites when missing; do not reconstruct the command from CI.

Report the resolved target, pins, exact command, duration or replay mode, outcome, and
any artifact path. Do not silently fix a finding or promote corpus files. If the task
also includes remediation, treat the finding as a Rust-first behavioral change and apply
the `behavior-change` skill plus `add-conformance-vector` when the case is
fixture-expressible.
