---
name: context-resolve
description: Resolve the smallest complete provider-neutral context set for a Keyless task before planning, implementation, review, validation, documentation, or release work.
metadata: {"authority":"non-authoring","review_guard":"none"}
---

# Resolve repository context

Use this skill before acting whenever a task can touch more than one package, a
cross-language contract, generated content, release metadata, or AI configuration.
The resolver owns path and concern routing; prose links do not extend the mandatory
context set by themselves.

From the repository root, run:

```bash
python3 .agents/skills/context-resolve/scripts/resolve_context.py \
  --mode <understand|plan|implement|review|validate|docs|release> \
  --path <touched-or-proposed-path> \
  --task "<short description>"
```

Repeat `--path` for every known path and add `--concern` for `ai-config`,
`conformance`, `security`, `generated`, `release`, `ci`, or `fuzz`. A pathless request
must supply a task whose aliases resolve to a registered scope. Unknown paths,
concerns, and pathless unresolved tasks fail closed.

Read every emitted mandatory file in order. Re-run the resolver when the intended
diff expands. Provider directories containing `adapter.toml` are routing anchors only:
they select neutral adapter policy, but their contents never become core authority.

When routing data changes, update `context-map.toml` and `fixtures.toml`, then run:

```bash
python3 .agents/skills/context-resolve/scripts/resolve_context.py --check-fixtures
```

Consult `--help` for executable syntax. Do not duplicate the map or command contract
in provider adapters.
