---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: normative
owner: AI-CONFIG.md
document_kind: provider-comparative-meta
---

# AI Provider Adapters

This companion owns provider interoperability, adapter projections, hooks, integrations, native permission mapping, additional-provider requirements, and optional generation. See [`AI-CONFIG.md`](../AI-CONFIG.md) for dependency direction and semantic-parity invariants.

## Current interoperability reality

The architecture is based on the actual overlap between the current specifications.

| Concern | Portable common core | Claude Code projection | Codex projection |
|---|---|---|---|
| Reusable skill format | Agent Skills `SKILL.md`, scripts, references, assets | Discovers project skills under `.claude/skills/`; supports symlinked skill directories | Discovers repository skills under `.agents/skills/`; supports symlinked skill directories |
| Root instructions | Neutral bootstrap content | `CLAUDE.md`, with native `@path` imports | `AGENTS.md`; no equivalent Markdown import mechanism should be assumed |
| Scoped instructions | Neutral rule schema plus the context-resolution skill | `.claude/rules/*.md` supports path-scoped loading | `AGENTS.md` hierarchy is resolved from repository root to launch CWD once per run; do not confuse `.codex/rules/` with behavioral guidance |
| Specialized agents | Portable skill/catalog; neutral role only when required | Markdown plus YAML frontmatter in `.claude/agents/` | TOML files in `.codex/agents/` |
| Lifecycle automation | Shared scripts and neutral hook manifest | Bindings in `.claude/settings.json` or related Claude hook locations | Bindings in `.codex/hooks.json` or `.codex/config.toml` |
| MCP integrations | Logical integration registry | Project `.mcp.json` and Claude settings/scopes | `.codex/config.toml` |
| Permissions | Abstract capabilities and side-effect classes | Claude permission/settings model | Codex sandbox, approvals, permission profiles, and command rules |
| Command rules | Neutral policy intent | Claude permissions/hooks | `.codex/rules/*.rules` controls commands outside the sandbox; it is not the equivalent of `.claude/rules/*.md` |

The Agent Skills specification standardizes the skill package. It does **not** standardize root instruction files, subagent schemas, hook payloads, permission systems, MCP configuration files, or path-scoped rule discovery. Those remain adapter concerns.

---

## Hooks and deterministic automation

### Shared logic, provider bindings

When lifecycle behavior must be shared across providers, hook business logic
MUST live in a provider-neutral executable owner. Provider files only bind
native events and translate payloads/results. Provider normalizers and output
encoders remain adapter-owned; neutral handlers MUST NOT import provider
adapters. A provider-only informational convenience may remain local under the
capability-conditional rule.

```text
Claude event payload ─► Claude normalizer ─┐
                                           ├─► neutral hook handler
Codex event payload  ─► Codex normalizer  ─┘
                                           │
                              provider-specific output encoder
```

### Neutral hook manifest

A neutral manifest is required only when shared hook intent needs projection to
more than one binding. Example:

```yaml
schema_version: 1
hooks:
  - id: resolve-context-on-prompt
    event: UserPromptSubmit
    handler: scripts/resolve_context_hook.py
    purpose: Invoke the canonical context-resolution skill script and add its required-reading list to agent context.
    mode: augment
    timeout_seconds: 5
    failure_policy: open-with-warning

  - id: protect-generated-adapters
    event: PreToolUse
    handler: scripts/protect_generated.py
    purpose: Block direct edits to generated provider files.
    mode: enforce
    timeout_seconds: 2
    failure_policy: closed

  - id: verify-before-stop
    event: Stop
    handler: scripts/verify_completion.py
    purpose: Require declared verification for implementation tasks.
    mode: enforce
    timeout_seconds: 15
    failure_policy: closed
```

`resolve_context_hook.py`, when present, is only a lifecycle and payload adapter. It MUST delegate routing to:

```text
.agents/skills/<selected-context-resolution-skill>/scripts/<resolver-entrypoint>
```

It MUST NOT implement a second context map or provider-specific routing
algorithm. A hook MAY automate initial invocation, but it does not replace the
skill's explicit phase/scope lifecycle.

