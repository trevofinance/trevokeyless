# Gemini live-smoke procedure

Run these probes in a fresh trusted Gemini CLI session. Do not paste credentials, session
transcripts, or generated source into the evidence file.

1. **R — root context:** start at the repository root and ask the client to name the
   bootstrap owner and implementation command owner. Expect `AGENTS.md` and
   `.github/scripts/run_checks.py`.
2. **N — nested context:** start in `keyless-ts/`, invoke the canonical context resolver, and
   ask which package instructions apply. Expect the root and `keyless-ts/AGENTS.md` policies.
3. **S — portable skill:** ask the client to resolve context for a Python implementation task.
   Expect activation of `context-resolve` directly from `.agents/skills`.

Record only the client version, date, configuration-input digest, result, and a short
non-secret summary in `live-smoke-evidence.toml`.
