---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: normative
owner: AI-CONFIG.md
document_kind: provider-comparative-meta
---

# AI Context Resolution

This companion owns stable context-resolution requirements and routing lifecycle. The portable skill owns agent-facing invocation, executable help owns exact CLI syntax, and the context map owns repository routing data. See [`AI-CONFIG.md`](../AI-CONFIG.md) for conformance invariants.

## Bootstrap and context resolution

### Full context means a complete context graph

“Full context” does not mean loading all Markdown files into every prompt. It means that every relevant source is:

- canonical;
- indexed or otherwise reachable by the context-resolution skill;
- discoverable from the current work mode and repository path;
- loaded before an agent makes decisions affected by it;
- re-resolved when the work mode or path scope changes.

For large repositories, loading hundreds of kilobytes of documentation at session start is prohibited. It degrades instruction salience, wastes context, and can exceed provider instruction limits.

### Required bootstrap behavior

The selected canonical bootstrap MUST be small and operational. Recommended
target: **under 4 KiB and under 120 lines** per scope router or compiled root.

Under `canonical-router`, the effective bootstrap contract is the active router
plus the mandatory core context selected before substantive work. The router
itself MUST identify its scope, require the selected context-resolution skill, require
the returned mandatory context to be read, require reruns when phase or scope
changes, and fail visibly when resolution is incomplete. The remaining stable
operational rules belong in mandatory shared context rather than being copied
into every scoped router.

The effective bootstrap contract MUST ensure every agent can:

1. identify the repository root;
2. treat canonical documentation, portable skills, and the selected neutral
   bootstrap as authoritative while treating provider trees as adapters;
3. invoke the repository context-resolution skill before substantive analysis or modification;
4. select the resolver mode that matches the current phase of work;
5. run the resolver for the currently relevant repository path and read every returned required source;
6. rerun the skill when the mode changes or when new components, packages, directories, languages, or task types enter scope;
7. use canonical executable command owners rather than inventing build/test commands;
8. apply hard verification requirements before completion;
9. never edit generated adapters directly;
10. report missing, conflicting, stale, or unreachable context rather than silently guessing.

Each router MUST refer to the selected skill by the concrete repository name
bound in the conformance layer. A router MUST NOT copy the resolver command,
option list, phase table, argument cardinality, or examples. Those volatile
details belong to the skill and executable help.

### Context-resolution skill and deterministic script

A repository with more than a small handful of scoped documents MUST package context resolution as a provider-neutral Agent Skill. The canonical package has this shape:

```text
.agents/skills/<selected-context-resolution-skill>/
├── SKILL.md
└── scripts/
    └── resolve_context.py
```

The two files have different responsibilities:

- `SKILL.md` is the **agent-facing interface**. It defines when context resolution is required, how to choose a mode and path, how to interpret the result, and when to rerun the process.
- `scripts/resolve_context.py` is the **deterministic routing engine**. It lists
  the context required for a work phase and repository scope.

Providers MUST discover or project the skill through their normal skill mechanisms. They MUST NOT introduce an independently implemented Claude resolver, Codex resolver, root-level `tools/ai-config/resolve-context`, or provider-specific copy of the routing logic.

`SKILL.md` owns agent-facing invocation semantics: how to select the current
work phase and repository scope, when task or concern metadata is useful, how
to interpret results, and when to rerun. `resolve_context.py --help` owns the
executable option names, accepted values, cardinality, formats, and maintenance
operations. The skill and executable help MUST agree, and automated checks MUST
detect drift.

A router path supplied as scope input is a routing anchor, not a canonical
source of project knowledge. Resolver code MAY use its location to determine
applicable context, but router or adapter prose MUST NOT become the source of
architecture, rules, or procedures. Once concrete affected paths are known, a
root-only resolution is insufficient for work spanning those paths.

