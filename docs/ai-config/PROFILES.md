---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: normative
owner: AI-CONFIG.md
document_kind: provider-comparative-meta
---

# AI Configuration Profiles and Content Placement

This companion owns detailed bootstrap topology, capability-surface placement, and canonical documentation rules. The cross-cutting invariants and profile-selection requirements remain in [`AI-CONFIG.md`](../AI-CONFIG.md).

## Repository conformance selection

Each repository MUST keep one small machine-readable manifest that binds the
generic policy to the selected profiles and concrete local owners. It owns
values and identifiers only; rationale and history remain in the repository's
decision log. Validators MUST consume this manifest rather than parse prose,
infer selections from absent directories, or hardcode repository choices.

At minimum the manifest records:

- schema and policy-bundle versions;
- bootstrap, authority, delegation, command, capability-surface, and validation
  profiles;
- the selected context-resolution skill and review guard;
- bootstrap and validation owners; and
- the decision log that explains the selection.

Example:

```toml
schema_version = 1
policy_bundle = "provider-neutral-ai-config"
policy_version = "<policy-version>"

bootstrap_profile = "canonical-router"
authority_profile = "review-only-behavioral"
delegation_profile = "skill-centered"
command_profile = "executable-owner"
capability_surface_profile = "conditional-surfaces"
validation_profile = "core-and-adapter-contracts"

context_resolution_skill = "<selected-context-resolution-skill>"
review_guard_skill = "<selected-review-guard>"
bootstrap_owner = "<repository-relative-bootstrap-path>"
validation_owner = "<repository-relative-validation-entrypoint>"
validation_guidance = "<repository-relative-validation-doc>"
decision_log = "<repository-relative-decision-log>"
```

Installed adapters remain individually discoverable through their own
`adapter.toml` files. Do not duplicate their identities in this manifest.

## Reference repository layout

The following structure illustrates the superset of capability surfaces. It is
not a mandatory physical tree: include only the selected bootstrap and
capabilities the repository actually uses. Names may vary when the repository
conformance layer maps them unambiguously. The placeholder names below are
semantic roles, not required package or path names.

```text
repository/
├── AGENTS.md                         # canonical router or generated projection
├── CLAUDE.md                         # generated/thin Claude bootstrap adapter
├── docs/                             # canonical product and engineering knowledge
│   ├── README.md                     # documentation map
│   ├── architecture/
│   ├── decisions/                    # ADRs
│   ├── development/
│   ├── verification/
│   ├── operations/
│   └── ...
├── .agents/                          # canonical provider-neutral agent core
│   ├── README.md                     # human-facing map of the AI config
│   ├── dependencies.lock.yaml        # pinned shared neutral packages, if any
│   ├── vendor/                       # generated/vendored shared neutral packages
│   ├── core/                          # when compiled bootstrap or catalogs exist
│   │   ├── BOOTSTRAP.md              # compiled-bootstrap owner
│   │   ├── PRINCIPLES.md             # stable neutral principles
│   │   ├── context-map.yaml          # optional routing data consumed by the resolver skill
│   │   ├── commands.yaml             # optional stable-ID index, not command bodies
│   │   ├── capabilities.yaml         # optional abstract capability vocabulary
│   │   └── schemas/                  # schemas for neutral manifests
│   ├── rules/                        # when shared scoped rules exist
│   │   └── <rule-id>.md
│   ├── workflow-risk.toml            # required when a high-risk workflow exists
│   ├── evals/workflows/              # required integration suites for classified workflows
│   ├── skills/                       # canonical Agent Skills packages
│   │   ├── <context-resolution-skill>/
│   │   │   ├── SKILL.md              # agent-facing invocation and phase-selection contract
│   │   │   ├── scripts.toml          # declares Python entrypoints and helpers
│   │   │   ├── scripts/
│   │   │   │   └── resolve_context.py # deterministic context-listing engine
│   │   │   ├── references/           # optional routing/reference material
│   │   │   └── evals/evals.json      # optional skill-local evaluation suite
│   │   └── <other-skill-name>/
│   │       ├── SKILL.md
│   │       ├── scripts.toml          # required when Python scripts exist
│   │       ├── scripts/              # optional
│   │       ├── references/           # optional
│   │       ├── assets/               # optional
│   │       ├── evals/evals.json      # optional skill-local evaluation suite
│   │       └── agents/openai.yaml    # optional Codex/OpenAI adapter sidecar
│   ├── roles/                        # when neutral role semantics exist
│   │   └── <role-name>/
│   │       ├── role.yaml
│   │       └── ROLE.md
│   ├── hooks/                        # when shared lifecycle behavior exists
│   │   ├── manifest.yaml             # neutral hook intent
│   │   ├── scripts/                  # provider-neutral lifecycle logic
│   │   └── schemas/                  # neutral event/result schemas
│   ├── integrations/                 # when shared external integrations exist
│   │   └── mcp.yaml                  # logical MCP/service registry
│   └── tests/
│       ├── context/                  # tests/evals for the context-resolution skill
│       ├── roles/
│       └── hooks/
├── tools/ai-config/                  # compiler, validator, migration utilities
│   ├── generate
│   ├── validate
│   └── provider-mappings/
├── .claude/                          # Claude adapter only
│   ├── settings.json                 # provider-native settings and hook bindings
│   ├── settings.local.json           # local, ignored
│   ├── rules/                        # generated from .agents/rules
│   ├── skills/                       # includes a link/mirror of the context skill
│   ├── agents/                       # generated from .agents/roles
│   ├── hooks/                        # provider payload wrappers/normalizers
│   └── adapter-overrides/            # provider-only authored overlays, if needed
├── .codex/                           # example provider adapter, when installed
│   ├── config.toml                   # provider-native settings/MCP/permissions
│   ├── hooks.json                    # generated hook bindings
│   ├── agents/                       # generated from .agents/roles
│   ├── hooks/                        # provider payload wrappers/normalizers
│   ├── rules/                        # command-outside-sandbox policy only
│   └── adapter-overrides/            # provider-only authored overlays, if needed
├── .mcp.json                         # generated Claude project MCP adapter, if used
└── .gitignore
```

