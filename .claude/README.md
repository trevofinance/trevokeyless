# Claude Code adapter

This removable adapter translates Claude Code host mechanics into the provider-neutral
configuration rooted at `../AGENTS.md`. It does not own repository behavior, validation
commands, or delegated workflows.

Claude loads each committed `CLAUDE.md` shim, which imports the sibling `AGENTS.md`. The real
`skills/` directory contains one relative symlink for every canonical package under
`../.agents/skills/`; no skill content is copied. Git clients must preserve symlinks.

`settings.json` selects the repository's preferred model, effort, permission, sandbox, and
experimental team posture. These are host preferences rather than policy authority; command
line, user, managed, and host settings can override them. No project MCP server is enabled by
this adapter.

The adapter manifest declares native root and nested context, symlink-based skill fallback,
and host-native delegation fallback. Delegated procedures remain canonical skills.

Validate from the repository root:

```console
python3 .claude/check_adapter.py
python3 -m unittest discover -s .claude -p 'test_*.py' --verbose
python3 .github/scripts/check_ai_config.py --scope adapters
```

Use `SMOKE.md` for account-backed verification. Evidence is intentionally limited to
non-secret probe summaries.
