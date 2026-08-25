---
name: gate-set
description: Execute and report exactly one trevokeyless core gate set. Use only when a parent workflow, delegated worker task, or explicit user request names one of rust, python, ts, scripts, or cross.
metadata: {"authority":"non-authoring","review_guard":"required-at-top-level"}
---

# Execute one gate set

This is non-authoring diagnostic work. A direct top-level invocation establishes one
`review-guard`; a delegated invocation must receive and acknowledge the caller-owned
guard. Verify the top-level guard before reporting a passing result.

Resolve paths from the repository root. The request or delegated task must name exactly
one set: `rust`, `python`, `ts`, `scripts`, or `cross`. Do not broaden it to other sets.

Before running anything, read the requested set and execution contract from
`docs/development.md`, plus the affected package `AGENTS.md` when one exists. Execute the
named set through `.github/scripts/run_checks.py`; do not reconstruct its inventory in
this skill, a prompt, or provider configuration.

Execution contract:

1. Satisfy missing local prerequisites. The executable owner handles working directories
   and the standalone `cross` TypeScript prerequisite.
2. Apply the canonical gate execution contract from `docs/development.md`.
3. This is a read-only business-logic task: do not repair product source or behavior.
   Setup state, dependencies, toolchains, virtual environments, caches, scratch output,
   and build artifacts are allowed under the repository execution policy.
4. Never discard pre-existing changes.

Report one compact line per leaf or prerequisite command with `PASS`, `FAIL`, or `SKIP`.
For a failure, include only the decisive verbatim excerpt and a `file:line` location when
the output supplies one. End with `<set>: N/M passed` and append `; K skipped` when
applicable.
