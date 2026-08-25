---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: non-normative
owner: AI-CONFIG.md
document_kind: provider-comparative-meta
---

# AI Configuration Templates

This companion owns non-normative reference templates. The examples illustrate
shapes only and never create requirements. Adapt them to the repository's
selected profiles and present capabilities; the owning normative companion and
[`AI-CONFIG.md`](../AI-CONFIG.md) always take precedence.

## Minimal reference templates

### Compiled-bootstrap owner (`.agents/core/BOOTSTRAP.md`)

```markdown
# Repository agent bootstrap

Canonical project knowledge lives in `docs/` and `.agents/`. Provider-specific files are adapters and are not authoritative.

Before substantive work:

1. Identify the repository root.
2. Invoke the selected context-resolution skill for the current work phase and relevant path.
3. Read every context source returned as required before making affected decisions.
4. Re-run the skill whenever the work mode changes or scope expands to a new component, package, language, interface, or path.
5. Use canonical executable command owners or their optional stable IDs; do not invent replacement workflows.
6. Never edit files marked generated. Edit their canonical source and use the repository's documented generation owner.
7. Run applicable verification before completion and resolve validation-specific context for that phase.
8. State missing context, failed verification, and remaining uncertainty explicitly.
```

For `canonical-router`, keep root and scoped `AGENTS.md` files to scope identity,
resolver activation, required-result reading, rerun behavior, and visible
failure on incomplete resolution. Stable authority, generated-file, command,
and verification rules are delivered through mandatory shared context; do not
copy the full compiled-bootstrap checklist into every router. Omit
`.agents/core/BOOTSTRAP.md` unless it has another real consumer. The router
names the skill but does not duplicate its Python command, phase table, or
argument-selection procedure. Those have one canonical owner: the skill.

### `.agents/skills/<selected-context-resolution-skill>/SKILL.md`

The repository's actual portable skill is the template. It normally defines
activation, work-phase and repository-scope selection, invocation, result
interpretation, rerun behavior, failure behavior, and its required-reading
output contract. It tells agents to consult executable help rather than
copying an option list that can drift. This policy intentionally does not
reproduce the skill body or command line.

### Portable review skill

```markdown
---
name: review-code-change
description: Review a defined code change for correctness, regressions, architecture conformance, security risk, and missing verification. Use when asked to review a diff, branch, pull request, or recently modified files. Do not use for open-ended repository exploration without a defined change set.
metadata:
  owner: engineering
  version: "1.0.0"
  authority: review-only-behavioral
  guard-owner: self
  delegated-guard-owner: caller
  guard-skill: <selected-review-guard>
  guard-lifecycle: top-level-once
---

# Review a code change

## Inputs

- Defined change set: diff, commit range, pull request, or file list.
- Repository root.

## Procedure

1. When invoked directly, apply the selected review guard once at the top-level boundary. When delegated, reuse the caller-owned guard and do not establish another.
2. Invoke the selected context-resolution skill for the review phase and every material path in the change set.
3. Read all returned architecture, rules, decisions, and verification documents.
4. Inspect the diff and enough surrounding code to validate assumptions.
5. Identify correctness, security, compatibility, migration, and test risks.
6. Run or inspect the required verification commands.
7. Rank findings by material impact.

## Validation

Before returning:

- Every finding has file-level evidence.
- Speculation is labeled as uncertainty.
- Style-only comments are omitted unless they affect maintainability materially.
- Executed and unexecuted checks are distinguished.

## Output

Return findings first, ordered by severity. Then list verification gaps and remaining uncertainty.
```

### Neutral rule, when a rule surface exists

```markdown
---
id: generated-file-protection
summary: Generated files must be changed through their canonical source and generator.
severity: required
applies:
  paths:
    - "<declared-generated-root-projection>"
    - ".claude/rules/**"
    - ".claude/agents/**"
    - ".codex/agents/**"
priority: 100
---

# Generated-file protection

Do not edit a file that the selected profile declares generated. Locate its canonical source, update that source, run the AI-config generator, and validate that the generated diff is clean and expected. A canonical `AGENTS.md` router is authored, not generated.
```

### Neutral role

```yaml
# .agents/roles/repository-explorer/role.yaml
schema_version: 1
name: repository-explorer
description: Locate relevant code, docs, rules, and ownership for a bounded technical question without modifying files.
capabilities:
  filesystem: workspace-write
  shell: unrestricted-local
  network: none
  external_side_effects: forbidden
authority:
  inspect_source: allowed
  modify_source: forbidden
  modify_git_state: forbidden
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
parallelism:
  safe: true
  writes: false
output:
  required_sections:
    - answer
    - evidence
    - relevant-paths
    - unresolved-questions
```

### Context-routing test

Fixture schema is repository-owned routing data and is not prescribed here.
Fixtures can cover every phase and scope shape exposed by executable help,
cross-component unions, invalid input, deterministic ordering, duplicate
elimination, and existence of every returned path. Run deterministic cases more
than once and assert identical output.

---
