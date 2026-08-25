---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: normative
owner: AI-CONFIG.md
document_kind: provider-comparative-meta
---

# AI Skills and Delegation

This companion owns portable-skill, behavioral-rule, specialist-procedure, delegation, and parallelism policy. See [`AI-CONFIG.md`](../AI-CONFIG.md) for the authority/capability/enforcement distinction and selected conformance profiles.

## Agent Skills policy

### Canonical location

All portable repository skills MUST live at:

```text
.agents/skills/<skill-name>/SKILL.md
```

Codex consumes this location natively. Claude consumes the same directory through per-skill symlinks or generated mirrors under `.claude/skills/`.

### Strict portable profile

Canonical `SKILL.md` frontmatter MUST use only Agent Skills specification fields:

- `name` — required;
- `description` — required;
- `license` — optional;
- `compatibility` — optional;
- `metadata` — optional string-to-string map;
- `allowed-tools` — part of the specification but experimental.

This policy **forbids `allowed-tools` in canonical skills by default**, because tool identifiers and approval semantics are provider-specific. It MAY be enabled only through a documented compatibility decision covering every supported provider.

Canonical skills MUST NOT use Claude-only or Codex-only frontmatter such as:

- `context`;
- `agent`;
- `background`;
- `disable-model-invocation`;
- `user-invocable`;
- `argument-hint`;
- `arguments`;
- provider-native model, tool, sandbox, or permission fields.

Canonical skill bodies MUST NOT depend on provider-specific expansion or environment syntax, including:

- Claude dynamic command injection using ``!`command` ``;
- `$ARGUMENTS`, `$1`, or provider-only argument placeholders;
- `${CLAUDE_*}` variables;
- provider-native slash-command behavior;
- a provider-specific tool name as the only way to perform a step.

Use explicit prose, portable script interfaces, and relative file paths instead.

### Naming

A skill name MUST:

- contain 1–64 lowercase alphanumeric characters or hyphens;
- not begin or end with a hyphen;
- not contain consecutive hyphens;
- match its parent directory name;
- be unique within the effective repository skill catalog.

Prefer verb-object or capability names, for example:

- `context-resolve`;
- `review-runtime-change`;
- `generate-json-schema-types`;
- `prepare-release`.

### Description quality

The description is routing metadata. It MUST state:

1. what the skill does;
2. when it should be used;
3. important trigger words or artifacts;
4. when it should not be used, when ambiguity is likely.

Good:

```yaml
description: Review a Rust runtime change for architecture conformance, storage migration safety, weight changes, and required verification. Use for changes under runtime/ or pallets/, or when the task mentions a runtime upgrade, storage version, benchmark, or migration. Do not use for pure off-chain service changes.
```

Weak:

```yaml
description: Helps review code.
```

### Skill scope

A skill SHOULD represent one coherent reusable capability, comparable to a well-scoped function.

Do not create:

- a skill for a single one-line reminder;
- a universal “do all engineering work” skill;
- a skill that merely duplicates generic model knowledge;
- a skill that is actually a persistent rule;
- a skill that is actually a specialized agent identity.

### Skill structure

Recommended structure:

```text
skill-name/
├── SKILL.md
├── scripts/
├── references/
├── assets/
└── evals/
    ├── evals.json
    └── files/
```

`SKILL.md` SHOULD contain:

1. purpose and activation boundary;
2. required inputs and assumptions;
3. procedure;
4. decision points;
5. validation loop;
6. failure and escalation behavior;
7. output contract;
8. explicit links to optional resources and when to read them.

Keep `SKILL.md` under **500 lines** and preferably under **5,000 tokens**. Move detailed reference material into focused files loaded on demand.

### References and assets

- Skill-specific reference material MAY live under `references/`.
- General project architecture MUST remain in `docs/`; do not copy it into a skill.
- A repository-scoped skill MAY link directly to canonical repository docs using a relative path.
- A distributable skill MUST be self-contained or clearly declare external prerequisites.
- Templates and static output material belong under `assets/`.
- Reference chains SHOULD be one hop from `SKILL.md`; avoid “read A, which points to B, which points to C.”

### Scripts