### Portable event profile

Use the intersection currently supported by both providers when practical:

- `SessionStart`;
- `UserPromptSubmit`;
- `PreToolUse`;
- `PermissionRequest`;
- `PostToolUse`;
- `PreCompact`;
- `PostCompact`;
- `SubagentStart`;
- `SubagentStop`;
- `Stop`;
- `SessionEnd`.

Provider-only events MAY be used in provider adapters, but the neutral core must define a fallback or mark the behavior as an optional enhancement.

### Neutral input model

Normalizers SHOULD produce a stable internal object:

```json
{
  "schema_version": 1,
  "provider": "claude",
  "event": "PreToolUse",
  "session_id": "...",
  "cwd": "/repo/subdir",
  "repository_root": "/repo",
  "actor": {
    "kind": "primary-agent",
    "role": null
  },
  "operation": {
    "category": "filesystem-write",
    "native_tool": "Edit",
    "target_paths": ["src/lib.rs"],
    "raw_input": {}
  }
}
```

Neutral handlers MUST decide using normalized categories, not native tool names, wherever possible.

### Output adaptation

Hook output and blocking schemas differ by provider and sometimes by event. The neutral handler SHOULD return an internal result:

```json
{
  "action": "allow",
  "message": null,
  "additional_context": null,
  "updated_operation": null
}
```

A provider encoder maps it into the native output schema. Do not return a Claude-specific decision object directly from shared business logic.

### Repository-root resolution

Shared hook scripts MUST determine the repository root independently, using a
repository-aware mechanism such as Git's top-level discovery from the supplied
working directory. The executable handler owns the exact command.

Do not make shared logic depend on `${CLAUDE_PROJECT_DIR}` or any provider-specific environment variable. A provider wrapper MAY use such variables as hints but must normalize them.

### Guidance versus enforcement

Use hooks, tests, or CI for actions such as:

- preventing edits to generated files;
- blocking secret files from entering prompts or commits;
- requiring context resolution before writes;
- validating migrations before release;
- enforcing formatting or generated-code freshness;
- preventing destructive commands;
- checking completion criteria before a task stops.

Do not rely on “remember to do X” in a prompt when failure is unacceptable.

### Hook engineering requirements

Hook handlers MUST:

- treat input as untrusted;
- parse JSON safely;
- validate paths and prevent path traversal;
- quote shell arguments;
- set explicit timeouts;
- avoid printing secrets or full sensitive payloads;
- be silent on success unless context is intentionally injected;
- provide concise actionable errors;
- declare fail-open or fail-closed behavior;
- avoid modifying source files unless modification is the hook's explicit purpose;
- avoid network access unless declared;
- be unit-tested with recorded provider payload fixtures.

### Trust and CI fallback

Project hooks may not run in untrusted workspaces or unsupported hosted surfaces. Therefore, no critical repository invariant may rely on a local hook alone. The same invariant MUST have an independent CI, build-system, server-side, or credential boundary where material.

---

## MCP and external integrations

### Logical registry

When a cross-provider external integration exists, its canonical logical intent
lives in a neutral registry. A repository with no such integration MUST NOT add
an empty registry. `.agents/integrations/mcp.yaml` is one possible location.

Example:

```yaml
schema_version: 1
servers:
  github:
    purpose: Read repository metadata, issues, pull requests, and CI state.
    required_for:
      - review-pull-request
      - diagnose-ci
    optional_for:
      - implement-change
    data_classification: internal
    side_effects:
      read: allowed
      write: approval-required
    authentication:
      source: environment-or-oauth
      repository_secret_forbidden: true

  internal-docs:
    purpose: Search internal engineering documentation.
    required_for: []
    optional_for:
      - architecture-research
    data_classification: confidential
    side_effects:
      read: allowed
      write: forbidden
```

The registry describes logical purpose, policy, and capability. It does not contain credentials.

### Provider projections

