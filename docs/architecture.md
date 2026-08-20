# Typrest Architecture

> Status: Current architecture. Code and tests are the executable source of truth.

## System Boundary

Typrest is a service, not a PDF library. Clients submit JSON to an HTTP endpoint and receive a PDF
compiled by Typst. The implemented runtime is filesystem mode: templates live under the
`templates_root` configured by `typrest.toml`.

Store mode, tags, publishing, remote backends, authentication, SDKs, and compilation caches are
outside the current scope. Filesystem mode supports literal, fully versioned packages from the
Typst Universe `preview` namespace.

## Workspace Responsibilities

| Area | Responsibility |
|---|---|
| `typrest-core` | Domain types, schemas, validation, compiler ACL, prepared registry, rendering |
| `typrest::loader` and `typrest::package` | Filesystem loading, Universe package cache/transport, and watch mode |
| `typrest::http` | Axum listener, routes, handlers, HTTP state, and response mapping |
| `typrest` application | CLI/configuration composition, diagnostics, and process lifecycle |
| `webui` | Optional Svelte development UI and committed embedded production assets |

`typrest-core` has no HTTP dependencies and does not access the physical filesystem. The
application's loader and package modules implement core-owned loading and package repository
ports. The CLI command layer prepares the registry and optional watcher before passing ready state to HTTP.
Application responsibilities use private Rust modules; a Cargo package is reserved for the
enforced core dependency boundary.

## Canonical Domain Terms

| Term | Meaning |
|---|---|
| `TemplateId` | Validated slash-separated route ID whose safe-ASCII segments do not begin with `_` |
| `Schema` | Parsed optional `schema.yml` or legacy `schema.json` payload contract |
| `TemplateSource` | Ordered map of relative virtual path to bytes |
| `TemplateBundle` | Raw `TemplateSource` plus `Option<Schema>` returned by a loader |
| `PackageCoordinate` | Exact package namespace, name, and version |
| `PackageSource` | Ordered map of package-relative virtual path to bytes |
| `PackageRepository` | Preparation-time package cache and download boundary |
| `PreparedTemplate` | Opaque compiler-owned, template-invariant Typst state |
| `PreparedTemplateBundle` | A prepared template plus its optional schema |
| `TemplateRegistry` | In-memory ID-to-prepared-bundle map built at startup and updated by watch mode |
| `RenderRequest` | `TemplateId` plus JSON payload |
| `Document` | PDF bytes plus non-fatal compiler warnings |
| `CompilationDiagnostic` | Typrest-owned severity, message, virtual location, hints, and trace |
| `CompilationReport` | Ordered diagnostics for one failed compilation or export |
| `LoaderError` | Template ID, lookup, schema, or filesystem loading failure |
| `RenderError` | Missing template, invalid payload, or request-time compilation failure |

## Template Lifecycle

```text
FilesystemLoader
  -> TemplateBundle
  -> PdfCompiler::prepare -> PackageRepository
  -> generated warmup payload
  -> PdfCompiler::compile_prepared
  -> TemplateRegistry
  -> RenderService
  -> Document
```

Filesystem discovery walks route folders recursively. A non-root directory containing `main.typ`
defines its relative path as a template ID and stops route discovery beneath it. Directories whose
names begin with `_` are skipped during discovery. Once a template is selected, all descendant
files—including underscore directories and nested `main.typ` files—remain part of its source.
`templates/main.typ` is invalid.

At startup, the application lists and loads every template, prepares compiler state, performs a
real compile and PDF export, and collects all failures before passing ready state to HTTP and
binding the socket. A schema supplies its deterministic sample payload; a schema-less template
warms with `{}`.

At request time, `RenderService` looks up the prepared bundle, validates and applies schema defaults
when a schema exists, and compiles with request-specific `sys.inputs.data`. Normal requests never
call a loader or package repository, read files, or access the network.

Watch mode builds candidates without holding the registry write lock. Ordinary source changes
replace one entry atomically. Route-topology and `_shared/` changes build a complete candidate and
swap the registry atomically; failures leave the last known good registry active.

Registry builds and reloads can notify a Typrest-owned lifecycle observer. The core emits only
template IDs, status, and warning counts; command and watch adapters decide how pretty and JSON
output is rendered. Package cache/install events stay in the application package resolver.

See [Startup Template Compilation](startup-template-compilation.md) for the detailed lifecycle.

## Compiler Anti-Corruption Layer

Only `crates/typrest-core/src/compiler/` may import `typst`, `typst-pdf`, and `typst-assets`. The
application's `package::universal_package` module uses `typst-kit` and `typst-syntax` to access the
official package cache and Universe downloader. The public compiler surface is Typrest-owned:

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

The private compiler files own Typst worlds, sources, fonts, package import discovery, PDF export,
and diagnostic mapping. All public diagnostics contain owned Typrest values. `typst`, `typst-pdf`,
`typst-assets`, `typst-kit`, and `typst-syntax` are exactly pinned to the same version in the
workspace manifest. The package repository exchanges only `PackageCoordinate` and `PackageSource`
with the compiler.

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
| `GET` | `/_api/templates/{*template_id}` | Always |
| `GET` | `/{*template_id}` | Always |
| `POST` | `/{*template_id}` | Always |
| `GET` | `/` | Redirects to `/_webui/` only with `web_ui = true` |
| `GET` | `/_webui`, `/_webui/`, `/_webui/*` | Only with `web_ui = true` |

The metadata handlers read exclusively from `TemplateRegistry`. The current server does not expose
`/healthz` or `/templates` aliases.

## Architectural Invariants

- A named non-root template directory requires `main.typ`; a schema is optional.
- The selected template directory is the virtual source root and exposes entrypoint `main.typ`.
- Template ID segments beginning with `_` are reserved for Typrest routes.
- `_shared/` is resolved by the loader, not by the compiler.
- Global `_shared/` files are inserted after local source and win same-path virtual collisions.
- Template paths remain virtual and relative throughout the compiler boundary.
- Startup validates all discovered templates in deterministic ID order.
- Compiler warnings are retained and reported but do not fail otherwise successful compilation.
- Literal, fully versioned `@preview` imports resolve during preparation and remain in memory for
  request compilation.
- Package coordinates remain separate virtual roots; package and project paths never collide.
- Other package namespaces and dynamic package import expressions are unsupported.

See the [architectural decision records](adr/README.md) for rationale and the
[testing guide](testing.md) for the tests that enforce these boundaries.