Every skill that contains Python files under `scripts/` MUST include a
skill-local `scripts.toml` manifest. The manifest declares standalone CLI
entrypoints separately from imported helpers and the unit tests that cover
each helper:

```toml
schema_version = 1
entrypoints = ["scripts/resolve_context.py"]

[[helper]]
path = "scripts/routing_model.py"
tests = ["scripts/test_routing_model.py"]
```

Every Python file under `scripts/` MUST have exactly one declared role as an
entrypoint, helper, or helper test. A helper test MAY cover more than one
helper. Declared entrypoints MUST:

- be non-interactive unless interaction is the purpose of the skill;
- expose `--help` or equivalent usage without performing operational work;
- reject unknown options and invalid input with a non-zero result;
- return non-zero on failure;
- write diagnostics to stderr and structured results to stdout where practical;
- validate inputs;
- avoid silent partial success;
- document dependencies;
- avoid relying on the caller's current directory when the skill root or repository root can be resolved explicitly;
- be deterministic and idempotent where possible;
- avoid hidden network access;
- never print secrets;
- produce helpful errors for agents and suppress raw tracebacks for expected
  command-line or input errors.

Helpers are not CLI entrypoints and MUST NOT be required to expose `--help`.
They MUST be safe and silent to import, perform no operational filesystem,
process, or network action at import time, avoid secrets, behave
deterministically, and name at least one executable unit-test file in the
manifest. Deterministic validation imports each helper repeatedly under an
operational-side-effect audit and executes every declared helper test.

Prefer JSON output when another agent step must consume the result. For the
selected context-resolution skill, a deterministic plain-text or Markdown list
of repository-relative context paths is also conforming when `SKILL.md` defines
how required entries, optional entries, and diagnostics are distinguished.

### Operational invocation belongs in the skill; provider activation belongs in adapters

A portable `SKILL.md` owns the provider-neutral operational contract for its capability, including:

- the canonical script or command to execute;
- portable arguments and how to select them;
- result interpretation;
- retry, rerun, and failure behavior;
- the validation loop.

A provider adapter owns only provider-native activation and presentation concerns, including:

- whether the skill may activate implicitly;
- UI metadata;
- provider-specific tool declarations;
- approval or permission wiring;
- discovery projections or symlinks.

