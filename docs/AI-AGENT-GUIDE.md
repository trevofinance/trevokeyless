# AI agent guide

This is the operational entry point for AI-assisted work. [AI-CONFIG.md](AI-CONFIG.md)
and its immutable companions define cross-provider policy; [DECISIONS.md](DECISIONS.md)
records this repository's selected profiles and bindings.

## Starting work

1. Inspect the worktree and preserve unrelated changes.
2. Read `.agents/skills/context-resolve/SKILL.md` and resolve every touched or proposed
   path in the intended work mode.
3. Read every mandatory document and re-resolve whenever paths, concerns, or mode expand.
4. Select the closest portable workflow under `.agents/skills/`.
5. Run validation selected from `docs/development.md`; report only observed results.

An unknown path, concern, missing knowledge file, or pathless unresolved task is a hard
stop. The context map is the routing authority; recursively following prose links is not
a substitute.

## Authority and review

Authoring authority permits in-scope tracked changes. Review, audit, or planning authority
is non-authoring even when the host permits shell and workspace writes. A top-level
review establishes one `review-guard` snapshot before diagnostics and verifies it before
the verdict. Delegated review passes reuse the caller-owned guard.

Untracked build, cache, and diagnostic output is allowed. Additional tracked worktree,
index, ref, HEAD, Git-operation, credential, protected-environment, or external mutation
is not. A failing guard prevents an approving verdict and must not be hidden by restoring
possibly overlapping user work.

## Core and adapter contract

The core consists of `AGENTS.md` routers, canonical documentation, `.agents/skills/`,
the conformance/risk data they own, and neutral validation. It must remain complete when
all provider adapters and wrappers are absent.

An adapter is a one-way consumer. Its `adapter.toml` declares discovery mechanics,
support outcomes, static and mutation checks, live-smoke ownership, evidence, and every
adapter-owned path outside its directory. Provider settings control host mechanics only;
they do not override task authority or project semantics.

Optional rules, roles, hooks, integrations, catalogs, generated projections, and external
enforcement registries remain absent until an actual workflow requires them. An absent
surface is preferable to an empty placeholder.

## Live-smoke outcomes

Each supported adapter owns a procedure and a digest-bound evidence record for three
provider-neutral outcomes:

- **R — root context:** a fresh root session automatically receives `AGENTS.md` and can
  identify the shared resolver.
- **N — nested context:** a fresh `keyless-rust/` session receives the root and Rust
  routers in root-to-leaf order without unrelated package routers.
- **S — portable skill:** the session discovers and invokes `context-resolve` from the
  canonical `.agents/skills/` package.

Every probe uses a fresh session. Results are `PASS`, `FAIL`, `NOT_RUN`, or
`INDETERMINATE`; missing clients and unexecuted probes remain `NOT_RUN`. Static checks
never promote a live result. Raw prompts, transcripts, credentials, rollouts, and local
client state stay untracked.

## Git and commits

Stay in the active worktree. Stage explicit paths only and inspect the staged diff before
committing. One logical task gets one commit with an imperative title of at most 70
characters. Creating or pushing tags, publishing packages, merging pull requests, and
other external mutations require separate explicit authority.

## Maintaining configuration

Update shared semantics in the neutral core first. Update an adapter only when its
mechanics must change to expose that core. When routing topology changes, update the
context map and fixtures together. When workflow risk changes, update the registry and
its exact matching evaluation suite together.

The aggregate deterministic owner is `.github/scripts/check_ai_config.py`. Provider live
probes remain separate because repository structure cannot prove client behavior.
