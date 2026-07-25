# Project Context

This file is loaded automatically by Claude Code before each session.
It contains the project design context, architectural decisions, and rules.

---

## What This Project Is

An HTTP server in Rust for server-side PDF generation using **Typst** as the compilation engine. It is designed as a lightweight, performant, language-agnostic alternative to current solutions (Puppeteer, pdfmake, ReportLab, etc.).

The central differentiator: any system in any language integrates through simple HTTP. This is not a library; it is a service.

---

## Two Operation Modes

### Mode 1 - Filesystem (current MVP)

```bash
papelon serve --templates ./templates 8080
```

- Reads templates directly from the `./templates` directory
- Templates live inside the container: updating a template means rebuild + redeploy
- **No store, no tags, no versioning**
- Routes: `POST /render/:template` (no tag)
- **This is the only mode being developed now**

### Mode 2 - Store (future version, DO NOT implement now)

```bash
papelon serve --store /path/to/store 8080
```

- Templates published through `papelon push`
- Versioning with tags (`invoice:latest`, `invoice:v2.1`)
- Routes: `POST /render/:template/:tag`
- Pluggable backends: filesystem, S3, GCS

**Do not implement Mode 2 yet. The code should be prepared for it through the trait, but without an implementation.**

---

## Template Structure (what users create)

```
templates/
  invoice/
    main.typ        ← Typst template (required)
    schema.json     ← data contract (required)
    assets/         ← optional
      logo.svg
  welcome/
    main.typ
    schema.json
  _shared/          ← files available to all templates
    brand.typ
    logo.svg
```

### `schema.json` Format

The official schema specification is in:

- docs/schema-v1.md

Always consult this document before implementing any validation-related functionality.

Implementations must strictly follow the specification.
Do not introduce new fields or change DSL behavior without updating the specification first.

### How Typst Receives Data

```typst
#let data = json(bytes(sys.inputs.data))
```

The server injects the JSON payload through `sys.inputs.data` during compilation.

---

## Domain - Canonical Terms

Use **exactly** these names in code (structs, variables, functions, comments):

| Term | Meaning |
|---|---|
| `TemplateName` | newtype over String. Template identifier. Invariant: non-empty, ASCII alphanumeric only, hyphen (`-`), and underscore (`_`). |
| `Schema` | Recursive structure parsed from `schema.json`. Contains payload validation rules. |
| `TemplateSource` | Map of `relative path -> bytes`. Raw template files read by the loader. |
| `TemplateBundle` | `TemplateSource` + `Schema`. Everything the compiler needs; self-contained. Return value of `TemplateLoader`. |
| `PreparedTemplate` | Compiler ACL-owned prepared template state. Opaque outside `compiler/`; contains no public `typst::*` type. |
| `TemplateRegistry` | In-memory map of `TemplateName` to prepared template state plus `Schema`. Built at startup from a `TemplateLoader`. |
| `RenderRequest` | `TemplateName` + `serde_json::Value` (payload). Input of `RenderService`. |
| `Document` | PDF bytes. Output of `RenderService`. |
| `SchemaVersionError` | Load error: missing or unsupported `schemaVersion`. |
| `SchemaError` | Load error: structurally invalid schema (`properties` without object, enum with mixed types, etc.). |
| `ValidationError` | Render error: payload does not satisfy the schema (missing required field, wrong type, invalid enum). |
| `CompilationError` | Typst compiler failed during startup warmup or request rendering. Contains message and error line. |
| `LoaderError` | Enum that unifies `TemplateLoader` errors: `TemplateNotFound`, `SchemaVersionError`, `SchemaError`, `IoError`. |
| `RenderError` | Enum that unifies `RenderService` errors: `TemplateNotFound`, `ValidationError`, `CompilationError`. |

### Critical Distinction: Load Error vs Render Error

- **Load errors** (`SchemaVersionError`, `SchemaError`) are detected when the template is read from disk: at startup or under `--watch`. They never reach render.
- **Startup compilation errors** are detected while building `TemplateRegistry`. They must be shown during `papelon serve` startup before the server listens.
- **Render errors** (`ValidationError`, `CompilationError`) are detected at request time. Request-time `CompilationError` remains possible for payload-dependent template behavior.

---

## Architecture - Central Trait

The HTTP server never accesses the filesystem directly. All template reads go through the `TemplateLoader` trait:

```rust
#[async_trait]
pub trait TemplateLoader: Send + Sync {
    async fn load(&self, name: &TemplateName) -> Result<TemplateBundle, LoaderError>;
    async fn list(&self) -> Result<Vec<TemplateName>, LoaderError>;
}
```

Implementations:
- `FilesystemLoader` reads from the filesystem (Mode 1, implement now)
- `StoreLoader` reads from the Store (Mode 2, **DO NOT implement now**)

