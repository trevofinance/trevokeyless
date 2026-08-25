# Codex live-smoke procedure

Run these probes in a fresh trusted Codex session. Do not paste credentials, session
transcripts, or generated source into the evidence file.

1. **R — root context:** start at the repository root and ask the client to name the
   bootstrap owner and implementation command owner. Expect `AGENTS.md` and
   `.github/scripts/run_checks.py`.
2. **N — nested context:** start in `keyless-rust/` and ask which package instructions
   apply. Expect both the root and `keyless-rust/AGENTS.md` policies.
3. **S — portable skill:** ask the client to resolve context for a Rust implementation task.
   Expect activation of the canonical `context-resolve` skill from `.agents/skills`.

Record only the client version, date, configuration-input digest, result, and a short
non-secret summary in `live-smoke-evidence.toml`.
