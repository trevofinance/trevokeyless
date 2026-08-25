# Repository decisions

This ledger records repository-specific bindings for the immutable AI configuration
policy. Provider mechanics and provider-only limitations belong in their adapters.

## Selected AI configuration profile

Settled 2026-08-24. `.agents/ai-config.toml` selects:

- `canonical-router` because root and package `AGENTS.md` files provide stable neutral
  scope anchors while detailed knowledge stays on demand;
- `review-only-behavioral` because useful review diagnostics require shell and scratch
  writes, with `review-guard` enforcing the tracked-state postcondition;
- `skill-centered` because portable workflows already own orchestration semantics;
- `executable-owner` so reusable check selection and ordering have one implementation;
- `conditional-surfaces` so unused rules, roles, hooks, integrations, and projections
  remain absent; and
- `core-and-adapter-contracts` so a stale adapter cannot invalidate the neutral core.

`context-resolve`, `review-guard`, `AGENTS.md`, `.github/scripts/check_ai_config.py`,
`docs/development.md`, and this ledger are the concrete selected owners.

## Rust-first cross-language authority

Settled before policy adoption and retained. Rust owns observable encoding semantics.
Existing conformance-vector values are frozen on-chain boundaries and require explicit
confirmation to change. Python, TypeScript, fixtures, public surfaces, versions,
changelogs, and canonical documentation move with an observable behavior change.

## Core-first adapter boundary

Settled 2026-08-24. Portable semantics live only in the neutral core. Installed adapters
are removable one-way consumers discovered through `adapter.toml`. Equivalent outcomes,
not identical provider-native files, define parity. The core must validate with every
adapter absent; a stale present adapter fails adapter scope only.

Redundant provider-native agent wrappers are not part of the selected topology. A future
wrapper requires a provider capability that cannot be expressed by a portable skill and
must contain mechanics only.

## Permission and live-evidence posture

Settled 2026-08-24. Repository-owned provider settings may keep broad execution mechanics
needed by the repository, but those settings never grant task authority or bypass host,
organization, credential, protected-environment, or server-side controls.

Live behavior is claimed only by current digest-bound evidence. An unavailable client or
an unexecuted probe is `NOT_RUN`; ambiguous evidence is `INDETERMINATE`. Static success is
not live success.

## External mutation

Settled 2026-08-24. Portable workflows prepare repository changes and release metadata
only. They do not merge, publish, deploy, transfer assets, mutate production data, or use
credentials. No external-enforcement registry is needed while that remains true. Adding
such a capability requires an explicit registry, fail-closed assurance, and live evidence
in the same change.
