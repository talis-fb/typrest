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

### Render Execution

- [x] Request compilation and PDF export run on blocking threads, off the async runtime
- [x] Bounded render concurrency with a FIFO queue; `503 render_capacity_exceeded` with `Retry-After`
- [x] Panics during compilation become `500 render_execution_failed` and the server keeps serving
- [x] Startup, `check`, and watch warmups share the executor with `Background` admission
- [x] `[server]` table in `typrest.toml` with `max_concurrent_renders` and `max_queued_renders`
- [x] Configurable request body limit `[server].max_body_bytes` (default 2 MiB) with `413 payload_too_large`

### Graceful Shutdown

- [x] `SIGTERM`/`SIGINT` stop `serve` with a bounded drain (`[server].shutdown_timeout_seconds`), exiting `0` on a clean drain and `1` otherwise
- [x] Explicit runtime that exits without waiting for detached compilations, and a graceful-stop check (exit status 0) in the container check

### Container Image

- [x] Container image serving the bundled `examples/minimal` project through `typrest.toml` discovery
- [x] `Dockerfile` and `docker-compose.yml` use project-config-driven `typrest serve`
- [x] Docker and Compose quickstart in `README.md`
- [x] Container check (`scripts/check-container.sh`) run by the CI `container` job

### Website and Brand

- [x] Brand kit in `brand/`, the only copy of logos, colors, type, themes, and icons
- [x] Static Astro site in `website/`: landing page, examples gallery, Getting Started, and docs
  generated from `docs/specs/` and `docs/architecture/` (see
  [`docs/development/website.md`](docs/development/website.md))
- [x] Pre-rendered example PDFs and previews from `scripts/render-website-examples.sh`
- [x] CI `website` job that validates the site, and manual deployment to Cloudflare Workers
- [x] Cloudflare Web Analytics without click tracking, and `noindex` until launch

## Active Follow-up Work

### Documentation Maintenance

- [x] Create a concise vendor-neutral `AGENTS.md`
- [x] Add a documentation index and current architecture guide
- [x] Align specifications and examples with implemented CLI, routes, schemas, and compiler layout
- [x] Adopt a documentation standard: specifications with requirement IDs, architecture, development
  guides, a pruned ADR set, and implementation plans (see [`docs/README.md`](docs/README.md))
- [ ] Keep specifications and examples synchronized in every future behavior change
- [x] Run `scripts/check-doc-links.py` in CI, together with the Web UI build and `webui/dist` drift check

### Website Launch

- [ ] Web Analytics token, then a first manual deployment to `typrest.talison.dev`
- [ ] Make the repository public, set `SITE_INDEXABLE` to `true`, and redeploy
- [ ] Repository homepage URL and social preview (`brand/social/social-card.png`)
- [x] Web UI on the brand themes and fonts, then a new Web UI screenshot for the site

### Render Execution Follow-ups

- [ ] Trim the process-global `comemo` memoization cache (owner and policy undecided)
- [ ] Run startup warmup of independent templates in parallel through the executor
- [ ] Move watch-mode preparation and loader I/O off the async runtime
- [ ] Revisit the default `max_concurrent_renders` after measuring CPU oversubscription with
  Typst's internal rayon pool

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
