---
name: review-guard
description: Guard a diagnostic-write-capable review with one tracked-source and Git-state snapshot owned by the top-level review execution.
metadata: {"authority":"non-authoring","review_guard":"none"}
---

# Guard review-only execution

The top-level review creates exactly one guard before any analyzer, generator, test, or
delegated review runs. Create a temporary directory outside the repository and invoke
`scripts/review_guard.py snapshot` with the repository root and a snapshot path inside
that directory. Consult `--help` for exact syntax.

Delegated reviewers reuse the caller's guard and report `REVIEW_GUARD=CALLER_OWNED`.
When no caller-owned guard is declared, a directly invoked review establishes its own.

After all review work, run `scripts/review_guard.py verify` against the same snapshot.
Untracked diagnostics are allowed. Any new tracked-source, index, ref, HEAD, or Git
operation mutation prevents an approving verdict. Report the mutation and do not
silently restore it because it may overlap user work.

The final review records exactly one of `TRACKED_STATE_POSTCONDITION=PASS`,
`TRACKED_STATE_POSTCONDITION=FAIL`, or `TRACKED_STATE_POSTCONDITION=ERROR`.
