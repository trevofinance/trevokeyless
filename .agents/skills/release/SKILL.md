---
name: release
description: Prepare a trevokeyless Rust, Python, or TypeScript package release for a requested X.Y.Z version, including version sync, changelog, and checks. Use for release preparation only; never creates or pushes tags and never publishes.
metadata: {"authority":"authoring","review_guard":"none"}
---

# Release preparation

This is an authoring workflow limited to repository release preparation. External release
actions are outside its authority even when provider tools make them technically possible.

Resolve paths from the repository root. Require the user's request to identify one package
(`rust`, `python`, or `ts`) and an `X.Y.Z` version. Before editing, read
`docs/RELEASING.md`, `docs/versioning.md`, the package changelog, and
`docs/generated-files.md`. Inspect the changes since that package's latest release and
verify that the requested stable version satisfies the documented minimum bump.

Follow the preparation procedure and commands in `docs/RELEASING.md`: use the repository
metadata writer, review every generated and lockfile change, finalize the changelog,
run the metadata checker, and apply `gates` for the package. Do not reproduce version
sync points, reusable check commands, or publish-queue details in this skill.

Finish with a PR-ready summary of the version decision, changed files, changelog, writer
output, and checks. Committing, pushing, opening a pull request, or merging occurs only
when the user's request also authorizes that action.

**Hard stop:** release preparation never creates or pushes a tag and never publishes a
package. Those actions trigger external releases and remain a separate human-directed
operation governed by `docs/RELEASING.md`.
