---
name: add-conformance-vector
description: Add or change vectors or groups in the generated cross-language conformance fixture using its authoritative repository workflow. Use when a request changes conformance/vectors.json or the fixture generator.
metadata: {"authority":"authoring","review_guard":"none"}
---

# Adding a conformance vector or group

This is an authoring workflow. Its authority is limited to the requested fixture or
generator change and the repository-defined synchronized consequences. It does not grant
permission to alter a frozen existing vector value; the canonical explicit-confirmation
gate remains controlling.

Resolve paths from the repository root. Read `AGENTS.md`, `docs/CONTRIBUTING.md`,
`docs/conformance.md`, and `docs/conformance-schema.md`. Execute the canonical numbered
procedure without copying or weakening it here.

Finish by reporting every file updated and every check actually run. The hard stop for
existing vector values is defined in the canonical procedure.
