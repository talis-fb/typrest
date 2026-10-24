# Typrest Roadmap

This file summarizes delivered capabilities, active follow-up work, and intentionally deferred
scope. It is a status document, not an API specification. Current contracts live under [`docs/specs/`](docs/specs/README.md); scope and rationale live in the
[product vision](docs/product/vision.md).

## MVP Goal

Validate Typst as a fast, language-agnostic server-side PDF engine exposed through HTTP.

The core success criterion is implemented: a filesystem project can be checked, started, and
rendered over HTTP with payload validation and structured diagnostics.

## Delivered Milestones

### Foundation and Filesystem Loading

- [x] Cargo workspace with one non-I/O core package and one modular application package
- [x] Canonical domain types and typed errors
- [x] `TemplateLoader` abstraction with filesystem and in-memory implementations
- [x] Folder-derived template IDs, recursive sources, `_shared/`, nested assets, and direct routes
- [x] Optional canonical `schema.yml` with legacy `schema.json` compatibility
- [x] Recursive schema validation, defaults, enums, and deterministic paths

### Typst Compilation and Rendering

- [x] In-process Typst compilation and PDF export
- [x] Request data injection through `sys.inputs.data`
- [x] Default and template-local font loading
- [x] Compiler anti-corruption layer with owned diagnostics
- [x] Prepared template state reused across request compilations

### Prepared Startup Runtime

- [x] Load, prepare, compile, and export every template before socket binding
- [x] Aggregate failures across templates in deterministic order
- [x] Schema-derived warmup payloads and schema-less `{}` warmup
- [x] Registry-backed rendering and metadata
- [x] Watch reloads with atomic replacement and last-known-good behavior

### CLI, HTTP, and Diagnostics

- [x] `typrest init [path]`, `typrest check`, and project-config-driven `typrest serve`
- [x] Ancestor discovery for `typrest.toml`
- [x] PDF render, health, and prepared-template metadata routes
- [x] Structured validation and compilation error responses
- [x] Pretty diagnostics, JSON Lines output, warnings, hints, traces, and safe locations
- [x] Concise package-install, template-compilation, and ready-registry lifecycle feedback
- [x] One stdout tracing stream with specialized annotated compiler diagnostics
- [x] Unit, integration, server, CLI, and end-to-end test coverage

### Development Web UI

- [x] Optional embedded Svelte UI with committed Vite output
- [x] Prepared-template metadata and schema-aware payload controls
- [x] Raw JSON editing, PDF preview, and download
- [x] Client validation, request cancellation, Blob cleanup, and accessibility checks

## Active Follow-up Work

### Docker Project Configuration Migration

- [ ] Add a valid `typrest.toml` to the container example or generate an equivalent project layout
- [ ] Replace obsolete positional `serve` arguments in `Dockerfile` and `docker-compose.yml`
- [ ] Restore a tested Docker/Compose quickstart before advertising it in `README.md`
- [ ] Add CI or a repeatable manual check for the container entrypoint

The current Docker files predate project discovery and do not start with the implemented CLI. This
documentation cleanup does not modify non-Markdown files.

### Documentation Maintenance

- [x] Create a concise vendor-neutral `AGENTS.md`
- [x] Add a documentation index and current architecture guide
- [x] Align specifications and examples with implemented CLI, routes, schemas, and compiler layout
- [x] Adopt a documentation standard: specifications with requirement IDs, architecture, development
  guides, a pruned ADR set, and implementation plans (see [`docs/README.md`](docs/README.md))
- [ ] Keep specifications and examples synchronized in every future behavior change
- [x] Run `scripts/check-doc-links.py` in CI, together with the Web UI build and `webui/dist` drift check

## Deferred: Store Mode

Do not implement these items until filesystem mode is in production and real usage justifies the
additional operational model:

- [ ] Define Store domain and backend traits
- [ ] Implement content storage and manifests
- [ ] Add template publishing, refs, tags, and rollback
- [ ] Implement filesystem and remote Store backends
- [ ] Add Store-aware CLI commands and tagged render routes

In filesystem mode, templates are deployment artifacts with no tags or independent release
workflow. Store mode becomes relevant only when real usage requires independent template
deployment.

Literal, versioned `@preview` Typst Universe imports are implemented with preparation-time download
and a project-local cache. Authentication, SDKs, S3, garbage collection, compilation caches, other
package namespaces, custom registries, and package lockfiles remain out of scope until separately
designed and, where the choice is costly to reverse, recorded in an ADR under
[`docs/adr/`](docs/adr/README.md).
