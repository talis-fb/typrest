# Typrest Documentation

This directory contains the canonical technical documentation for Typrest. Use this page to find
the smallest relevant document before changing code or a public contract.

## Start by Task

| Task | Document | Kind |
|---|---|---|
| Understand the system and vocabulary | [Architecture](architecture.md) | Current architecture |
| Use or change the CLI | [CLI specification](cli-spec.md) | Current specification |
| Change project configuration | [`typrest.toml` specification](typrest-toml-spec.md) | Current specification |
| Change schemas or validation | [`schema.yml` v1 specification](schema-v1.md) | Normative specification |
| Change startup, registry, or watch mode | [Startup template compilation](startup-template-compilation.md) | Current architecture |
| Change the Typst integration | [Typst integration reference](typst-integration.md) | Implementation reference |
| Change package resolution | [ADR-0011](adr/0011-resolve-preview-packages-during-preparation.md) | Accepted decision |
| Change workspace boundaries | [ADR-0012](adr/0012-consolidate-application-packages.md) | Accepted decision |
| Change diagnostics or logging | [Compiler diagnostics and logging](diagnostics.md) | Current contract |
| Add or update tests | [Testing guide](testing.md) | Development guide |
| Use or develop the Web UI | [Web UI development guide](webui-development.md) | Development guide |
| Change Web UI behavior | [Web UI specification](webui-spec.md) | Current specification |
| Understand past architectural choices | [Architectural decision records](adr/README.md) | Decision records |
| Check delivered and planned work | [`ROADMAP.md`](../ROADMAP.md) | Status and roadmap |

User-facing setup and API examples live in the repository [`README.md`](../README.md). Agent-wide
instructions live in [`AGENTS.md`](../AGENTS.md).

## Sources of Truth

- Current code, manifests, and tests are the executable truth for implemented behavior.
- Specifications define contracts that must be updated in the same change as their implementation.
- ADRs explain why non-obvious choices were made. Supersede accepted decisions with a new numbered
  record instead of silently rewriting history.
- `ROADMAP.md` records status and future scope; it is not an API specification.

If these sources disagree, verify behavior in code and tests, correct the documentation, and call
out any remaining implementation gap.

## Documentation Conventions

- Use one H1 per file and sentence-case headings below it.
- Prefer descriptive relative links to duplicated explanations.
- Label fenced code blocks with their language or `text`.
- Use `must` for normative requirements and present tense for implemented behavior.
- Keep examples executable and verify CLI syntax against `--help`.
- Do not copy dependency versions into several documents; the workspace `Cargo.toml` is canonical.
- Update this index when adding, removing, or substantially repurposing a document.
