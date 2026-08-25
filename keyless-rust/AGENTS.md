# Rust package router

This scope is the behavioral source of truth for Keyless encoding.

Before acting, apply the repository-root `AGENTS.md`, resolve every touched path with the
`context-resolve` skill, and read all mandatory knowledge. The resolver selects
`../docs/rust-implementation.md`, shared cross-language contracts, and any conformance,
generated-file, security, CI, fuzz, or release documents required by the task.

Re-run resolution when the change reaches another package or shared fixture. Do not put
Rust implementation rules or command inventories in this router.
