---
name: parity-review
description: Review a trevokeyless diff for violations of the cross-language behavior, fixture, public-surface, generated-file, and release contracts.
metadata: {"authority":"review-only","review_guard":"required-at-top-level"}
---

# Cross-language parity review

Establish one top-level `review-guard` before any review pass or diagnostic command, or
acknowledge an explicitly caller-owned guard when delegated. Verification is mandatory
before the verdict; a guard failure prevents approval.

Resolve paths from the repository root. This is a review-only business-logic task. Do not
implement fixes or intentionally edit product source. Repository setup, scratch output,
builds, regeneration for comparison, and focused checks are allowed under the repository
execution policy. Never discard pre-existing changes; report any tracked-file change
caused by a check.

Resolve and state the review scope using the canonical procedure in
`docs/CONTRIBUTING.md`.

## Review against authoritative sources

Read `docs/CONTRIBUTING.md` and `docs/cross-language-contracts.md` before judging the
diff. Read the conformance documents when the fixture, encoding, validation, shared
errors, or host-only parity can be affected. Read affected package `AGENTS.md` files and
`docs/RELEASING.md` when version or release metadata is involved.

Build the checklist from the changed behavior and those sources. Cover every applicable
contract category, including behavioral lockstep, the frozen-value boundary, complete
fixture workflow, shared validation and errors, host-only adversarial parity, public
surface and executable README pins, and generated or release metadata writers. These
labels are review dimensions, not substitutes for the authoritative details.

Do not dismiss a diff merely because it changes documentation, CI, or comments: package
READMEs are executable, CI can enforce contracts, and canonical documentation can alter
the agreements. Stop early only after inspecting the diff and establishing that no
contract-bearing behavior or workflow is affected.

Report each violation as `file:line — broken contract — concrete fix`. Then list the
applicable contracts that passed, the checks actually run, and any scope or evidence
limitation. If no violation is found, say so explicitly without claiming checks that were
not performed.
