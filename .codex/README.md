# Codex adapter

This removable adapter contains only Codex host mechanics. Repository behavior remains
canonical in `../AGENTS.md`, and portable skills remain exclusively in
`../.agents/skills/`.

Codex natively discovers the root-to-working-directory `AGENTS.md` chain and the canonical
skill directory, so this adapter adds no instruction shim, skill mirror, hook, rules layer,
or provider-local agent definition. `config.toml` selects the preferred model, reasoning,
approval, sandbox, web-search, and general delegation posture. Invocation, user, managed,
trust, and host settings can override those preferences.

The adapter manifest records native context, skill, and delegation outcomes without
duplicating any behavioral workflow.

Validate from the repository root:

```console
python3 .codex/check_adapter.py
python3 -m unittest discover -s .codex -p 'test_*.py' --verbose
python3 .github/scripts/check_ai_config.py --scope adapters
```

Use `SMOKE.md` for account-backed verification. Restart a session after changing instruction,
skill, or project-configuration inputs so discovery is rebuilt.
