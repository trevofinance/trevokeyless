---
name: behavior-change
description: End-to-end workflow for any observable trevokeyless behavior change, including encoding, validation order, error kinds/messages, account-name handling, or public API — keeps Rust, Python, TypeScript, the conformance fixture, and shared contracts in lockstep within one PR.
metadata: {"authority":"authoring","review_guard":"none"}
---

# Behavioral change checklist

This is an authoring workflow scoped to the requested observable change. It does not
authorize changing an existing frozen vector without confirmation or performing an
external release action.

Resolve paths from the repository root. Read `AGENTS.md`, the affected package
`AGENTS.md` files, and the complete behavioral change checklist in
`docs/CONTRIBUTING.md`. Apply the contracts in `docs/cross-language-contracts.md`.

Apply the `add-conformance-vector` skill for fixture-expressible behavior. After
implementation, apply the `verify` skill for evidence. Then delegate the complete diff to
an independent review worker when the runtime supports isolated workers, instructing it
to apply the `parity-review` skill under a caller-owned `review-guard` and not implement
fixes. Otherwise establish the guard and apply
`parity-review` directly. Do not apply `gates` separately when `verify` already selected
the same sets.

Report the changed files, checks, and review outcome.
