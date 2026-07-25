# ROADMAP.md - MVP Phase 1 (Filesystem Mode)

Current development status. Update as tasks are completed.

---

## MVP Goal

Technically validate the core idea: **Typst as a server-side PDF engine, accessible over HTTP, language-agnostic**.

Success criterion: `docker run` starts the server, `curl POST /render/invoice` with JSON returns a valid PDF in less than 1 second.

---

## Phase 1 - Foundation (weeks 1-2)

**Goal:** working workspace, domain types defined, filesystem template loading.

- [x] Create a Cargo workspace with three crates: `papelon-core`, `papelon-server`, `papelon-cli`
- [x] Define domain types in `papelon-core`:
  - [x] `TemplateName` (newtype over String with validation)
  - [x] `Schema` and `SchemaField` (parsed from schema.json)
  - [x] `TemplateSource` (path -> bytes map)
  - [x] `TemplateBundle` (TemplateSource + Schema)
  - [x] `RenderRequest` (TemplateName + serde_json::Value)
  - [x] `Document` (PDF bytes)
  - [x] `ValidationError` and `CompilationError` (with thiserror)
- [x] Define the `TemplateLoader` trait in `papelon-core`
- [x] Implement `FilesystemLoader`:
  - [x] Recursively scan the templates directory
  - [x] Parse each template's `schema.json`
  - [x] Include `_shared/` files in every template context
  - [x] Preserve subdirectory structure in the file map
- [x] Implement `MemoryLoader` for tests (receives an in-memory map)
- [x] `schema.json` parser and validator:
  - [x] Types: string, number, boolean, enum, array, object
  - [x] Required vs optional fields
  - [x] Defaults for optional fields
  - [x] `validate(schema, payload) -> Result<(), Vec<ValidationError>>`
- [x] Unit tests: FilesystemLoader reads templates correctly, schema validation
- [x] CI: GitHub Actions with `cargo fmt`, `cargo clippy`, `cargo test`

**Exit criterion:** I can create a `FilesystemLoader` pointing to a templates directory, call `load("invoice")`, and receive the files and parsed schema.

---

## Phase 2 - Typst Integration (weeks 3-5)

**Goal:** given a template and a payload, compile and obtain PDF bytes.

- [x] Add `typst` and `typst-pdf` dependencies to `papelon-core`
- [x] Implement `TypstCompiler`:
  - [x] Receives `TemplateSource` (path -> bytes map) + JSON payload
  - [x] Injects payload through `sys.inputs.data`
  - [x] Returns `Document` (PDF bytes) or `CompilationError`
  - [x] Map Typst errors to `CompilationError` with a useful message
- [x] Implement `RenderService`:
  - [x] Initial loader-backed constructor
  - [x] `render(request: RenderRequest) -> Result<Document, RenderError>`
  - [x] Validates payload against the schema before compiling
  - [x] Uses the files from the `TemplateBundle`
  - [x] Calls the initial single-step compiler API
- [x] Integration tests:
  - [x] Minimal template (only `#let data = json(bytes(sys.inputs.data))`) compiles
  - [x] Invalid payload returns `ValidationError` before calling the compiler
  - [x] Syntax error in .typ returns `CompilationError` with line and message

**Exit criterion:** a test receives a real .typ template and JSON and produces valid PDF bytes (magic bytes `%PDF`).

---

## Phase 3 - HTTP Server (weeks 6-8)

**Goal:** expose RenderService over HTTP with correct error handling.

- [x] Create `papelon-server` with `axum` and `tokio`
- [x] Implement routes:
  - [x] `POST /render/:template` -> calls RenderService, returns PDF or JSON error
  - [x] `GET /templates` -> lists available templates
  - [x] `GET /healthz` -> returns `{"status": "ok"}`
- [x] Map domain errors to HTTP:
  - [x] `TemplateNotFound` -> 404
  - [x] `ValidationError` -> 422 with a violations list
  - [x] `CompilationError` -> 500 with the Typst message
- [x] Structured logs with `tracing`: each render logs template, latency, PDF size
- [x] Create `papelon-cli` with `main.rs` with argument parsing through `clap`:
  - [x] `serve <templates>` (required in Mode 1)
  - [x] `<port>` (default 8080)

