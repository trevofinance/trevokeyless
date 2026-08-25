---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: normative
owner: AI-CONFIG.md
document_kind: provider-comparative-meta
---

# AI Configuration Migration

This companion owns staged migration guidance and prohibited migration/configuration patterns. It does not override the normative invariants in [`AI-CONFIG.md`](../AI-CONFIG.md).

## Migration policy for existing Claude-heavy repositories

Migration is a semantic classification exercise, not a bulk rename.

### Phase 1: Inventory and freeze

1. Freeze creation of new project knowledge under `.claude/`.
2. Inventory:
   - all `CLAUDE.md` and `CLAUDE.local.md` files;
   - `.claude/rules/`;
   - `.claude/skills/`;
   - `.claude/agents/`;
   - hooks and hook scripts;
   - settings and MCP definitions;
   - documentation referenced from any of them.
3. For every content block, record:
   - current path;
   - purpose;
   - scope;
   - whether it is fact, rule, procedure, role, automation, permission, or provider setting;
   - canonical target;
   - provider-specific features used;
   - enforcement requirement;
   - duplicate locations.

Do not start by copying everything into `AGENTS.md`.

### Phase 2: Establish the required neutral surfaces

Select and record a bootstrap profile, retain canonical documentation and
portable skills, and inventory which rule, role, hook, integration, generation,
or stable-command-ID capabilities actually exist. Create neutral owners only
for present cross-provider behavior. Do not create an empty reference skeleton.