At server startup, the selected `TemplateLoader` is used to build a `TemplateRegistry`. Startup must list templates, load every `TemplateBundle`, prepare compiler state, run a real warmup compile/export for every template, and fail before listening if any template is broken.

`RenderService` receives the prepared `TemplateRegistry`, not an `Arc<dyn TemplateLoader>`. HTTP handlers receive the `RenderService`. Normal render requests never read templates from disk and never call `TemplateLoader::load()`.

`GET /templates` lists names from the prepared `TemplateRegistry`, not from a live filesystem scan.

See `docs/startup-template-compilation.md` before implementing startup compilation, registry behavior, request lifecycle changes, or watch-mode reload behavior.

---

## Typst Compiler Architecture - Anti-Corruption Layer

The `typst` crate has an unstable API across minor versions. To protect the rest of the application from those changes, the `compiler/` module is an **Anti-Corruption Layer (ACL)**: no other module imports `typst::*` directly.

### Absolute Rule

> If any file outside `crates/papelon-core/src/compiler/` contains `use typst::` or `use typst_pdf::`, the architecture has been violated.

The Rust compiler reinforces this: `world.rs` and `fonts.rs` are `pub(super)`, invisible outside the module.

### Public Interface - Stable, Never Changes

Defined in the `compiler` module. This is everything `RenderService` and tests know:

```rust
// The only compilation error the external world knows.
// Contains no typst::* type.
#[derive(Debug, thiserror::Error)]
pub enum CompilationError {
    #[error("compilation failed: {message}")]
    Failed { message: String, line: Option<u32> },

    #[error("pdf export failed: {0}")]
    ExportFailed(String),
}

// The only trait RenderService uses.
// Never exposes typst::* in the signature.
#[async_trait::async_trait]
pub trait PdfCompiler: Send + Sync {
    async fn prepare(
        &self,
        source: &TemplateSource,
    ) -> Result<PreparedTemplate, CompilationError>;

    async fn compile_prepared(
        &self,
        template: &PreparedTemplate,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError>;
}

// Opaque prepared template state. Contains no public typst::* type.
pub struct PreparedTemplate { /* private internals */ }

// The only concrete exported struct.
pub struct TypstCompiler { /* private internals */ }

impl TypstCompiler {
    pub fn new() -> Self { ... }
}
```

### Internal Files - May Change With Typst

All `pub(super)`, invisible outside `compiler/`:

```
compiler/
  compiler.rs  ← public interface: PdfCompiler, CompilationError, TypstCompiler
  world.rs     ← implements typst::World. ONLY file that imports typst::*
  fonts.rs     ← FontRegistry: loads typst-assets + template fonts
```

**`world.rs`** contains:
- `struct TemplateWorld` implementing `typst::World`
- `fn prepare(source, fonts) -> Result<PreparedTemplate, CompilationError>`: private function that builds reusable template-invariant state
- `fn compile_prepared(template, payload) -> Result<Vec<u8>, CompilationError>`: private function that builds request-specific inputs, orchestrates compilation, and converts Typst errors to `CompilationError`
- `fn map_errors(Vec<SourceDiagnostic>) -> CompilationError`: the only place that knows `SourceDiagnostic`
- `fn build_inputs(payload: &serde_json::Value) -> typst::foundations::Dict`: converts the payload to Typst's format

**`fonts.rs`** contains:
- `struct FontRegistry`: loaded once at startup, shared between compilations through `Arc`
- Loads default fonts from `typst-assets`
- Loads custom template fonts (`.ttf`, `.otf` files under `assets/`)

### When Typst Changes Its API

The impact stays **100% inside `world.rs` and `fonts.rs`**. Examples of historical changes and where to absorb them:

| Typst change | Where to absorb |
|---|---|
| `PdfOptions` gained new fields | `world.rs`: only PDF export inside `compile_prepared()` |
| `compile()` changed return type | `world.rs`: only Typst orchestration inside `compile_prepared()` |
| `SourceDiagnostic` gained/lost fields | `world.rs`: only `fn map_errors()` |
| `FileId`/`VirtualPath` changed module | `world.rs`: imports at the top |
| `FontBook` changed API | `fonts.rs`: only `FontRegistry` |

### MockCompiler for Tests

`RenderService` and HTTP handlers are tested with `MockCompiler` and prepared test entries: no Typst, no real compilation:

```rust
// In tests; never in production.
struct MockCompiler;

#[async_trait]
impl PdfCompiler for MockCompiler {
    async fn prepare(&self, _: &TemplateSource)
        -> Result<PreparedTemplate, CompilationError>
    {
        Ok(PreparedTemplate::for_test())
    }

    async fn compile_prepared(&self, _: &PreparedTemplate, _: &serde_json::Value)
        -> Result<Document, CompilationError>
    {
        // %PDF is the magic byte sequence: valid enough for HTTP integration tests.
        Ok(Document::new(b"%PDF-1.4 mock".to_vec()))
    }
}
```

