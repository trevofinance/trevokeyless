---
policy_bundle: provider-neutral-ai-config
policy_version: 3.0.0
status: normative
owner: AI-CONFIG.md
document_kind: provider-comparative-meta
---

# AI Configuration Validation

This companion owns executable-command indexing rules, deterministic checks, CI gates, conformance evidence, and policy maintenance. See [`AI-CONFIG.md`](../AI-CONFIG.md) for the concise conformance checklist.

## Command ownership and verification

### One command source

The executable script, task-runner target, package script, or build-system
target is the canonical owner of command semantics. Human validation
documentation explains when and why to use it, its prerequisites, sequencing,
restrictions, and expected result. Do not copy executable shell strings into a
second authored catalog.

A repository MAY add a structured index when automation or portable skills
need stable command IDs. Such an index points to executable owners:

```toml
[commands.agent_config]
entrypoint = "<repository-validation-owner>"

[commands.agent_adapters]
entrypoint = "<adapter-validation-owner>"
```

The example illustrates references, not mandatory repository paths.

When full, fast, pre-commit, or similar aggregate runners share a discovery
policy, keep that selection in one executable owner with named profiles. The
runners consume the selected owner list; workflow prose explains why to choose
a profile and MUST NOT reproduce its discovery loop or current inventory.
Mutation or contract tests SHOULD pin inclusions, exclusions, ordering, and
deterministic output.

### Command IDs in agent content

When a structured index exists, skills, roles, and rules SHOULD use its stable
IDs. Otherwise they refer to the executable owner or to the human validation
workflow. They MUST NOT maintain another copy of the executable body. The
context-resolution skill MAY list applicable IDs or owners, but command lookup
is not its required core function.

### Verification plans

Every implementation task SHOULD derive a verification plan from:

- changed paths;
- applicable rules;
- activated skills;
- executable command owners and any optional command index;
- explicit user acceptance criteria.

The final response or handoff MUST distinguish:

- checks executed and passed;
- checks executed and failed;
- checks not executed and why;
- remaining uncertainty.

### No invented commands

When a canonical executable owner exists, agents MUST use it. If an optional
index is stale, report and repair the reference rather than silently substitute
an undocumented workflow.

---

## Validation and CI policy

A conforming repository MUST run an AI-configuration validation job.

### Required gates

#### A. Schema and manifest validation

Always validate the repository conformance selection, context-resolution
routing data, and every installed adapter's manifest and deterministic entry
points. Confirm that selected identifiers resolve and that the manifest's
policy version matches the policy bundle. Validate command indexes, capability
catalogs, neutral rules, role manifests, hook manifests, integration
registries, and provider capability matrices only when those surfaces exist.

#### B. Agent Skills validation

For every `.agents/skills/*`:

- run a conformant Agent Skills structure validator;
- verify directory/name equality;
- reject forbidden provider-only frontmatter;
- reject provider-only body syntax;
- verify referenced files exist;
- require every Python file under a skill's `scripts/` directory to have one
  declared role in `scripts.toml`;
- exercise only declared CLI entrypoints for side-effect-free help and
  invalid-input behavior;
- repeatedly import declared helpers under an operational-side-effect audit
  and execute their declared unit tests; and
- validate `evals/evals.json` when present.

#### C. Dependency-direction validation

Parse Markdown links/imports, symlinks, script paths, and known config path fields.

Fail when:

- `.agents/` runtime content depends semantically on a provider adapter or
  wrapper; using a router location only as a repository-scope anchor is not a
  dependency on its prose;
- one provider adapter depends on another;
- canonical docs rely on provider adapters for normative content;
- a shared script imports a provider-specific implementation instead of a neutral interface.

#### D. Core independence and adapter contracts

Run the provider-neutral core after removing or hiding every provider adapter.
A local detached-worktree check MUST overlay the complete change under review:
staged and unstaged tracked changes plus non-ignored untracked files. Testing
only the previous `HEAD` is not evidence for a dirty current patch. Each
installed adapter MUST declare and pass a static check plus a mutation contract
that proves the static check detects representative breakage. The remote
aggregate MUST execute both the core and all installed adapter contracts.

#### E. Generated freshness, when generation exists

Run the canonical generator and require a clean tracked diff for all generated
adapters.

#### F. Generated-file protection, when generated files exist

Check that:

- generated markers exist;
- source paths are valid;
- source digests match;
- no generated file contains an unapproved hand-edited block.

#### G. Context-routing skill coverage