A classified high-risk workflow requires its integration suite under
`.agents/evals/workflows/`; a skill-local `evals/evals.json` remains optional
and tests only that skill's activation and output quality.

### Generated versus authored files

The repository MUST document which files are generated. At minimum:

| Path | Status |
|---|---|
| `.agents/**` except `vendor/**` and explicitly marked provider sidecars | canonical, hand-authored |
| `.agents/vendor/**` | generated or vendored from pinned neutral packages; not hand-edited |
| `.agents/dependencies.lock.yaml` | canonical dependency lock, hand-authored or update-tool managed |
| `docs/**` | canonical, hand-authored |
| `AGENTS.md` | canonical neutral router or generated projection, according to the selected bootstrap profile |
| `CLAUDE.md` | provider wrapper or generated projection |
| `.claude/rules/**` | generated when neutral scoped rules exist |
| `.claude/skills/**` | symlinks or generated mirrors |
| `.claude/agents/**` | thin or generated projections when native workers are used |
| `.codex/agents/**` | thin or generated projections when native workers are used |
| `.codex/hooks.json` | present only when hooks exist; generated unless a deliberate exception is documented |
| `.codex/rules/**` | provider-specific, hand-authored or generated from a separate execution-policy source |
| `.claude/settings.json`, `.codex/config.toml` | provider adapter, hand-authored or generated; never canonical project knowledge |

A generated text file MUST begin with a marker similar to:

```text
GENERATED FILE — DO NOT EDIT.
Canonical source: .agents/roles/code-reviewer/ROLE.md
Generator: tools/ai-config 1.4.0
Source digest: sha256:...
```

### Multi-repository and organization-wide configuration

A group of repositories MAY share a separate provider-neutral package containing organization-wide rules, portable skills, schemas, hook libraries, and generator code. Shared content must follow the same layering rules as repository-local content.

Recommended dependency direction:

```text
provider adapter
      │
      ▼
repository-neutral core
      │
      ├────────► repository docs and code
      │
      └────────► pinned shared neutral package
```

The shared package MUST NOT depend on a consuming repository or on any provider adapter. Repository-specific architecture, commands, ownership, and product semantics MUST remain in that repository. A shared skill may define extension points, but it must not assume that every repository has the same paths, build system, or component names.

Every repository MUST be operable from a clean standalone checkout. Therefore:

- do not make correctness depend on unversioned files under `~/.claude/`, `~/.codex/`, `$HOME/.agents/`, or another repository on the developer's machine;
- do not use cross-repository symlinks such as `.agents/skills/foo -> ../../shared-ai/foo` as the only copy available to cloud, CI, or IDE agents;
- do not fetch essential instructions from the network during the first agent turn;
- do not track a floating branch or mutable URL as a configuration dependency.

The preferred distribution model is a pinned, in-repository snapshot under `.agents/vendor/<package>/`, produced by an update tool from a tagged release, commit, package artifact, Git subtree, or equivalent immutable source. A Git submodule MAY be used only when every supported local, cloud, IDE, and CI surface guarantees recursive checkout before an agent starts. Otherwise, vendor the files directly.

Example lock file:

```yaml
schema_version: 1
packages:
  engineering-agent-core:
    source: git+https://example.invalid/engineering/agent-core.git
    revision: 4d9f7f1f5e8c2d0c4d8b0d9f5a1d74e3a2a90abc
    version: 2.3.1
    digest: sha256:...
    mount: .agents/vendor/engineering-agent-core
```

Shared and local content MUST have deterministic merge rules:

