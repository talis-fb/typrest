# Typrest Documentation

This directory holds the canonical documentation for Typrest. Each kind of document has one job;
use this page to find the smallest relevant one before changing code or a public contract.

## Start by task

| Task | Document |
|---|---|
| Understand what Typrest is and is not | [Product vision](product/vision.md) |
| Understand the system and vocabulary | [Architecture overview](architecture/overview.md) |
| Use or change the CLI | [CLI specification](specs/cli.md) |
| Change project configuration | [`typrest.toml` specification](specs/typrest-toml.md) |
| Change template discovery, bundles, fonts, or packages | [Template layout specification](specs/templates.md) |
| Change schemas or validation | [`schema.yml` v1 specification](specs/schema-v1.md) |
| Change routes, responses, or HTTP errors | [HTTP API specification](specs/http-api.md) |
| Change compiler diagnostics | [Compiler diagnostics](specs/diagnostics.md) |
| Change logs or process streams | [Logging and process output](specs/logging.md) |
| Change Web UI behavior | [Web UI specification](specs/webui.md) |
| Change startup, registry, or watch mode | [Template lifecycle](architecture/template-lifecycle.md) |
| Change the Typst integration or upgrade Typst | [Compiler architecture](architecture/compiler.md) |
| Add or update tests | [Testing guide](development/testing.md) |
| Use or develop the Web UI | [Web UI development guide](development/webui.md) |
| Understand why a costly choice was made | [Architectural decision records](adr/README.md) |
| Plan multi-step work | [Implementation plans](plans/README.md) |
| Check delivered and planned work | [`ROADMAP.md`](../ROADMAP.md) |

User-facing setup lives in [`README.md`](../README.md), contributor workflow in
[`CONTRIBUTING.md`](../CONTRIBUTING.md), and agent instructions in [`AGENTS.md`](../AGENTS.md).

## Document types

Each fact has one canonical home. Link to it instead of restating it.

| Type | Location | Answers | Templates |
|---|---|---|---|
| Product vision | [`product/`](product/vision.md) | Why Typrest exists, scope boundaries, high-level requirements | Conventions below |
| Specification | [`specs/`](specs/README.md) | What the system does, observably | [`specs/template.md`](specs/template.md) |
| Architecture | [`architecture/`](architecture/overview.md) | How the system is built today | Conventions below |
| Development guide | [`development/`](development/README.md) | How to work on the project | Conventions below |
| ADR | [`adr/`](adr/README.md) | Why a costly-to-reverse choice was made | [`adr/template.md`](adr/template.md) |
| Plan | [`plans/`](plans/README.md) | What is intended and not yet done | [`plans/template.md`](plans/template.md) |
| Roadmap | [`ROADMAP.md`](../ROADMAP.md) | Delivered, open, and deferred scope | none |

Specifications describe current behavior and nothing else. Architecture documents describe the
current implementation and contain no requirement IDs. An ADR records a decision and its rationale,
never the current state of the system. A plan describes intended change and is deleted when the
work is done.

## Sources of truth

- Code, manifests, and tests are the executable truth for implemented behavior.
- A specification is the contract for the behavior it covers. Update it in the same change as the
  implementation.
- When code, tests, and prose disagree, verify the behavior in the code and tests, correct the
  prose, and record any remaining implementation gap. Do not silently choose between conflicting
  accepted decisions; supersede the earlier ADR with a new one.
- Do not state in documentation that a test or behavior exists unless it has been verified.

## Requirement IDs

Externally observable contracts carry requirement IDs: the CLI (`CLI`), `typrest.toml` (`CONFIG`),
template layout (`TEMPLATE`), schema v1 (`SCHEMA`), and the HTTP API (`HTTP`). Architecture
documents, guides, logging, diagnostics, and the Web UI specification have none.

- Format: `NAMESPACE-NNN`, zero-padded, unique within the namespace. Group related requirements
  with gaps in numbering (`SCHEMA-010`, `SCHEMA-020`) so related ones can be added later.
- Write one requirement per list item: `- **CLI-014:** The command must ... .`
- IDs are stable. Never renumber or reuse one. To remove a requirement, keep its ID in place marked
  `CLI-014: (removed)`. There is no separate registry.
- Changing a requirement's meaning keeps its ID. If the new meaning is a different requirement,
  remove the old ID and add a new one.
- Refer to a requirement by ID, not by quoting it.

## Normative language

Use lowercase `must`, `must not`, `should`, and `may`. Write requirements in the present tense for
implemented behavior.

- `must` and `must not` are mandatory.
- `should` is a strong default with a justified exception.
- `may` is optional.

Do not use uppercase RFC 2119 keywords.

## Document conventions

- Use one H1 per file and sentence-case headings below it.
- Begin every document under `docs/` with a list of `- **Status:**` and, where useful,
  `- **Scope:**`. Status is exactly one value: `Current` or `Draft` for non-ADR documents (a plan is
  `Draft` until deleted); put any explanation in Scope. Index files named `README.md` are exempt.
- Use relative links, with descriptive text. Link to a heading anchor when pointing at a section.
- Label fenced code blocks with a language or `text`.
- Keep examples executable and consistent with the requirements they illustrate. Verify CLI syntax
  against `--help`.
- Do not copy dependency versions into several documents; the workspace `Cargo.toml` is canonical.
- Record contradictions and unresolved questions in the document they affect or in the plan that
  resolves them; never pick silently.
- Update this index when adding, removing, or repurposing a document.

## Updating documentation with code

A change that alters observable behavior updates its specification in the same change. A change
that alters structure updates the architecture document. A non-obvious, costly-to-reverse choice
needs a human-approved ADR (see [ADR policy](adr/README.md)). A change that alters scope or status
updates [`ROADMAP.md`](../ROADMAP.md).

Check links and anchors after editing documentation:

```bash
python3 scripts/check-doc-links.py
git diff --check
```