Verify the resolver lifecycle, ownership split, reachability, and failure
behavior defined by
[`CONTEXT-RESOLUTION.md`](CONTEXT-RESOLUTION.md#bootstrap-and-context-resolution).
The checker derives executable modes and option shapes from the declared
interface and verifies their fixture coverage; it does not maintain a second
phase table or routing contract.

#### H. Link and path integrity

Check:

- broken Markdown links;
- missing referenced scripts/assets;
- obsolete glob paths;
- symlink targets;
- case-sensitive path mismatches;
- references escaping the repository unexpectedly.

#### I. Duplicate-content detection

Deterministic CI MUST detect exact or normalized copies of known canonical
policy paragraphs and volatile command contracts between:

- core and provider adapters;
- `CLAUDE.md` and `AGENTS.md`;
- skills and canonical docs;
- multiple rules.

Generated duplication with declared provenance and the selected router's
minimal activation template are exempt. Broader semantic or paragraph-similarity
analysis MAY emit review warnings, but it MUST NOT become a hard gate without a
stable threshold and an explicit exemption mechanism.

#### J. Hook validation, when hooks exist

- lint scripts;
- unit-test normalized payloads;
- test allow/block/augment paths;
- test timeout behavior;
- test malformed JSON and missing fields;
- test secret redaction;
- verify provider bindings reference only approved handlers.

#### K. Command-index validation, when an index exists

- command IDs are unique;
- referenced command IDs exist;
- command scopes are valid;
- optional smoke checks confirm commands still exist in package/build metadata.

#### L. High-risk workflow and skill-local evaluations

Consume and validate the risk inventory and repository-native behavior-eval
schema owned by
[`SKILLS-AND-DELEGATION.md`](SKILLS-AND-DELEGATION.md#skill-evaluation-corpus).
The checker verifies referential integrity, exact inventory-to-suite matching,
and every required case/assertion class from that schema. This deterministic
gate proves corpus conformance, not model behavior; it MUST NOT introduce a
second risk taxonomy or case schema.

Run actual model or client behavior evaluations at the cadence selected by the
repository and after relevant provider or workflow changes. Record provider,
client version, date, result, and evidence. An unavailable client is **not run**
and an ambiguous response is **indeterminate**; neither is a pass.

#### M. Secret scanning

Scan all tracked layers for tokens, private keys, authorization headers, OAuth
material, provider authentication or session state, and accidental transcript
data in both filenames and content. Failure output MUST identify the affected
path without echoing the matched value. Mutation fixtures SHOULD be generated
at runtime so credential-shaped samples and transcript artefacts are not
committed merely to test the scanner.

### Provider live-smoke evidence

Every installed adapter MUST own a clean-session live-smoke procedure for
native discovery and behavior that static inspection cannot prove. Critical
repositories SHOULD include these probes:

1. ask the agent to list effective instruction sources;
2. ask it to identify the canonical core and adapter boundary;
3. ask it which skill applies to a representative task;
4. invoke a critical skill;
5. exercise review-only delegation and its mutation postcondition;
6. confirm a generated-file edit is blocked;
7. confirm required context is resolved for a path outside the launch CWD;
8. confirm the expected verification plan is selected.

These tests validate behavior, not merely file existence. The adapter-owned
procedure MUST keep a machine-visible current-evidence summary containing the
run date, exact client and version or `UNAVAILABLE`, and a per-probe result of
`PASS`, `FAIL`, `NOT_RUN`, or `INDETERMINATE`. Static checks validate the
summary's presence and vocabulary but MUST NOT promote a live result. Keep raw
prompts, model-visible input, event streams, transcripts, final messages,
rollouts, credentials, and local client state untracked; commit only a
non-secret summary sufficient to explain each verdict.

The adapter manifest MUST declare the evidence path, expected client name and
exact tested version or `UNAVAILABLE`, adapter-input selectors, and relevant
canonical-profile selectors. The evidence record MUST include:

- schema and probe-set schema versions;
- adapter ID;
- client name and exact version;
- execution or invalidation date;
- a digest of the declared adapter inputs;
- a digest of the declared canonical inputs;
- explicit invalidation triggers; and
- stable probe IDs, statuses, and non-secret summaries.

Digests are computed deterministically from the ordered tracked path, file
type, and current content of every selected input; the evidence file itself is
excluded to avoid a digest cycle. Evidence is current only when both digests,
the adapter ID, and the manifest's client identity match. A changed client
version, instruction-discovery input, skill projection, permission or hook
mapping, adapter configuration, or relevant canonical profile invalidates a
previous result. Deterministic validation MUST then fail until the record is
updated to an honest `NOT_RUN`, `INDETERMINATE`, `FAIL`, or newly observed
`PASS` state with current digests. `UNAVAILABLE` requires every probe to be
`NOT_RUN`. A calendar expiry MAY supplement these rules but MUST NOT replace
configuration and version invalidation.

Rerun affected probes after a client upgrade or a change to instruction
discovery, skills, hooks, permissions, integrations, plugins, worker projection,
or delegation mechanics. An unavailable client is `NOT_RUN`; missing,
ambiguous, or incomplete evidence is `INDETERMINATE`; neither is a pass.

### Conformance report

The repository's aggregate validation owner SHOULD output a concise report.
For example:

```text
AI config policy: PASS
Skills: 14 valid, 0 invalid
Roles: 7 projected to Claude and Codex
Rules: 23 routed, 0 unreachable
Hooks: 6 handlers, 12 provider bindings, 0 schema drift
Generated adapters: clean
Dependency graph: acyclic
Context tests: 18/18 passed
Warnings: 1 optional Codex UI sidecar missing icon
```

---

## Conformance checklist

A repository conforms when all answers are “yes”:

### Source of truth

- [ ] Are `docs/`, portable skills, and the selected neutral bootstrap the only authored sources of cross-provider project knowledge?
- [ ] Does every fact, rule, command, workflow, and role have one canonical owner?
- [ ] Does one machine-readable conformance manifest own all selected profiles and canonical identifiers?
- [ ] Are unused capability surfaces absent rather than represented by empty scaffolding?
- [ ] Is the dependency graph adapter → core → docs/code, with no reverse or cross-provider semantic dependencies, while allowing `AGENTS.md` solely as a resolver path anchor?
- [ ] When shared across repositories, are neutral packages pinned, self-contained in a clean checkout, and merged without silent shadowing?

### Context

- [ ] Is the bootstrap small and does it require invocation of the selected context-resolution skill?
- [ ] Is the selected context-resolution skill the agent-facing context interface and sole owner of phase/scope selection and script invocation?
- [ ] Is its declared resolver entry point the single deterministic routing engine?
- [ ] Does executable help own the exact CLI, and do automated checks keep it aligned with the skill?
- [ ] Do Claude and Codex discover or project the same canonical skill package without duplicating its command contract?
- [ ] Is there no duplicate resolver under `tools/ai-config/`, `.claude/`, or `.codex/`?
- [ ] Is every work phase and option advertised by executable help covered by tests?
- [ ] Does resolution rerun when the phase or affected scope changes?
- [ ] Can it route tasks spanning multiple directories and languages?
- [ ] Are all normative documents and rules reachable through phase/scope routing tests?

### Skills

- [ ] Are canonical skills under `.agents/skills/`?
- [ ] Do they validate against Agent Skills?
- [ ] Are provider-only fields and syntax excluded?
- [ ] Does Claude use symlinks or generated mirrors rather than authored copies?
- [ ] Is every high-risk workflow explicitly classified with entrypoints, participants, risk rationale, and exactly one matching integration suite?
- [ ] Do workflow suites cover activation, ambiguity, non-activation, unauthorized actions, output, and allowed/forbidden side effects?
- [ ] Do skill-local suites remain separately validated as one-skill activation and output tests?
- [ ] Does every portable Python skill file have one declared script role, with CLI contracts limited to entrypoints and import-safety plus test coverage required for helpers?

### Rules and roles

- [ ] When scoped rules exist, are they neutral and routed?
- [ ] When `.codex/rules/` exists, is it used only for command execution policy?
- [ ] Do specialist procedures live in portable skills with neutral orchestration metadata where needed?
- [ ] Are custom native worker files present only when they add required execution semantics?
- [ ] Are authority, capability, and enforcement assurance represented separately?
- [ ] Does diagnostic-write-capable review enforce a tracked-state postcondition without claiming hard read-only behavior?
- [ ] Are parallel writes controlled?

### Hooks and enforcement

- [ ] When shared hooks exist, is their business logic provider-neutral?
- [ ] When provider hook bindings exist, are payloads normalized?
- [ ] For each present hook, are failure policies and timeouts explicit?
- [ ] Are critical invariants duplicated in CI or server-side controls?
- [ ] Does every hard external boundary have current evidence, with unverified controls failing closed rather than being promoted by prose?
- [ ] When generated files exist, are they protected from direct edits?

### Adapters and CI

- [ ] Are root provider files thin or generated?
- [ ] When generated files exist, are they marked with source provenance?
- [ ] When generation exists, does it produce a clean diff?
- [ ] Does CI reject stale adapters, broken links, secret material, and dependency violations?
- [ ] Does local core-independence validation exercise the complete current patch?
- [ ] Does every adapter retain current live-smoke status without static promotion?
- [ ] Have representative tasks been smoke-tested in every supported provider, with not-run and indeterminate outcomes reported honestly?

---

## Maintenance policy

### Change the highest canonical layer

When behavior is wrong:

1. fix canonical product documentation if the fact is wrong;
2. fix a neutral rule if applicability or instruction is wrong;
3. fix a skill if the procedure is wrong;
4. fix a role if delegation behavior is wrong;
5. fix shared hook logic if enforcement is wrong;
6. fix the provider generator/mapping only when the neutral intent is correct but the provider projection is wrong.

Do not patch generated adapter text as the first response.

### Versioning

The neutral configuration schema and generator SHOULD use semantic versioning.

- Patch: wording, routing, or generator fixes without schema change.
- Minor: backward-compatible schema capability.
- Major: breaking layout, manifest, or adapter contract change.

Generated files SHOULD record the generator and schema version.

### Vendor changes

Review provider documentation periodically. When a provider changes discovery, hook schemas, agent formats, or permission behavior:

- update the provider capability matrix;
- update provider mappings and fixtures;
- regenerate adapters;
- rerun provider smoke tests;
- avoid changing neutral project knowledge unless the intended workflow itself changed.

---