**Exit criterion:** `cargo run -- serve ./examples/templates 8080` starts the server and `curl -X POST http://localhost:8080/render/invoice -d @payload.json` returns a PDF.

---

## Phase 4 - Deploy + DX (weeks 9-10)

**Goal:** any developer can use the project in less than 5 minutes.

- [x] Multi-stage Dockerfile:
  - [x] Stage 1: compile the Rust binary
  - [x] Stage 2: final debian:slim image with only the binary
  - [x] `COPY templates/` in the example Dockerfile
- [x] Example `docker-compose.yml` with templates volume
- [x] `--watch` mode in the server:
  - [x] Use the `notify` crate to detect changes in the templates directory
  - [x] On change: rescan the changed template, update `FilesystemLoader`
  - [x] Log which template was reloaded
- [x] Minimal CLI in `papelon-cli`:
  - [x] `papelon init <name>`: creates `name/main.typ` and `name/schema.json` with boilerplate
  - [x] `main.typ` boilerplate already includes `#let data = json(bytes(sys.inputs.data))`
- [x] README with:
  - [x] What the project is (3 paragraphs)
  - [x] Quickstart: docker-compose + curl in 5 minutes
  - [x] `schema.json` reference
  - [x] Complete .typ template example
  - [x] HTTP API reference
- [x] Example templates in the repository:
  - [x] `examples/templates/invoice/` - complete template with items and totals
  - [x] `examples/templates/welcome/` - simple welcome template
- [x] End-to-end test: start server on a random port, POST, verify PDF

**Exit criterion (= MVP criterion):** the README works: someone without context can start the server and generate a PDF by following the quickstart.

---

## Phase 5 - Startup Compilation + Prepared Runtime

**Goal:** fail fast on broken templates and avoid rebuilding template-invariant Typst state on every request.

- [ ] Document the startup compilation architecture in `docs/startup-template-compilation.md`
- [ ] Add `PreparedTemplate` inside the compiler ACL with no public `typst::*` exposure
- [ ] Split `TypstCompiler` into:
  - [ ] `prepare(&TemplateSource) -> Result<PreparedTemplate, CompilationError>`
  - [ ] `compile_prepared(&PreparedTemplate, &serde_json::Value) -> Result<Document, CompilationError>`
- [ ] Generate minimal valid startup payloads from `Schema`
- [ ] Build `TemplateRegistry` at server startup from the selected `TemplateLoader`
- [ ] Warm-compile and PDF-export every template before binding the server socket
- [ ] Fail startup with clear template-specific errors when load, schema, compile, or export fails
- [ ] Refactor `RenderService` to use `TemplateRegistry` instead of `TemplateLoader`
- [ ] Refactor `GET /templates` to list names from `TemplateRegistry`
- [ ] Ensure normal render requests do not call `TemplateLoader::load()` or rebuild template fonts/source maps
- [ ] Update `--watch` mode:
  - [ ] Rebuild affected prepared entries on template changes
  - [ ] Rebuild all prepared entries on `_shared/` changes
  - [ ] Keep the last known good prepared entry active when reload fails
- [ ] Add core tests for registry-backed rendering and sample payload generation
- [ ] Add compiler tests for prepare, warmup compilation, and dynamic `sys.inputs.data`
- [ ] Add server tests for startup failure, successful prepared rendering, `/templates`, and watch reload behavior

**Exit criterion:** `papelon serve ./examples/templates 8080` compiles all templates before listening, reports broken templates during startup, and render requests reuse prepared template state while still producing payload-specific PDFs.

---

## Post-MVP - Mode 2 (Store)

Do not implement until Mode 1 is in production and has feedback from real users.

- [ ] Define `StoreBackend` trait
- [ ] Implement `FilesystemStore`
- [ ] Implement push protocol (blobs, manifests, refs)
- [ ] Implement `StoreLoader`
- [ ] Implement `S3Store`
- [ ] CLI: `papelon push`, `papelon tag`, `papelon ls`
- [ ] Tagged routes: `POST /render/:template/:tag`