- Claude project MCP configuration may be generated into `.mcp.json` or the appropriate Claude settings layer.
- Codex project MCP configuration may be generated into `.codex/config.toml`.
- Server names SHOULD remain stable across providers so skills can refer to a logical identity.
- Provider-specific transport, OAuth, environment forwarding, timeout, or tool filtering belongs in adapters.

### Credentials

Credentials MUST NOT be committed. Use environment variables, OAuth, OS keychains, secret managers, or managed provider configuration.

Configuration may name an environment variable but must not include its value.

### Graceful degradation

A skill or role that requires an integration MUST state what to do when it is unavailable:

- fail with a precise prerequisite message;
- switch to a documented non-mutating fallback;
- ask for explicit setup or authorization;
- never fabricate external data.

---

## Permissions, sandboxing, and hard policy

### Adapter mapping baseline

[`AI-CONFIG.md`](../AI-CONFIG.md#authority-capability-and-enforcement) owns the
distinction between task authority, workspace capability, and enforcement
assurance. Adapters consume that contract and MUST NOT infer authority from a
native permission or sandbox mode.

Recommended defaults:

| Capability | Default |
|---|---|
| Read repository | allowed |
| Write repository workspace | task/role dependent |
| Non-mutating shell commands | allowed within the selected sandbox |
| Commands that change dependencies or generated lockfiles | approval or explicit task required |
| Network | disabled unless needed |
| External writes: issues, email, deployments, releases | explicit approval required |
| Secrets | unavailable unless the task explicitly requires a named secret |
| Destructive operations | forbidden or explicit approval plus narrow allowlist |
| Production deployment | manual-only role and external authorization |

### Provider mapping

Adapters map declared capabilities and enforcement requirements to:

- Claude permissions, settings, native tool lists, skill visibility, and hooks;
- Codex sandbox mode or permission profile, approval policy, MCP availability, hooks, and command rules;
- equivalent mechanisms in future providers.

The core MUST NOT name native tools such as `Bash`, `Edit`, or provider-specific sandbox keys as policy primitives.

### Codex `.rules` meaning

`.codex/rules/*.rules` is reserved for Codex command execution policy outside the sandbox. It MUST NOT contain or be generated from behavioral Markdown rules.

The naming collision with `.claude/rules/*.md` is semantic, not merely syntactic. Treating them as equivalent is a configuration defect.

### Prose is not enforcement

Instructions such as “never deploy without approval” are useful but insufficient. Enforce the boundary through at least one of:

- credentials unavailable to ordinary roles;
- native approval policy;
- provider hook;
- CI environment protection;
- release-system authorization;
- command deny/prompt policy;
- server-side access control.

### Local policy may be stricter

User, machine, or organization-managed settings MAY be stricter than repository defaults. Repository configuration MUST NOT attempt to weaken managed policy or require bypass modes.

---

## Claude Code adapter

### `CLAUDE.md`

`CLAUDE.md` MUST be generated or mechanically minimal. Under the
`canonical-router` profile, its recommended form is:

```markdown
@AGENTS.md

## Provider adapter

Provider-specific behavior is configured under `.claude/`.
Canonical project knowledge lives under `.agents/` and `docs/`.
```

Under `compiled-bootstrap`, the import MAY instead target the generated neutral
bootstrap projection. Claude's native import syntax allows either selected
bootstrap owner to remain canonical.

Do not place architecture, commands, coding rules, or workflows directly in `CLAUDE.md`.

### Nested `CLAUDE.md`

Existing nested `CLAUDE.md` files SHOULD be eliminated after their content is classified and migrated. Retain only generated adapters for independently operated subprojects.

### `.claude/rules/`

- Generated from `.agents/rules/`.
- May use native `paths` frontmatter.
- Must contain source provenance.
- Must not be edited directly.
- Native path loading is an optimization; the selected context-resolution skill remains authoritative.

### `.claude/skills/`

- Contains per-skill symlinks to `.agents/skills/`, or generated mirrors.
- No independently authored skill body.
- Claude-only invocation visibility should use `skillOverrides` when possible.
- Claude-only dynamic syntax belongs only in a separately named adapter wrapper, and only when a portable script cannot express the requirement.

### `.claude/agents/`

- Present only when native custom workers add required execution semantics.
- Generated from neutral roles or kept as validated thin mappings from a
  neutral skill-centered reviewer catalog.
- Native YAML maps abstract capabilities to Claude tools and permission behavior.
- Canonical role instructions are compiled into the Markdown body or loaded through a reliable canonical reference.
- Provider model selection remains in the adapter.

### Hooks

Claude hook bindings belong in `.claude/settings.json` or another documented Claude adapter scope. Bindings invoke shared scripts through provider-specific normalizers.

Shared logic must not require `${CLAUDE_PROJECT_DIR}`, though the Claude wrapper may use it as a validated hint.

### MCP

Project MCP definitions may be generated into `.mcp.json`. Provider-specific approval, transport, or authentication details remain in the adapter. Secrets remain external.

### Local state

The following are local and SHOULD be ignored:

- `.claude/settings.local.json`;
- `CLAUDE.local.md`;
- provider auth/cache/state;
- personal auto-memory;
- temporary hook logs containing session data.

Local files must not become hidden canonical project knowledge.

---

## Codex adapter

### `AGENTS.md`

Codex reads `AGENTS.md` before work and has a finite instruction budget. Under
the `canonical-router` profile, root and scoped `AGENTS.md` files are the
provider-neutral canonical routers and MUST remain free of Codex-only
mechanics. Under `compiled-bootstrap`, `AGENTS.md` is a generated projection of
the selected neutral bootstrap.

Illustrative `compiled-bootstrap` form:

```markdown
<!-- GENERATED FILE — DO NOT EDIT
Source: .agents/core/BOOTSTRAP.md
-->

[compiled BOOTSTRAP.md content]

## Codex adapter

Use `.agents/skills/` as the canonical skill catalog.
Provider-specific settings live under `.codex/`.
```

Keep either form small. The selected profile and repository validation own its
actual size limit.

### Nested `AGENTS.md`

Codex discovers instruction files from repository root down to the CWD used at session start. It does not discover arbitrary descendant instructions merely because a later task touches those paths.

Therefore:

- nested `AGENTS.md` MUST NOT be the only source of critical component rules;
- create one only for a directory that is commonly used as an independent launch root;
- generate it from neutral rules or make it a canonical router that requires the selected context-resolution skill;
- do not generate hundreds of nested files as a substitute for context resolution.

### Skills

`.agents/skills/` is already the canonical Codex repository skill path. No copy is needed.

`agents/openai.yaml` may configure OpenAI-specific presentation, implicit invocation, and dependencies. It remains a provider sidecar.

### `.codex/agents/`

- Present only when native custom workers add required execution semantics.
- Generated from neutral roles or kept as validated thin mappings from a
  neutral skill-centered reviewer catalog.
- Uses Codex's native TOML schema.
- Maps abstract capabilities to sandbox, MCP, skills, and other native settings.
- Contains generated canonical role instructions in `developer_instructions`.

### Hooks

Use `.codex/hooks.json` or inline hooks in `.codex/config.toml`, but not both in the same layer unless intentionally merged. Generated bindings invoke shared hook logic through a Codex normalizer.

### `.codex/rules/`

Use only for command-prefix allow/prompt/forbid policy outside the sandbox. Keep behavioral rules in `.agents/rules/`.

### `.codex/config.toml`

This file may contain:

- project-scoped sandbox/permission defaults;
- project MCP bindings;
- hooks or hook references;
- provider-native feature flags;
- subagent/global agent settings;
- provider-specific behavior.

It MUST NOT contain unique architecture or development knowledge.

Project `.codex/` settings may be ignored in untrusted projects. Critical invariants therefore require independent enforcement.

---

## Additional-provider adapter contract

Every installed provider directory MUST contain an immediate-child,
machine-readable `adapter.toml`. Its versioned schema declares at least:

- stable adapter `id`;
- canonical neutral bootstrap and skill-source entry points;
- one deterministic static check;
- one mutation contract proving that check detects representative damage; and
- one provider-owned live-smoke entry point.

Required static-check, mutation-contract, and live-smoke paths are
repository-relative, traversal-free, and validated as existing files by generic
adapter discovery. Wrapper paths, provider-specific boundary signatures, and
other optional capability fields remain conditional. The live owner follows
the current-evidence contract in
[`VALIDATION.md`](VALIDATION.md#provider-live-smoke-evidence); its tracked status
is not a substitute for running the provider client.

Every new provider adapter must document a capability matrix:

```yaml
provider: example-agent
version_tested: "3.2"
instructions:
  root_file: EXAMPLE.md
  imports_canonical_files: false
  nested_discovery: launch-path-only
skills:
  agent_skills_spec: true
  canonical_path_supported: false
  adapter_path: .example/skills
  symlinks_supported: true
roles:
  custom_roles: true
  native_format: yaml
hooks:
  supported: partial
  events:
    - UserPromptSubmit
    - PreToolUse
permissions:
  native_sandbox: true
mcp:
  supported: true
```

The adapter MUST define:

1. how the neutral bootstrap is loaded;
2. how canonical skills are discovered;
3. how roles are projected;
4. how scoped rules are delivered;
5. how hooks are bound and normalized;
6. how abstract capabilities map to native permissions;
7. which neutral features lack a native equivalent;
8. the tested provider/client version;
9. the fallback when a feature is unavailable.

A missing native feature does not justify moving project knowledge into the
adapter. Each adapter MUST declare support status for every workflow it exposes:

| Status | Meaning |
|---|---|
| `supported` | The native mechanism provides the full normative behavior. |
| `supported-with-equivalent-fallback` | A different mechanism provides the same normative outcomes. |
| `limited` | The workflow is unavailable and fails explicitly before execution. |
| `optional-enhancement-unavailable` | Only nonnormative ergonomics are unavailable. |

A documented degradation cannot count as semantic parity. It is valid only for
an optional enhancement; otherwise the affected workflow is `limited` and MUST
stop before execution. The adapter manifest records the status and mechanism so
validation does not infer support from provider features or filesystem shape.

---

## Adapter generation

### Compiler model

When an adapter contains derived projections, those projections are compiled
output. Thin provider-native bindings and settings may remain hand-authored
when they do not duplicate canonical semantics:

```text
canonical docs + neutral manifests + provider overlays
                         │
                         ▼
                tools/ai-config/generate
                         │
          ┌──────────────┴──────────────┐
          ▼                             ▼
   Claude adapter                 Codex adapter
```

When an adapter has no derived projection, this generation pipeline does not
apply; its thin bindings remain subject to the adapter validation contract.

### Determinism

Generation MUST be deterministic:

- stable ordering;
- normalized line endings;
- no timestamps in generated content unless excluded from freshness checks;
- stable serialization;
- pinned generator version;
- same input produces byte-identical output.

### Commit generated adapters

Generated adapters SHOULD be committed because:

- IDE agents need them before running setup;
- cloud agents may not execute a bootstrap generator;
- trust restrictions may disable project hooks/config;
- reviewers need to see the effective provider surface;
- CI can prove freshness with a clean diff.

A repository MAY omit generated output only when every supported execution surface guarantees generation first.

### Generation owner

When generation exists, provide one executable generation owner and one
executable validation owner. Human documentation points to those owners and
explains prerequisites; it does not maintain copied shell bodies. Generated
adapters must remain usable without the generator language runtime.

### Lossy mapping

The generator MUST fail when a required neutral capability cannot be represented safely in a provider adapter. It must not silently drop:

- a required skill;
- a forbidden side effect;
- a declared authority, capability, or enforcement constraint;
- an enforced hook;
- required context;
- an invocation restriction.

Optional enhancements may be dropped only with a visible warning and capability-matrix entry.

### Hand-authored provider overlays

Overlays are allowed only for provider-native concerns. The generator should reject prose blocks that substantially duplicate canonical content.

### Symlink fallback

When symlinks are unavailable, the generator may create mirrors. Each mirror must include or be paired with:

- canonical source path;
- content digest;
- generated marker;
- CI freshness check.

---
