---
name: gates
description: Run the local CI-equivalent gate suite for one trevokeyless package (rust, python, ts), the CI helper scripts, the cross-language differential check, or everything. Use when asked to run gates, tests, or local CI; accepts rust, python, ts, scripts, cross, or all.
metadata: {"authority":"non-authoring","review_guard":"required-at-top-level"}
---

# Local CI gates

This is non-authoring diagnostic work. Establish one top-level `review-guard` before
delegation or direct execution and pass its ownership to every `gate-set` invocation.
Verify the same guard after all selected sets finish, including after failures.

Resolve paths from the repository root. Derive the requested set from the user's request:
`rust`, `python`, `ts`, `scripts`, `cross`, or `all`. Default to `all`. Reject unknown set
names instead of guessing.

When the runtime supports isolated workers, delegate each selected set to a separate
worker and instruct it to apply the `gate-set` skill so build and test logs stay outside
the caller's context. For `all`, start `rust`, `python`, `scripts`, and `ts` in parallel
where runtime capacity permits. Start `cross` only after `ts` finishes, even when `ts`
failed, so dependency installation and TypeScript build output cannot race the
cross-language prerequisite. A standalone `cross` request is responsible for satisfying
its own prerequisite through `gate-set`.

If isolated-worker delegation is unavailable, apply `gate-set` directly for each selected
set with the same scheduling constraint. `.github/scripts/run_checks.py` is the command
and ordering owner; do not reproduce its inventory here.

Present one consolidated table with a row per set, its passed/total counts, skips, and
verdict. Include only decisive failure excerpts and explicitly distinguish a set that
could not run from one that passed.