Real Typst compilation tests live in `compiler/world.rs` as module tests (`#[cfg(test)]`): isolated, slower, and not blocking domain tests.

---

## Cargo Workspace Structure

```
papelon/
  Cargo.toml              ← workspace root
  CLAUDE.md               ← this file
  DECISIONS.md            ← architectural decision log
  ROADMAP.md              ← phase tasks with checkboxes
  docs/
    schema-v1.md          ← schema.json reference
    startup-template-compilation.md ← startup compilation + prepared runtime design
    typst-integration.md  ← typst crate API reference
  crates/
    papelon-core/         ← domain + types + traits (no HTTP, no direct filesystem access)
      src/
        lib.rs
        template_name.rs  ← TemplateName
        template.rs       ← TemplateSource, TemplateBundle, Document
        render_request.rs ← RenderRequest
        errors.rs         ← ValidationError, CompilationError
        loader.rs         ← TemplateLoader trait + LoaderError
        loader/           ← MemoryLoader (tests)
        schema.rs         ← parser, validator, Schema, SchemaField, schema errors
        compiler.rs       ← Typst ACL: public interface
        compiler/         ← private world.rs and fonts.rs
        render/           ← RenderService, RenderError
    papelon-server/       ← HTTP server (axum)
      src/
        main.rs
        routes/
        errors/
    papelon-cli/          ← CLI (clap): only `papelon init` in the MVP
      src/
        main.rs
```

**Rule:** `papelon-core` has no HTTP dependencies. It is testable in isolation with `MemoryLoader` in tests.

**Note:** the module inside `papelon-core/src/` for the trait and implementations is `loader`, not `resolver`; this follows the trait rename.

---

## HTTP API - Mode 1

### `POST /render/:template`

Request:
```
Content-Type: application/json
Body: { payload matching the template schema.json }
```

Successful response:
```
200 OK
Content-Type: application/pdf
Body: PDF bytes
```

Error responses:
```
404 - template not found
422 - invalid payload (with violations list)
500 - Typst compilation error
```

Error format:
```json
{
  "error": "schema_validation_failed",
  "message": "Payload validation failed",
  "violations": [
    { "field": "amount", "issue": "required field missing" },
    { "field": "currency", "issue": "invalid enum value 'EUR'" }
  ]
}
```

### `GET /templates`

Response:
```json
{
  "templates": [
    { "name": "invoice" },
    { "name": "welcome" }
  ]
}
```

### `GET /healthz`

```json
{ "status": "ok" }
```

---

## Crates Used

| Crate | Use |
|---|---|
| `axum` | HTTP server |
| `tokio` | async runtime |
| `serde` + `serde_json` | serialization |
| `typst` + `typst-pdf` | template compiler |
| `notify` | file watching (`--watch` mode) |
| `tracing` + `tracing-subscriber` | structured logs |
| `thiserror` | typed domain errors |
| `anyhow` | CLI errors |
| `clap` | CLI |
| `bytes` | byte handling |
| `async-trait` | async trait |

---

## Code Rules

- **No file outside `compiler/` imports `typst::*` or `typst_pdf::*`**: ACL violation
- Domain errors are always typed with `thiserror`: no `unwrap()` outside tests
- HTTP handlers never return generic errors: they always map to the JSON format above
- Integration tests use `MemoryLoader`: no filesystem dependency in core tests
- `_shared/` is included automatically in the compilation context of every template
- Subdirectories inside a template are allowed: the relative path is preserved in `TemplateSource`
- Schema errors are detected when loading the template, never during render
- Unknown fields at the root of `schema.json` are ignored with a warning, not rejected
- Use modern Rust module organization (edition 2018+): prefer `foo.rs` + `foo/` for submodules and do not create new `mod.rs` files. When touching existing modules, migrate gradually to this pattern if it is within the scope of the change.
- Do not create `pub use` only as an alias/import convenience between modules. Prefer exposing the real submodule (`pub mod memory;`, `pub mod filesystem;`) and importing through the explicit path.

---

## What NOT To Do In This MVP

- Do not implement Store (blobs, manifests, refs, tags)
- Do not implement `StoreLoader`
- Do not implement the CLI-to-server push protocol
- Do not implement authentication (token, OAuth)
- Do not implement SDKs for other languages
- Do not implement compilation cache
- Do not implement `papelon gc`
- Do not implement S3 backend
- Do not support Typst Universe packages (`@preview/...`): return a clear error

If one of these becomes necessary during development, record it in `DECISIONS.md` and discuss it before implementing.
