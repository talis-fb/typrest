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

- [ ] Add `typst` and `typst-pdf` dependencies to `papelon-core`
- [ ] Implement `TypstCompiler`:
  - [ ] Receives `TemplateSource` (path -> bytes map) + JSON payload
  - [ ] Injects payload through `sys.inputs.data`
  - [ ] Returns `Document` (PDF bytes) or `CompilationError`
  - [ ] Map Typst errors to `CompilationError` with a useful message
- [ ] Implement `RenderService`:
  - [ ] Receives `Arc<dyn TemplateLoader>` in the constructor
  - [ ] `render(request: RenderRequest) -> Result<Document, RenderError>`
  - [ ] Validates payload against the schema before compiling
  - [ ] Uses the files from the `TemplateBundle`
  - [ ] Calls `TypstCompiler::compile`
- [ ] Integration tests:
  - [ ] Minimal template (only `#let data = json(bytes(sys.inputs.data))`) compiles
  - [ ] Invalid payload returns `ValidationError` before calling the compiler
  - [ ] Syntax error in .typ returns `CompilationError` with line and message

**Exit criterion:** a test receives a real .typ template and JSON and produces valid PDF bytes (magic bytes `%PDF`).

---

## Phase 3 - HTTP Server (weeks 6-8)

**Goal:** expose RenderService over HTTP with correct error handling.

- [ ] Create `papelon-server` with `axum` and `tokio`
- [ ] Implement routes:
  - [ ] `POST /render/:template` -> calls RenderService, returns PDF or JSON error
  - [ ] `GET /templates` -> lists templates available through TemplateLoader
  - [ ] `GET /healthz` -> returns `{"status": "ok"}`
- [ ] Map domain errors to HTTP:
  - [ ] `TemplateNotFound` -> 404
  - [ ] `ValidationError` -> 422 with a violations list
  - [ ] `CompilationError` -> 500 with the Typst message
- [ ] `main.rs` with argument parsing through `clap`:
  - [ ] `--templates <path>` (required in Mode 1)
  - [ ] `<port>` (default 8080)
- [ ] Structured logs with `tracing`: each render logs template, latency, PDF size
- [ ] End-to-end test: start server on a random port, POST, verify PDF

**Exit criterion:** `cargo run -- --templates ./examples/templates 8080` starts the server and `curl -X POST http://localhost:8080/render/invoice -d @payload.json` returns a PDF.

---

## Phase 4 - Deploy + DX (weeks 9-10)

**Goal:** any developer can use the project in less than 5 minutes.

- [ ] Multi-stage Dockerfile:
  - [ ] Stage 1: compile the Rust binary
  - [ ] Stage 2: final debian:slim image with only the binary
  - [ ] Result: image < 100MB
  - [ ] `COPY templates/` in the example Dockerfile
- [ ] Example `docker-compose.yml` with templates volume
- [ ] `--watch` mode in the server:
  - [ ] Use the `notify` crate to detect changes in the templates directory
  - [ ] On change: rescan the changed template, update `FilesystemLoader`
  - [ ] Log which template was reloaded
- [ ] Minimal CLI in `papelon-cli`:
  - [ ] `papelon init <name>`: creates `name/main.typ` and `name/schema.json` with boilerplate
  - [ ] `main.typ` boilerplate already includes `#let data = json(bytes(sys.inputs.data))`
- [ ] README with:
  - [ ] What the project is (3 paragraphs)
  - [ ] Quickstart: docker-compose + curl in 5 minutes
  - [ ] `schema.json` reference
  - [ ] Complete .typ template example
  - [ ] HTTP API reference
- [ ] Example templates in the repository:
  - [ ] `examples/templates/invoice/` - complete template with items and totals
  - [ ] `examples/templates/welcome/` - simple welcome template

**Exit criterion (= MVP criterion):** the README works: someone without context can start the server and generate a PDF by following the quickstart.

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
