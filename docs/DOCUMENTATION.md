# Documentation conventions

Keep one full source of truth per concept and link to it elsewhere. Provider-specific
discovery and invocation mechanics stay in the relevant adapter.

| Surface | Owns |
|---|---|
| Root/package README | Published purpose, installation, API, and executable examples |
| Root/package `AGENTS.md` | Compact scope identity and resolver activation only |
| `docs/*-implementation.md` | Package file ownership and implementation invariants |
| Cross-language/conformance docs | Shared observable behavior and fixture lifecycle |
| Development/fuzz/release docs | Human selection, prerequisites, and interpretation |
| Executable scripts/workflows | Current command bodies and ordering |
| `AI-CONFIG.md`, `ai-config/*.md` | Immutable provider-neutral configuration policy |
| `AI-AGENT-GUIDE.md` | Current repository AI operating contract |
| `DECISIONS.md` | Accepted repository-specific bindings and non-goals |
| Adapter README | Provider mechanics, limitations, and diagnosis only |

Canonical project prose remains meaningful without knowing the active provider. Shared
documents must not import, link to, or require a concrete adapter. Adapter documentation
may link toward shared content.

The immutable AI policy documents use provider-comparative metadata where comparison is
their purpose. That permits provider names in prose, not a machine-resolvable dependency
from core to adapter.

Public package documentation is self-contained because it ships to registries. Internal
implementation motivation belongs in the applicable implementation or contract document;
source comments should explain only the local seam and link to a canonical owner when
more context is needed. Generated material is changed only through the writer named in
`generated-files.md`.

Create optional documentation or configuration surfaces only when they contain a real
contract and consumer. Do not create empty reference-architecture placeholders.
