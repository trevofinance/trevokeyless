# Repository agent instructions

These instructions are provider-neutral. A nested `AGENTS.md` identifies package scope;
canonical implementation detail lives in the documents selected by the shared resolver.

## Resolve context before acting

Read `.agents/skills/context-resolve/SKILL.md` completely and run its resolver for every
touched or proposed path before planning, implementation, review, validation,
documentation, or release work. Read every mandatory result and re-run it whenever the
scope expands. Stop on an unknown path, concern, or unresolved scope.

## Purpose and authority

This repository contains Rust, Python, and TypeScript implementations of Trevo keyless
account encoding. Rust is the behavioral source of truth; the shared conformance fixture
keeps all three implementations in lockstep.

Task authority controls whether work is authoring or review-only. Tool availability does
not expand that authority. Review-only work may create diagnostic artifacts but must use
the `review-guard` skill and introduce no tracked-source, index, ref, HEAD, or external
mutation. Authoring work may update in-scope repository files. Host, organization,
credential, protected-environment, and server-side controls retain higher precedence.

Inspect the worktree before editing, preserve unrelated changes, stage explicit paths,
and never silently revert user work. Provider adapters must not add project semantics or
turn local permission settings into repository authority.

## Sources of truth

- `keyless-rust/` defines encoding and decoding semantics.
- `conformance/vectors.json` is the generated executable cross-language specification.
- `docs/cross-language-contracts.md` defines shared observable behavior.
- `docs/conformance.md` and `docs/conformance-schema.md` define fixture maintenance.
- `docs/rust-implementation.md`, `docs/python-implementation.md`, and
  `docs/typescript-implementation.md` define package ownership and local invariants.
- `.github/workflows/` and their executable owners define current CI behavior.
- Package READMEs and public-surface tests define published consumer APIs.

Do not restate these contracts in provider configuration. Update the canonical owner and
link to it.

## Cross-language invariant

Every observable behavioral change lands Rust-first with matching fixture, Python,
TypeScript, tests, documentation, version, and changelog work in the same change. Stop
for explicit confirmation before changing an existing conformance-vector value; that is
a frozen on-chain encoding boundary. Use documented writers for generated and lock files.

## Portable workflows and adapters

Reusable workflows are defined only under `.agents/skills/<name>/`. Provider adapters
may expose those packages through imports, links, or native discovery, but may not copy
or redefine them. Dependencies point from adapters to the neutral core. The core must
remain useful and valid when every adapter is absent.

Use the closest portable skill for the task. When a skill composes another, activate it
through the host when possible; otherwise read its `SKILL.md` completely and follow it.

## Verification

Select checks from `docs/development.md` and the applicable skill based on the actual
blast radius. Run independent leaf checks separately when practical. Report only commands
actually executed and results actually observed; identify every skipped check and reason.
