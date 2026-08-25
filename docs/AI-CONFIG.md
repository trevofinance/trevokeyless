---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: normative
owner: self
document_kind: provider-comparative-meta
---

# Provider-Neutral AI Agent Configuration Policy

**Documentation snapshot reviewed:** 2026-08-24
**Applies to:** Repositories that support one or more coding-agent providers

## Purpose and precedence

This policy defines how repository knowledge, portable agent workflows,
context routing, provider adapters, authority, and validation fit together. Its
goal is one maintainable source for each semantic contract with tested behavior
across supported providers.

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative. This entry document owns the cross-cutting invariants and conformance
decision surface. The companion documents linked below own detailed policy by
concern; they do not restate these invariants.

Each repository supplies a conformance layer that binds the policy's semantic
roles—such as the selected context-resolution skill, review guard, bootstrap
owner, and validation owner—to concrete local identifiers. A repository
decision log owns the rationale, alternatives, and history for those bindings.
Portable `SKILL.md` packages own workflow procedure; executable help and
scripts own their runtime interfaces and behavior; provider-owned rationale
files own only provider-specific decisions.

A lower or provider-specific layer MUST NOT override a higher canonical owner.
User, organization, credential, protected-environment, and server-side security
policy may be stricter and MUST NOT be weakened by repository configuration.

## Core invariants

### One canonical owner

Every normative semantic contract, executable interface, routing rule, and
enforcement requirement MUST have exactly one canonical owner. Links, generated
projections, indexes, and thin adapters MAY repeat a pointer or generated
representation; independently maintained normative meanings are forbidden.

Canonical project knowledge belongs in ordinary repository documentation.
Portable procedures belong under `.agents/skills/`. The selected neutral
bootstrap owns the minimum always-on routing contract. Provider adapters MUST
NOT become the only usable source of project architecture, invariants, command
semantics, or workflow policy.

### One-way dependency direction

The allowed semantic dependency direction is:

```text
provider adapter  ─────►  neutral agent core  ─────►  docs and product truth
```

Provider adapters MAY consume neutral routers, skills, executables, and
documentation. The neutral core MUST NOT depend on a provider adapter, and one
provider adapter MUST NOT depend on another. The core MUST remain valid and
useful when every provider adapter and wrapper is absent.

A router location may be passed to neutral routing code solely as a repository
scope anchor. That does not authorize the resolver to consume router or adapter
prose as routing policy.

The location of an immediate-child `adapter.toml` MAY identify an installed
adapter directory as a generic root-scope anchor. This is discovery metadata,
not a semantic dependency: neutral routing must neither parse adapter content
nor require an adapter to be installed, and its fixtures must remain complete
without provider trees.

