# Contributing

The Rust crate is the source of truth for the Trevo keyless encoding. Python and
TypeScript must reproduce its observable behavior.

## The one rule that matters

Every behavioral change lands with the regenerated conformance fixture and matching
Rust, Python, and TypeScript changes in the same pull request.

Before changing encoding, validation, errors, account-name handling, classification, or
public API, read [cross-language-contracts.md](cross-language-contracts.md). Fixture work
also requires [conformance.md](conformance.md) and
[conformance-schema.md](conformance-schema.md).

## Behavioral change checklist

1. Establish the observable blast radius and design the behavior in Rust.
2. Stop for explicit confirmation if an existing conformance-vector value would change.
3. Implement and test Rust. Use the fixture for representable behavior and paired Python/
   TypeScript adversarial tests for host-only inputs.
4. Regenerate the fixture through its writer and move every group, count, schema, and
   consumer pin required by the conformance procedure.
5. Port the behavior to Python and TypeScript under the shared contracts.
6. Update affected public-surface pins, executable README examples, versions, changelogs,
   and canonical documentation.
7. Run the verification selected by [development.md](development.md), including the
   differential harness for shared behavior.
8. Review the complete diff against every applicable contract before committing.

Do not split one behavioral change across multiple pull requests.

## Review procedure

Resolve review scope in this order:

1. Use an explicitly supplied file set, diff, commit, or range.
2. Otherwise review staged and unstaged changes against `HEAD`, including relevant
   untracked files.
3. Otherwise compare `HEAD` with its merge base against `origin/main`, falling back to
   local `main` when the remote-tracking reference is unavailable.

Review documentation and CI changes as contract-bearing when appropriate: package README
examples execute, workflows enforce public behavior, and canonical docs define working
agreements.

For each issue, report `file:line`, the broken contract, and the concrete fix. Also state
which applicable contracts passed, which checks actually ran, and any evidence limitation.

## Pull request expectations

- Preserve unrelated and pre-existing worktree changes.
- Change generated or lock files only through their documented writer.
- Keep public API changes Rust-first and version them according to
  [versioning.md](versioning.md).
- Keep package README examples self-contained and executable.
- Include decisive test evidence; do not describe unexecuted checks as passing.
- Keep AI configuration changes within the ownership and one-way dependency rules in
  [ai-configuration.md](ai-configuration.md).
- Use [RELEASING.md](RELEASING.md) for release preparation. Creating or pushing a release
  tag and publishing are separate, explicitly authorized operations.