1. stable IDs and skill names must be unique in the effective catalog;
2. repository-local content may extend a shared rule or role through an explicit overlay mechanism;
3. silent shadowing by path or filename is forbidden;
4. replacing a shared required rule requires a documented exception with an owner and rationale;
5. the effective merged configuration must be inspectable and validated in CI.

A central generator or validator MAY be distributed as a package, but generated provider adapters required at session start SHOULD still be committed in each repository. Updates to the shared package must be explicit: update the lock, regenerate, run cross-provider tests, and review the resulting semantic diff.

---

## Content classification and placement

Before adding content, classify it by function.

| Content | Canonical location | Must not be maintained in |
|---|---|---|
| Architecture, domain facts, invariants, rationale | `docs/architecture/`, `docs/decisions/` | provider adapters, agent prompts |
| Minimal rules needed in every task | selected canonical bootstrap | multiple independently authored root files |
| Path- or task-specific behavioral constraint | canonical docs or `.agents/rules/<id>.md` when that surface exists | `.codex/rules/`; duplicated nested instruction files |
| Reusable procedure or capability | `.agents/skills/<name>/` | long `CLAUDE.md`/`AGENTS.md` sections |
| Specialist review procedure and orchestration metadata | portable skill and its neutral catalog | independently authored provider procedures |
| Native delegation role, when required | `.agents/roles/<name>/` or a thin provider projection from a neutral catalog | independently authored provider procedures |
| Deterministic lifecycle action, when shared | provider-neutral executable implementation | prose-only instructions or duplicated adapter logic |
| Build/test/lint command | executable script or build-system target; optional stable-ID index | every skill and root prompt |
| External tool/service purpose and logical identity, when shared | neutral integration registry | duplicated provider MCP configs |
| Model name, reasoning effort, provider tool name, native sandbox key | provider adapter | neutral core |
| Personal preference or local exception | ignored local config | committed project core |
| Secret, token, OAuth state | environment, keychain, or secret manager | repository files of any layer |

### Decision rule

Use this sequence:

1. Is it a fact about the system? Put it in `docs/`.
2. Is it a constraint that always applies? Put the minimum statement in the selected canonical bootstrap; put rationale and detail in `docs/`.
3. Is it conditional on path, language, component, or task type? Put it in canonical documentation, or in `.agents/rules/` when a dedicated neutral rule surface is justified, and make it reachable through the context-resolution skill.
4. Is it a repeatable multi-step procedure? Make it a skill.
5. Does delegation require semantics beyond a portable skill and catalog? Make it a neutral role; otherwise use tested host-native delegation.
6. Must it happen regardless of model judgment? Implement it as a hook, script, test, linter, permission, or CI gate.
7. Does it mention a provider's model, tool name, invocation syntax, or config key? Keep that part in the provider adapter.

---

## Canonical documentation policy

### Documentation owns knowledge

Architecture, product semantics, security assumptions, data models, protocol behavior, invariants, and rationale MUST live in canonical documentation or executable specifications—not only in AI configuration.

Agent configuration may tell an agent **which document to read and when**. It should not restate that document.

### Recommended document metadata

Normative documents SHOULD have lightweight frontmatter:

```yaml
---
id: validator-architecture
status: active
scope:
  - crates/validator/**
owners:
  - runtime-team
last_reviewed: 2026-08-10
supersedes: []
---
```

Metadata is for routing and maintenance. Do not add metadata that is never consumed.

### Documentation categories

Use clear categories:

- **Architecture:** current system structure and invariants.
- **Decisions / ADRs:** why a consequential choice was made; accepted, superseded, or rejected status.
- **Development:** environment, code organization, conventions, local workflows.
- **Verification:** required tests, properties, formal checks, threat checks, release gates.
- **Operations:** deployment, migrations, incident procedures, rollback.
- **Reference:** schemas, APIs, terminology, generated specifications.

### Avoid duplicated command lists

Build, lint, test, formatting, type-check, generation, and release commands MUST
have one executable canonical owner. Prefer the actual build system (`Makefile`,
`justfile`, package scripts, task runner, or executable check). Add a structured
index only when a real consumer needs stable command IDs.

Do not repeat the same commands in:

- `README.md`;
- `CLAUDE.md`;
- `AGENTS.md`;
- several skills;
- every role prompt.

When human-facing documentation needs an example, link to the executable owner
or its human validation workflow, or clearly label the example as
non-normative. Detailed command ownership is in
[`VALIDATION.md`](VALIDATION.md#command-ownership-and-verification).

### Document size and routing

Documents SHOULD be cohesive rather than arbitrarily small. Split a document when different sections have different scopes or are commonly needed independently.

A task-specific read set SHOULD usually be measured in a few focused documents, not an entire documentation tree.

### Provider neutrality

Canonical documentation MUST avoid operational dependencies on provider-specific features. It MAY mention providers in comparative or meta-documentation, but product and engineering facts MUST remain usable by humans and any agent.

---