The selected context-resolution skill is a specialized instance of this
portable ownership model. Its lifecycle, ownership split, and consumer contract
are defined only in
[`CONTEXT-RESOLUTION.md`](CONTEXT-RESOLUTION.md#bootstrap-and-context-resolution).

For a production-deployment skill, the neutral description and body may say that explicit user authorization is required. The provider adapter should also disable implicit invocation. The actual deployment permission must still be enforced by credentials, approvals, or hooks.

### Claude discovery adapter

Expose each canonical skill through a per-skill link from
`.claude/skills/<name>` to `.agents/skills/<name>`.

Per-skill symlinks are preferred because both the target and conflict are explicit. If symlinks are unsuitable for the repository's supported platforms, generate read-only mirrors and validate their source digest.

Do not maintain a second authored `.claude/skills/<name>/SKILL.md`.

### OpenAI sidecar

A skill MAY include `agents/openai.yaml` for OpenAI/Codex UI metadata, activation policy, or provider-specific tool dependencies. This file is a provider adapter sidecar and MUST NOT contain unique workflow instructions or duplicate the operational invocation contract from `SKILL.md`.

### Skill evaluation corpus

The primary risk-inventory unit is `workflow_id`, not an individual skill. A
workflow may compose entrypoint skills, specialist skills, roles or native
workers, a review guard, adapter permission mappings, hooks, and external
commands or services. The inventory MUST classify that composition through an
explicit provider-neutral taxonomy. “Important” and “frequently used” are not
sufficient classifications by themselves. A workflow is high risk when it owns
at least one boundary such as:

- mandatory context or configuration authority;
- a user-approval transition;
- intentional tracked authoring or local commit creation;
- coordination of a top-level change-set review verdict;
- durable external mutation or a hard security boundary; or
- unusually resource-heavy generation that authors tracked artifacts.

When high-risk workflows exist, a machine-readable neutral inventory MUST
define the risk classes and record, for each `workflow_id`:

- entrypoint skills;
- participating skills and any role or worker mapping;
- the selected guard and lifecycle when applicable;
- risk classes and rationale; and
- exactly one repository-relative integration-suite path.

Workflow integration suites live under
`.agents/evals/workflows/<workflow-id>/evals.json`. The repository extension
schema is versioned independently from the Agent Skills format. Each case has:

- `case_type`: `should-trigger`, `ambiguous`, `should-not-trigger`, or
  `unauthorized-action`;
- a realistic `prompt`;
- `expected_activation.activate` and `expected_activation.do_not_activate`
  skill lists;
- `expected_outputs`;
- `allowed_side_effects`; and
- `forbidden_side_effects`.

Deterministic validation MUST require all four case types and an exact
inventory-to-workflow-suite match in both directions. Adding, removing, or
materially changing an entrypoint, participant, guard, risk class, or workflow
suite requires reviewing the inventory in the same change.

Skill-local `evals/evals.json` remains the Agent Skills-aligned unit for one
skill's activation precision and output quality. It contains `skill_name`,
realistic prompts, expected outputs, optional fixture files, and objective
activation, output, and side-effect assertions. A skill-local suite does not by
itself prove delegation, authority, guard lifecycle, adapter behavior, or the
side effects of a composed workflow.

Evaluate:

1. activation precision;
2. output correctness;
3. output usefulness;
4. token and time overhead;
5. behavior with and without the skill;
6. behavior in each supported provider for critical skills.

Mechanical assertions should be checked by scripts rather than model judgment.
Deterministic routing engines, parsers, and other utilities retain native
fixtures and CLI tests even when their owning workflow also has an evaluation
suite; prompt cases do not replace deterministic behavior coverage.

Use three separate evidence layers:

```text
skill-local eval
    Tests one skill's activation and output quality.

workflow integration eval
    Tests composition, delegation, authority, guards, and side effects.

provider live smoke
    Tests actual client discovery and execution.
```

Corpus conformance and behavioral execution are separate evidence. A live
provider run evaluates the cases against an actual model or client and records
the provider, client version, date, result, and non-secret evidence summary. A
schema-valid corpus is not proof that a live behavior run passed.

### Validation

CI MUST run a conformant Agent Skills structure validator for every skill, plus
repository-specific checks for forbidden provider fields and syntax. A
repository MAY use `skills-ref` or an equivalent executable owner.

---

## Behavioral rules and instructions

### Neutral rules are not provider rule files

When a repository needs a dedicated provider-neutral rule surface, canonical
behavioral rules live under `.agents/rules/`. Otherwise the constraint remains
in canonical documentation or its owning skill. Provider-specific behavioral
rule files are projections, never independent semantic owners.

### Neutral rule format

Recommended format:

```markdown
---
id: rust-error-boundaries
summary: Preserve typed error boundaries in Rust production code.
severity: required
applies:
  paths:
    - "**/*.rs"
  task_types:
    - implementation
    - refactor
    - review
  exclude_paths:
    - "**/fixtures/**"
priority: 60
---

# Rust error boundaries

- Do not replace typed domain errors with unstructured strings at public boundaries.
- Add context at integration boundaries without discarding the original source.
- Tests may use convenience assertions when production behavior remains typed.

Rationale and examples: [Rust error architecture](../../docs/development/rust-errors.md).
```

Required metadata:

- stable `id`;
- short `summary`;
- applicability;
- severity;
- priority when overlaps are possible.

### Rule content

A rule SHOULD contain concise behavioral statements. Detailed rationale, diagrams, historical decisions, and examples belong in `docs/`.

A rule MUST be testable or at least reviewable. Avoid vague text such as “write good code” or “follow best practices.”

### Always-on versus scoped

- Truly universal rules may be summarized in the selected canonical bootstrap.
- Language-, package-, directory-, or task-specific rules MUST remain scoped.
- A rule that applies only during a procedure usually belongs in that skill instead.
- A rule that must be mechanically guaranteed belongs in a hook, linter, test, permission, or CI gate.

### Claude projection

The generator MAY project neutral rules into `.claude/rules/*.md` and translate `applies.paths` into Claude's native `paths` frontmatter.

The generated Claude rule MUST retain a source marker and MUST NOT add new semantic content except provider-specific loading instructions.

### Codex projection

Do **not** project behavioral rules into `.codex/rules/*.rules`. Codex `.rules` files govern command execution outside the sandbox.

For Codex, scoped behavioral rules are delivered through:

1. the context-resolution skill;
2. the selected root `AGENTS.md` bootstrap, canonical or generated according to profile;
3. optional hook-based context augmentation;
4. nested `AGENTS.md` only for directories that are genuine independent launch roots.

### Nested instruction files

Nested provider instruction files SHOULD NOT be the canonical source of component knowledge.

They MAY exist when a subdirectory is a separately operated project that agents routinely launch from directly. Even then, the file must be generated from neutral rules or contain only a pointer that requires invocation of the canonical context-resolution skill.

### Conflicts

CI SHOULD detect:

- overlapping rules with contradictory imperatives;
- duplicate rule IDs;
- always-on rules that conflict with scoped rules without an explicit override;
- provider-generated projections that differ from canonical source;
- obsolete rules referencing removed paths.

---

## Agent roles and subagents

### Role versus skill

A **skill** defines how to perform a reusable capability in the current agent context.

A **role** defines a specialized delegated worker with its own objective, operating constraints, tool/capability surface, and output contract.

### Neutral reviewer catalogs

A neutral reviewer catalog MAY own stable orchestration metadata only:
specialist ID, canonical skill, activation condition, required status,
authority class, parallel safety, and output-contract reference. It MUST NOT
duplicate specialist procedure, project knowledge, or provider execution
mechanics. Provider adapters consume this mapping without becoming an
independent reviewer roster.

Create a neutral role package only when a distinct identity or permission profile
contains semantics beyond a portable skill plus stable orchestration metadata.
Isolation, parallel investigation, or narrow expertise alone may be provided by
host-native delegation. Do not create a role merely to package reference
knowledge or to force identical provider files; use a skill.

### Conditional canonical role package

When a cross-provider role capability actually requires a neutral role source,
use a package such as:

```text
.agents/roles/code-reviewer/
├── role.yaml
└── ROLE.md
```

Example `role.yaml`:

```yaml
schema_version: 1
name: code-reviewer
description: Review a defined change set for correctness, regressions, security, architecture conformance, and missing verification.
activation:
  task_types:
    - review
  use_proactively_after_code_change: true
capabilities:
  filesystem: workspace-write
  shell: unrestricted-local
  network: none
  external_side_effects: forbidden
authority:
  inspect_source: allowed
  propose_changes: allowed
  modify_source: forbidden
  modify_git_state: forbidden
  external_actions: forbidden
enforcement:
  tracked_state:
    kind: postcondition
    owner_skill: <selected-review-guard>
    lifecycle: top-level-once
    delegated_behavior: reuse-caller-guard
    direct_behavior: establish-own-guard
  external_actions: credentials-and-sandbox
entrypoint:
  guard_owner: self
required_skills:
  - <selected-context-resolution-skill>
  - review-code-change
parallelism:
  safe: true
  writes: false
output:
  format: markdown
  required_sections:
    - findings
    - verification-gaps
    - uncertainties
    - evidence
```

`ROLE.md` contains the canonical role instructions:

```markdown
# Objective

Review only the assigned change set. Prioritize correctness and material risk over style.

# Procedure

1. Resolve repository context for every touched component.
2. Read the diff and the surrounding implementation.
3. Check architecture and accepted ADRs.
4. Run or inspect the required verification.
5. Report findings by severity with exact file references.

# Boundaries

- Do not modify files.
- Do not broaden scope without stating why.
- Do not report speculative issues as confirmed defects.

# Output contract

Return findings first. For each finding include severity, evidence, impact, and a concrete fix direction.
```

### Abstract capabilities

Canonical roles, when present, MUST use provider-neutral capability names, not
provider tool names. Capability does not imply authority.

Recommended vocabulary:

- `filesystem: none | read | workspace-write`;
- `shell: none | hard-read-only | workspace-write | unrestricted-local`;
- `network: none | allowlisted | unrestricted`;
- `mcp: [logical-server-id, ...]`;
- `external_side_effects: forbidden | approval-required | allowed`;
- `secrets: [logical-secret-id, ...]`;
- `destructive_operations: forbidden | approval-required`.

Provider mappings translate these into tools, sandbox modes, permission rules, and MCP access.

### Provider execution choices

When a neutral role owns required semantics, provider-specific custom-agent
definitions MUST be generated or kept as validated thin projections of that
role. When a portable skill and catalog fully express the behavior, a provider
MAY instead use built-in delegation or current-agent execution.

Claude projection example:

```markdown
---
name: code-reviewer
description: Review a defined change set for correctness, regressions, security, architecture conformance, and missing verification.
tools: Read, Grep, Glob, Bash, Skill
model: inherit
permissionMode: default
skills:
  - <selected-context-resolution-skill>
  - review-code-change
---

<!-- Compiled from .agents/roles/code-reviewer/ROLE.md -->
[compiled canonical ROLE.md content]
```

Codex projection example:

```toml
name = "code-reviewer"
description = "Review a defined change set for correctness, regressions, security, architecture conformance, and missing verification."
sandbox_mode = "workspace-write"
developer_instructions = """
Compiled from .agents/roles/code-reviewer/ROLE.md.

[compiled canonical ROLE.md content]
"""
```

The exact provider fields may change. The generator owns that mapping.

A native tool allowlist is not sufficient to claim hard read-only behavior: a
shell tool can still mutate files, Git state, or external systems. A
diagnostic-write-capable reviewer MUST be described as review-only and guarded
once at its top-level execution boundary by the tracked-state postcondition.
Delegated passes reuse a caller-owned guard; an independently invoked pass
establishes its own. A portable skill with this authority SHOULD declare
`metadata.authority: review-only-behavioral` so deterministic validation can
reject an unguarded entry point. If a role declares `hard-read-only`, its
adapter MUST map that claim to tested technical controls. Native plan or bypass
mode names are capability details, not authority definitions.

Every review-only entrypoint MUST bind the selected review guard explicitly.
The neutral role schema uses `entrypoint.guard_owner: self | caller`. Portable
skill frontmatter uses the equivalent string metadata:

```yaml
metadata:
  authority: review-only-behavioral
  guard-owner: self
  delegated-guard-owner: caller
  guard-skill: <selected-review-guard>
  guard-lifecycle: top-level-once
```

`self` means a direct invocation establishes one top-level guard. `caller`
means the entrypoint is not independently invokable and MUST name an already
guarded caller. A directly invokable specialist therefore owns its guard when
called alone and reuses the caller's guard when delegated. An execution MUST
have exactly one guard owner: missing ownership, an unguarded caller reference,
and nested or duplicate self-owned guards are configuration errors.

### Allowed provider overlays

A provider overlay MAY specify:

- model or reasoning effort;
- native tool allow/deny lists;
- native sandbox or permission mode;
- invocation/delegation hints;
- provider-specific MCP binding;
- provider-specific lifecycle behavior.

It MUST NOT restate the role's objective, procedure, project facts, or output contract except as generated content.

### Context in subagents

A subagent MUST NOT assume that it inherits all parent context. Every generated role must either:

- preload the required portable skills and core context using the provider's supported mechanism; or
- follow the subagent context contract owned by
  [`CONTEXT-RESOLUTION.md`](CONTEXT-RESOLUTION.md#resolution-lifecycle).

This is especially important for isolated/forked agents.

### Parallelism and write ownership

Delegated reviewers default to review-only authority. This does not imply a
hard read-only workspace.

Parallel write agents are allowed only when:

- their file ownership is disjoint and declared;
- they operate in separate worktrees or equivalent isolated workspaces; or
- a single coordinator serializes integration.

Two agents MUST NOT concurrently modify the same file or generated artifact without an explicit merge protocol.

The primary agent owns final integration, verification, and conflict resolution.

### Role quality

A good role is narrow and opinionated. It must define:

- objective;
- activation boundary;
- inputs;
- required context and skills;
- capabilities and forbidden actions;
- procedure;
- output contract;
- verification expectations;
- handoff behavior;
- parallel-safety status.

---
