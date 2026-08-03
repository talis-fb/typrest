# Typrest Architecture

> Status: Current architecture. Code and tests are the executable source of truth.

## System Boundary

Typrest is a service, not a PDF library. Clients submit JSON to an HTTP endpoint and receive a PDF
compiled by Typst. The implemented runtime is filesystem mode: templates live under the
`templates_root` configured by `typrest.toml`.

Store mode, tags, publishing, remote backends, authentication, SDKs, compilation caches, and Typst
Universe packages are outside the current scope.

## Workspace Responsibilities

| Area | Responsibility |
|---|---|
| `typrest-core` | Domain types, schemas, validation, compiler ACL, prepared registry, rendering |
| `typrest-server` | Filesystem loader, watch mode, Axum routes, HTTP errors, startup, diagnostics |
| `typrest-cli` | Project discovery/configuration and the `init`, `check`, and `serve` commands |
| `webui` | Optional Svelte development UI and committed embedded production assets |

`typrest-core` has no HTTP dependencies and does not access the physical filesystem. The
filesystem loader belongs to `typrest-server` and implements the core `TemplateLoader` trait.

## Canonical Domain Terms

| Term | Meaning |
|---|---|
| `TemplateName` | Validated non-empty ASCII identifier containing alphanumerics, `-`, or `_` |
| `Schema` | Parsed optional `schema.yml` or legacy `schema.json` payload contract |
| `TemplateSource` | Ordered map of relative virtual path to bytes |
| `TemplateBundle` | Raw `TemplateSource` plus `Option<Schema>` returned by a loader |
| `PreparedTemplate` | Opaque compiler-owned, template-invariant Typst state |
| `PreparedTemplateBundle` | A prepared template plus its optional schema |
| `TemplateRegistry` | In-memory name-to-prepared-bundle map built at startup and updated by watch mode |
| `RenderRequest` | `TemplateName` plus JSON payload |
| `Document` | PDF bytes plus non-fatal compiler warnings |
| `CompilationDiagnostic` | Typrest-owned severity, message, virtual location, hints, and trace |
| `CompilationReport` | Ordered diagnostics for one failed compilation or export |
| `LoaderError` | Template name, lookup, schema, or filesystem loading failure |
| `RenderError` | Missing template, invalid payload, or request-time compilation failure |

## Template Lifecycle

```text
FilesystemLoader
  -> TemplateBundle
  -> PdfCompiler::prepare
  -> generated warmup payload
  -> PdfCompiler::compile_prepared
  -> TemplateRegistry
  -> RenderService
  -> Document
```

At startup, the server lists and loads every template, prepares compiler state, performs a real
compile and PDF export, and collects all failures before deciding whether to bind the socket. A
schema supplies its deterministic sample payload; a schema-less template warms with `{}`.

At request time, `RenderService` looks up the prepared bundle, validates and applies schema defaults
when a schema exists, and compiles with request-specific `sys.inputs.data`. Normal requests never
call the loader or read files.

Watch mode builds a candidate entry without holding the registry write lock. A successful candidate
replaces the old entry atomically; a failed reload leaves the last known good entry active. Changes
under `_shared/` rebuild every template.

See [Startup Template Compilation](startup-template-compilation.md) for the detailed lifecycle.

## Compiler Anti-Corruption Layer

Only `crates/typrest-core/src/compiler/` may import `typst::*` or `typst_pdf::*`. The public compiler
surface is Typrest-owned:

```rust
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
```

The private compiler files own Typst worlds, sources, fonts, PDF export, and diagnostic mapping.
All public diagnostics contain owned Typrest values. The three Typst crates are exactly and jointly
pinned in the workspace manifest.

## Error Boundaries

- Configuration errors occur before a loader is created.
- `SchemaVersionError` and `SchemaError` occur while loading templates, never during render.
- Registry preparation and warmup failures prevent startup.
- `ValidationError` occurs before request compilation.
- `CompilationError` may occur during warmup or rendering because template behavior can depend on
  the payload.
- HTTP handlers convert render failures into structured JSON and never expose physical paths or
  source excerpts.

## HTTP Surface

| Method | Route | Availability |
|---|---|---|
| `GET` | `/_healthz` | Always |
| `GET` | `/_api/templates` | Always |
| `GET` | `/_api/templates/:template` | Always |
| `POST` | `/render/:template` | Always |
| `GET` | `/webui`, `/webui/`, `/webui/*` | Only with `web_ui = true` |

The metadata handlers read exclusively from `TemplateRegistry`. The current server does not expose
`/healthz` or `/templates` aliases.

## Architectural Invariants

- `main.typ` is required; a schema is optional.
- `_shared/` is resolved by the loader, not by the compiler.
- Template paths remain virtual and relative throughout the compiler boundary.
- Startup validates all discovered templates in deterministic name order.
- Compiler warnings are retained and reported but do not fail otherwise successful compilation.
- Unsupported Typst packages resolve as missing virtual files; no network package fetch occurs.

See the [architectural decision records](adr/README.md) for rationale and the
[testing guide](testing.md) for the tests that enforce these boundaries.