Provider-comparative policy documents may declare
`document_kind: provider-comparative-meta` as defined in
[`DOCUMENTATION.md`](DOCUMENTATION.md#provider-neutral-semantics). This permits
provider names and illustrative trees, but never a machine-resolvable link,
instruction import, or symlink from neutral content into an adapter.

### Semantic parity

Supported providers MUST implement the same normative outcomes, authority
boundaries, required context, and output contracts. They do not need identical
files or native mechanisms. Behavioral parity is the conformance target;
byte-for-byte configuration parity is not.

A provider-only enhancement MAY remain adapter-local only when it is
nonnormative, optional, contains no unique project knowledge, does not reduce
correctness when absent elsewhere, and is declared by the adapter.

### Deterministic requirements

Prose is guidance, not a security or correctness boundary. Deterministic
requirements MUST be enforced by executable checks, sandbox or credential
controls, hooks, CI, protected environments, server-side authorization, or a
tested postcondition appropriate to the risk.

## Conformance profiles

Every repository MUST select and record one bootstrap profile plus any
applicable authority, delegation, command, capability-surface, and validation
profiles in one machine-readable conformance manifest. That manifest owns the
selected values and concrete canonical identifiers. The repository decision
log owns rationale, alternatives, and history; validators consume the manifest
instead of parsing prose or inferring filesystem shape. Installed providers
remain discoverable through their own adapter manifests and MUST NOT be copied
into the central selection. Detailed requirements and topology guidance live
in [`PROFILES.md`](ai-config/PROFILES.md).

### Bootstrap

`canonical-router` uses short root and scoped `AGENTS.md` files as
provider-neutral canonical routers. They contain no provider-only settings,
commands, hooks, tool names, or permissions. A provider may discover them
natively or through a thin wrapper. Deterministic context resolution covers
cross-scope work and scope discovered after launch.

`compiled-bootstrap` uses a neutral bootstrap at another canonical path and
generates or mechanically derives provider root projections. Choose it only
when divergent provider root formats or an established generation pipeline
provide real value.

Repository size does not choose the profile. The recorded owner, deterministic
routing behavior, and absence of independently authored duplicates do.

### Capability-conditional surfaces

Always required are:

- canonical documentation and the portable skills actually used;
- one selected bootstrap profile;
- deterministic context resolution for a large or multi-scope repository;
- unique semantic ownership and one-way dependency checks;
- adapter identity/discovery for each installed provider;
- core-independence, validation, and conformance entry points.

Neutral rules, roles, shared hooks, capability catalogs, command-ID indexes,
integration registries, generated projections, provider project settings, and
their schemas are required only when the corresponding capability exists. A
repository MUST NOT add empty directories or placeholder manifests merely to
match a reference architecture.

When required cross-provider behavior exists, it MUST have a neutral owner and
either a provider projection or an explicitly tested fallback for every
supported provider.

## Authority, capability, and enforcement

Configuration MUST model three independent dimensions:

| Dimension | Question | Example |
|---|---|---|
| Task authority | What did the user authorize? | Inspect and propose are allowed; tracked-source, Git-state, and external mutation are forbidden. |
| Workspace capability | What can the environment technically do? | Workspace writes and unrestricted local diagnostics are available. |
| Enforcement assurance | How is each prohibition guaranteed? | Tracked-state postcondition, sandbox, credentials, protected environment, or server-side approval. |

**Review-only authority** is non-authoring task authority. It is not a claim
that the filesystem or tools are hard read-only. A diagnostic-write-capable
reviewer MAY run compilers, tests, analyzers, and generators that create
untracked caches, build output, or temporary files. It MUST NOT intentionally
modify tracked source or Git state, apply fixes, or perform external actions.

When review execution can write to the workspace, its review-only authority
MUST be enforced by a tracked source and Git-state postcondition bound to the
selected review guard. The detailed guard declaration and lifecycle contract is
owned by
[`SKILLS-AND-DELEGATION.md`](ai-config/SKILLS-AND-DELEGATION.md#agent-roles-and-subagents).

Reserve **hard read-only** for technical controls that prevent writes.
Production deployment, credential access, irreversible database or chain
operations, destructive cloud actions, publishing, merging, releasing, asset
transfers, and equivalent external security boundaries MUST be hard-enforced;
behavioral instruction and after-the-fact detection are insufficient.

Repositories that expose an external mutation workflow MUST inventory its task
authority, security classification, required assurance, current evidence
status, and fail posture. A requirement for hard enforcement is not evidence
that the control exists. When the required control cannot be verified, the
workflow MUST fail closed and conformance MUST report **unverified**, not pass.
`not-exposed` is valid only when no portable workflow can execute the boundary.

## Context resolution

A large or multi-scope repository MUST provide one selected provider-neutral
context-resolution skill backed by one deterministic routing implementation.
Routers and consuming workflows refer to that owner instead of copying its
interface. Resolver ownership, invocation, lifecycle, routing-data, and fixture
requirements live only in
[`CONTEXT-RESOLUTION.md`](ai-config/CONTEXT-RESOLUTION.md).

## Skills and delegation

Portable skills own reusable procedure, criteria, approval gates, and output
contracts. Provider activation and presentation remain adapter concerns.
Detailed package, utility, evaluation, rule, reviewer-catalog, delegation,
guard, isolation, and parallel-write requirements live only in
[`SKILLS-AND-DELEGATION.md`](ai-config/SKILLS-AND-DELEGATION.md).

## Commands and validation ownership

Every command MUST have exactly one executable owner: a script, task-runner
target, package script, or build-system target. Human validation documentation
owns selection, prerequisites, sequencing, restrictions, and expected results.

When several aggregate or pre-commit runners share check selection, that
selection is executable behavior and MUST also have one executable owner.
Runners and skills consume named selection profiles; they MUST NOT maintain
independent discovery loops or current check inventories.

A structured command index is optional and is required only when a real
consumer needs stable command IDs. It points to executable owners and MUST NOT
copy internal shell commands.

Detailed command, check, CI, conformance-report, and maintenance requirements
live in [`VALIDATION.md`](ai-config/VALIDATION.md).

## Provider adapters

Each installed adapter MUST declare identity and deterministic entry points in
a versioned manifest discoverable by generic tooling. It owns only native
discovery, wrappers, settings, permission mappings, hook bindings, optional
worker projections, provider-local ergonomics, and live-smoke mechanics.

Adapters MUST be thin one-way consumers. An installed adapter may become stale
after a core change and fail its own contract without changing the core
verdict. Generated adapter artifacts, when used, SHOULD be committed unless
generation is guaranteed before every local, cloud, CI, IDE, and agent session.

Detailed interoperability, hooks, integrations, capability mapping, provider
projection, additional-provider, and generation rules live in
[`ADAPTERS.md`](ai-config/ADAPTERS.md).

## Validation requirements

Validation MUST distinguish provider-neutral core validity, adapter-boundary
validity, each installed adapter's contract, conditional capability checks,
and live client evidence. The detailed checker, CI, core-independence,
secret-scanning, evidence-recording, and maintenance requirements live only in
[`VALIDATION.md`](ai-config/VALIDATION.md). The high-risk taxonomy and behavior
evaluation corpus contract are owned by
[`SKILLS-AND-DELEGATION.md`](ai-config/SKILLS-AND-DELEGATION.md#skill-evaluation-corpus);
validation consumes that contract without redefining it.

## Concise conformance checklist

A repository conforms when all of the following are true:

- [ ] One machine-readable manifest owns the bootstrap and all repository-specific profile choices and canonical identifiers.
- [ ] Every semantic contract has one canonical owner.
- [ ] Provider adapters depend only toward the neutral core and product truth.
- [ ] The neutral core passes with every provider adapter absent.
- [ ] Comparative policy prose has no machine-resolvable edge into an adapter.
- [ ] Routers stay short and require the canonical resolver skill without copying its CLI.
- [ ] The resolver skill, executable help, routing data, and fixtures have distinct owners and agree.
- [ ] Canonical skills use the portable package contract and contain no provider-only syntax.
- [ ] Delegation parity is behavioral and tested; native worker files are conditional.
- [ ] Authority, workspace capability, and enforcement assurance are explicit and separate.
- [ ] Every review-only entrypoint binds exactly one selected guard owner for direct and delegated execution.
- [ ] Diagnostic-write-capable review enforces its tracked-state postcondition.
- [ ] External security boundaries are hard-enforced outside prose.
- [ ] Every exposed external mutation has current non-secret assurance evidence, and an unverified hard boundary fails closed.
- [ ] Every command has one executable owner and no copied command catalog body.
- [ ] Shared aggregate and pre-commit selection has one executable owner.
- [ ] Unused configuration capabilities have no empty scaffolding.
- [ ] Every installed adapter has identity, per-workflow support status, static validation, a mutation contract, a live-smoke owner, and current evidence.
- [ ] Remote CI runs core, adapter, secret, and dependency-direction gates.
- [ ] Surface-specific checks run whenever their corresponding surface exists.
- [ ] High-risk workflows are explicitly classified, exactly matched to integration suites, and cover activation, composition, authority, output, and side effects; skill-local suites remain separate.
- [ ] Local core-independence checks exercise the current patch rather than only the previous `HEAD`.
- [ ] Live evidence records current per-outcome status without promotion by static checks.
- [ ] Local state, transcripts, credentials, and secrets remain untracked.
- [ ] Validation evidence distinguishes passed, failed, not-run, and indeterminate outcomes.

## Companion ownership and routing

Load only the companion relevant to the configuration task:

| Concern | Detailed canonical owner |
|---|---|
| Bootstrap topology, physical profiles, and content placement | [`PROFILES.md`](ai-config/PROFILES.md) |
| Resolver lifecycle, ownership, and routing behavior | [`CONTEXT-RESOLUTION.md`](ai-config/CONTEXT-RESOLUTION.md) |
| Portable skills, workflow-risk and eval schemas, rules, reviewer catalogs, guards, and delegation | [`SKILLS-AND-DELEGATION.md`](ai-config/SKILLS-AND-DELEGATION.md) |
| Provider adapters, hooks, integrations, permissions, per-workflow support status, and generation | [`ADAPTERS.md`](ai-config/ADAPTERS.md) |
| Command ownership, deterministic checks, CI, conformance evidence, and policy maintenance | [`VALIDATION.md`](ai-config/VALIDATION.md) |
| Migration sequence only; normative behavior links to its owning companion | [`MIGRATION.md`](ai-config/MIGRATION.md) |
| Examples only; never a normative source | [`TEMPLATES.md`](ai-config/TEMPLATES.md) |
| Official external sources | [`REFERENCES.md`](ai-config/REFERENCES.md) |

Normal application development SHOULD NOT load these companions merely because
an AI agent is performing the work. Resolver routing MUST select them only for
the corresponding agent-configuration concern.

## Final policy statement

Provider-neutral configuration is a dependency and ownership discipline, not a
requirement to build every possible configuration surface. Keep knowledge and
portable procedures canonical, select an explicit bootstrap, route context
progressively, project only capabilities that exist, keep adapters thin, model
authority separately from capability, and prove the resulting behavior in
deterministic checks and clean-session evaluations.