Retain the existing context package as the canonical implementation and conform
it to the lifecycle and ownership contract in
[`CONTEXT-RESOLUTION.md`](CONTEXT-RESOLUTION.md#bootstrap-and-context-resolution).
Migration reuses that owner; it does not define a second resolver topology.

### Phase 3: Migrate documentation and `CLAUDE.md`

For each `CLAUDE.md` section:

| Existing content | Target |
|---|---|
| architecture or component explanation | `docs/architecture/` |
| rationale/history | ADR or decision document |
| universal short invariant | selected canonical bootstrap |
| path-specific constraint | canonical docs or a neutral rules surface when needed |
| multi-step workflow | `.agents/skills/` |
| specialist procedure | `.agents/skills/` plus stable neutral orchestration metadata when needed |
| distinct cross-provider role semantics | `.agents/roles/` when needed |
| “always run/check/block” requirement | hook, command, linter, or CI |
| model/tool/permission setting | `.claude/` adapter |
| personal preference | local ignored config |

After classification, replace root `CLAUDE.md` with the thin adapter. Remove nested files unless they are generated adapters for independent launch roots.

### Phase 4: Migrate skills

For each `.claude/skills/<name>`:

1. move the package to `.agents/skills/<name>`;
2. add or normalize `name` and `description`;
3. ensure name matches directory;
4. remove Claude-only frontmatter from the canonical file;
5. replace dynamic injection and argument placeholders with explicit scripts or instructions;
6. move only provider-native activation, UI, tool, and permission policy to provider adapters; keep portable script invocation and argument-selection logic in `SKILL.md`;
7. keep skill-specific references/assets with the skill;
8. keep general project knowledge in `docs/`;
9. conform high-risk workflows and skill-local suites to the inventory and
   evaluation contract in
   [`SKILLS-AND-DELEGATION.md`](SKILLS-AND-DELEGATION.md#skill-evaluation-corpus);
10. run the deterministic corpus and skill-package checks defined by
    [`VALIDATION.md`](VALIDATION.md#validation-and-ci-policy);
11. create a per-skill symlink, or a generated digest-checked mirror when the
    supported platform cannot reliably use symlinks;
12. test in every supported provider from clean sessions, recording unavailable clients
    as not run rather than pass.

A mechanical move is safe only for skills that already use the Agent Skills field set and portable body syntax.

### Phase 5: Migrate rules

1. Convert `.claude/rules/*.md` into neutral `.agents/rules/*.md` schema.
2. Separate detailed facts/rationale into `docs/`.
3. Register every rule in routing data consumed by the selected context-resolution skill.
4. Add phase/scope fixtures proving that the resolver returns the rule when applicable.
5. Generate Claude path-scoped rules.
6. Deliver the same rule set to Codex through the context-resolution skill and optional hook adapter.
7. Do not write behavioral content into `.codex/rules/`.

### Phase 6: Migrate agents

1. Extract specialist procedure, criteria, and output contracts into portable skills.
2. Put stable orchestration metadata in a neutral catalog without duplicating procedure.
3. Create a neutral role package only when distinct identity or permission semantics require one.
4. Keep provider model and native execution choices in adapters.
5. Generate or validate thin custom workers only for providers that need them; otherwise use tested host-native delegation.
6. Model task authority, workspace capability, and enforcement assurance separately.
7. Test delegation, output contracts, and review mutation postconditions in every supported provider.

### Phase 7: Migrate hooks

1. For shared hook behavior, move implementation logic out of the provider adapter into a neutral executable owner.
2. Define neutral event intent in a manifest only when projection consumers require one.
3. add Claude and Codex input/output normalizers under their adapter directories or `tools/ai-config/provider-mappings/`;
4. generate both provider bindings;
5. add unit fixtures from real payloads;
6. duplicate critical protections in CI or server-side policy.

### Phase 8: Migrate MCP and permissions

1. Create logical integration and capability registries only for capabilities that exist and have multiple consumers.
2. Generate or maintain thin provider configs.
3. remove credentials from repository files;
4. test unavailable-integration behavior;
5. ensure deployment, release, and destructive workflows remain approval-gated.

### Phase 9: Parallel verification

Run the same representative tasks through Claude and Codex:

- repository orientation;
- component-scoped implementation;
- cross-directory change;
- debugging;
- review;
- code generation;
- release preparation without execution;
- explicit production side-effect attempt.

Compare:

- context selected;
- skill activation;
- role delegation;
- files changed;
- commands run;
- verification performed;
- policy violations;
- final output quality.

Fix the neutral source or generator first. Patch a provider adapter only when the divergence is genuinely provider-specific.

Record the result using the current-evidence contract in
[`VALIDATION.md`](VALIDATION.md#provider-live-smoke-evidence). Migration adds no
separate evidence schema.

### Phase 10: Remove compatibility debris

After successful parallel operation:

- delete old duplicated provider content;
- remove temporary redirects;
- prohibit direct edits to generated adapters;
- add the full CI policy;
- document the canonical maintenance workflow.

---

## Prohibited patterns

The following are policy violations:

1. Copying `CLAUDE.md` into `AGENTS.md` and maintaining both.
2. Treating `AGENTS.md` as the entire provider-neutral knowledge base.
3. Loading hundreds of kilobytes of docs at every session start.
4. Keeping canonical skills under both `.claude/skills/` and `.agents/skills/`.
5. Using Claude-only skill frontmatter in the canonical `SKILL.md`.
6. Using provider-specific dynamic injection in a portable skill.
7. Mapping `.claude/rules/*.md` to `.codex/rules/*.rules`.
8. Hand-authoring Claude and Codex subagent prompts independently.
9. Keeping hook business logic under a provider folder.
10. Letting `.agents/` call into `.claude/` or `.codex/`.
11. Letting one provider adapter import another.
12. Editing generated adapter files directly.
13. Using nested provider instruction discovery as the only context-routing mechanism in a monorepo.
14. Storing architecture facts only in a skill or agent body.
15. Repeating build/test commands in many prompts and docs.
16. Treating Markdown prohibitions as hard security boundaries.
17. Committing local memory, transcripts, authentication state, or secrets.
18. Granting broad tool permissions through a portable skill.
19. Creating a subagent whose only purpose is to hold a long reference document.
20. Adding a rule or skill that has no routing path and can never activate.
21. Silent adapter degradation when a required capability is unsupported.
22. Allowing agents to modify generated providers without regeneration and validation.
23. Recreating or copying the resolver lifecycle or executable interface
    outside the owner defined by
    [`CONTEXT-RESOLUTION.md`](CONTEXT-RESOLUTION.md#bootstrap-and-context-resolution).
24. Treating a router used as a scope anchor as permission for neutral routing
    logic to depend semantically on adapter content.

---