An installed provider adapter's immediate-child `adapter.toml` location MAY be
used generically to recognize its directory as a root-scope path anchor. The
resolver MUST NOT parse the manifest for routing policy, name installed
providers in neutral routing data, or require any adapter to exist. Shared
automation paths that own AI configuration, including CI, MUST be registered in
neutral routing data and select their applicable AI-policy concern from the
path alone.

The script MUST:

- resolve paths consistently from the repository root;
- validate the mode and path;
- return a non-zero status on invalid input or incomplete resolution;
- list all required context needed for the supplied mode and path;
- produce deterministic output for the same repository state, mode, and path;
- avoid provider APIs, provider configuration, and hidden network access;
- avoid copying the contents of the returned documents into another canonical store;
- make missing or conflicting routing data visible rather than silently omitting it.

The output MAY be plain text or structured data. Whatever format is chosen, `SKILL.md` MUST explain how the agent distinguishes required context from diagnostics or optional recommendations. Machine-readable output is recommended when hooks, CI, or other scripts consume the result.

Direct script execution is allowed for humans, tests, CI, hooks, and providers that execute the skill procedure literally. It does not replace the skill as the agent-facing abstraction.

### Routing data ownership

This policy does not require a second standalone resolver manifest. Routing data MAY be stored:

- in data files owned by the selected context-resolution skill;
- in concise metadata attached to canonical documents or rules;
- in an optional `.agents/core/context-map.yaml` consumed by the skill;
- in another provider-neutral index consumed exclusively by the same script.

Whatever storage form is used:

1. `resolve_context.py` is the single routing implementation;
2. routing data contains paths, match conditions, priorities, identifiers, and short reasons, not copied documentation bodies;
3. architecture and engineering knowledge remains in `docs/`;
4. behavioral constraints remain in their canonical neutral owner:
   `.agents/rules/` when the repository has selected that surface, otherwise
   the relevant canonical documentation or owning skill;
5. changing routing data requires tests for the affected modes and paths;
6. provider adapters do not maintain separate routing tables.

When `.agents/core/context-map.yaml` exists, it is optional resolver input rather than a policy-mandated parallel source of truth.

### Resolution lifecycle

The agent MUST invoke the context-resolution skill:

- at the start of a task, normally in `understand` mode;
- when moving from understanding to planning, implementation, review, benchmarking, validation, integration, or documentation work;
- after the first repository scan if likely target paths were unknown initially;
- before editing a newly discovered package or component;
- after the user materially changes scope;
- after a subagent returns findings that expand scope;
- before release, migration, deployment, security-sensitive, or data-destructive steps;
- after context compaction when the provider may have discarded loaded details.

A previous resolution for one mode or path MUST NOT be assumed sufficient for another mode or newly affected path.

### Native discovery is secondary

Claude path-scoped rules and nested `CLAUDE.md` loading can improve ergonomics. Codex nested `AGENTS.md` can help when Codex is launched from the relevant subdirectory. Neither mechanism is sufficient as the sole routing system for tasks that span multiple directories or discover new scope after launch.

The context-resolution skill is authoritative. Native discovery may preload some of the same material, but it does not eliminate the required skill invocation.

### Context conflicts

When canonical sources conflict, apply this order unless a higher-level policy says otherwise:

1. non-overridable organization, credential, protected-environment,
   server-side, and security controls;
2. enforced repository policy and explicit hard prohibitions;
3. explicit current user authority and task acceptance criteria;
4. accepted architecture and ADRs, except where the task explicitly authorizes
   changing them;
5. the most specific applicable scoped rule;
6. global neutral rules;
7. examples, templates, and historical notes.

Current task criteria may authorize changing a lower canonical source, but they
do not weaken hard external controls or enforced prohibitions unless the
controlling authority explicitly permits that change.

A more specific rule overrides a general rule only when it explicitly states the exception. Newer text does not automatically override older text; status and supersession metadata decide precedence.

The resolver reports applicable sources; it does not silently resolve semantic conflicts between those sources. The agent MUST surface conflicts that affect the task.

---
