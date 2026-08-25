# Gemini CLI adapter

This removable adapter maps Gemini CLI host mechanics to the provider-neutral configuration.
The root `../GEMINI.md` imports `../AGENTS.md`; package routing remains explicit through the
canonical `context-resolve` skill because the adapter does not create a second tree of nested
instruction shims.

Gemini discovers the canonical `.agents/skills/` directory directly. This adapter therefore
adds no skill mirror, project settings, hooks, or custom-agent layer. The manifest records
native root context and skills, fallback nested context, and limited host delegation.

Validate from the repository root:

```console
python3 .gemini/check_adapter.py
python3 -m unittest discover -s .gemini -p 'test_*.py' --verbose
python3 .github/scripts/check_ai_config.py --scope adapters
```

Use `SMOKE.md` for account-backed verification. Evidence is limited to non-secret probe
summaries and must never contain a session transcript.
