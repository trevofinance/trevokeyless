# AI configuration implementation

The immutable policy is [AI-CONFIG.md](AI-CONFIG.md). This document maps that policy to
the repository's concrete owners.

```text
provider adapter -> provider-neutral core
```

## Neutral core

| Concern | Owner |
|---|---|
| Repository authority and scope activation | Root and nested `AGENTS.md` |
| Deterministic context | `.agents/skills/context-resolve/` |
| Review tracked-state postcondition | `.agents/skills/review-guard/` |
| Portable workflows | `.agents/skills/<name>/` |
| Repository profile selections | `.agents/ai-config.toml` |
| Operational guidance and decisions | `docs/AI-AGENT-GUIDE.md`, `docs/DECISIONS.md` |
| Product and package semantics | Canonical docs, package sources, fixtures, and tests |
| Deterministic conformance | `.github/scripts/check_ai_config.py` and neutral helpers |

Core files never import an adapter, require a provider CLI, or name provider-owned paths
as project authority. Aggregate validation may discover installed manifests generically;
core validation remains runnable when none exist.

## Adapter contract

Every installed immediate-child provider directory owns an `adapter.toml` with a version,
identifier, core entry, canonical skill source, support outcomes, static check, mutation
contract, live-smoke procedure, evidence path, boundary signature, and a closed inventory
of adapter-owned paths outside its directory.

An adapter may import routers, expose canonical skills, and configure native model,
reasoning, permissions, UI, discovery, or delegation. It must not copy a portable skill,
redefine repository behavior, or make core validity depend on its presence. Provider
settings are execution mechanics and never an authority source.

Equivalent outcomes are required for root context, nested context, portable-skill
discovery, and every workflow the manifest declares. A native capability may use a tested
fallback or be declared limited; absence and ambiguity fail closed rather than becoming
implicit support.

## Lifecycle

1. Change neutral semantics first.
2. Update context/risk data and their fixtures with the owning behavior.
3. Update adapters only when exposure mechanics change.
4. Run core validation independently, then each installed adapter contract.
5. Invalidate live evidence whenever a selected router, skill, adapter setting, client
   version, or boundary input changes.
6. Verify the complete working patch with adapters removed before declaring core
   independence.

Optional rules, roles, hooks, integrations, catalogs, schemas, projections, and external
enforcement exist only after a real requirement selects them.
