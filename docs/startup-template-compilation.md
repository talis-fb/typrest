# Startup Template Compilation

## Summary

Typrest should load, validate, prepare, and warm-compile every template before the HTTP server starts listening.

Startup compilation makes broken templates visible immediately on `typrest serve` startup instead of delaying template errors until the first matching render request. Requests still compile and export PDFs because each request has a different JSON payload, but they should reuse template-invariant Typst world state prepared at startup.

This behavior should stay policy-driven, not hardcoded. In the future we should be able to disable startup warmup for specific templates or globally, for example with a template config flag like `compile.lazy = true`. The startup and registry design should therefore treat warmup as a configurable step that can be skipped, not as an assumption baked into the type shape or control flow.

The phrase "shared World" means shared template-invariant world state. It does not mean one immutable global Typst `World` per template, because `sys.inputs.data` is request-specific.

## Compilation lifecycle

1. The server creates the configured `TemplateLoader` implementation. In Mode 1 this is `FilesystemLoader`.
2. The server calls `TemplateLoader::list()` to discover all available `TemplateName` values.
3. For each template name, the server calls `TemplateLoader::load()` to get a `TemplateBundle`.
4. Load-time errors remain load errors:
   - missing template files become `LoaderError::IoError`;
   - invalid or unsupported `schemaVersion` becomes `SchemaVersionError`;
   - structurally invalid schema JSON becomes `SchemaError`.
5. The compiler prepares the reusable template state from `TemplateSource`.
6. The startup path generates a minimal valid sample payload from the template `Schema`.
7. The compiler performs a real Typst compile and PDF export using the prepared template state and the generated sample payload.
8. All discovered templates are checked in name order. Any load, schema, compile, or export error
   aborts startup before the server binds its socket, after every template failure has been
   collected and reported.
9. Successful templates are stored in an in-memory `TemplateRegistry` keyed by `TemplateName`.

Startup errors must be logged or printed with the template name, the error category, the message, and the Typst line number when one is available.
Successful Typst warnings are reported with their complete diagnostic context but do not fail
startup.

The sample payload generator should use deterministic values:

- use explicit schema defaults first;
- use the first enum value when an enum is present;
- otherwise synthesize by type: `""`, `0`, `false`, `{}`, `[]`, or `null`;
- for objects, include known properties, especially required properties;
- for arrays, use an empty array unless the schema specification later requires item coverage.

## Request lifecycle

1. The HTTP handler parses the route `TemplateName` and JSON request body.
2. `RenderService` looks up the template in `TemplateRegistry`.
3. If the name is missing, `RenderService` returns `RenderError::TemplateNotFound`.
4. The payload is validated with defaults using the stored `Schema`.
5. If validation fails, `RenderService` returns `RenderError::ValidationError` before calling the compiler.
6. The compiler builds only the request-specific `sys.inputs.data` value from the validated payload.
7. The compiler reuses the prepared template state and runs Typst compile plus PDF export.
8. The HTTP response returns PDF bytes or maps `RenderError` to the existing JSON error format.

Normal request handling must not read template files from disk, call `TemplateLoader::load()`, rebuild loaded template fonts, or recreate the full template source map.

Request-time `CompilationError` can still happen for payload-dependent template behavior. For example, a template may branch on payload data and import or evaluate code that was not exercised by the startup sample payload.

## Compiler design

The compiler module remains the anti-corruption layer around Typst. No file outside `crates/typrest-core/src/compiler/` may import `typst::*` or `typst_pdf::*`.

Add an opaque public compiler type:

```rust
pub struct PreparedTemplate {
    // private Typst-facing internals
}
```

The public compiler trait should split template preparation from request compilation:

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

`PreparedTemplate` should own or share the template-invariant data that is currently rebuilt inside `TemplateWorld::new()`:

- `main.typ` as a Typst `Source`;
- the file map converted to Typst byte storage;
- loaded fonts for the template, including default fonts and template fonts from `assets/`;
- any stable file identifiers or source metadata needed for diagnostic mapping.

The request-specific world can then borrow or clone the prepared state and build a fresh Typst `Library` with `sys.inputs.data` for each render.

## Registry design

Add a startup-built `TemplateRegistry` that stores each successfully prepared template:

```rust
pub struct TemplateRegistry {
    // TemplateName -> prepared entry
}

pub struct PreparedTemplateEntry {
    pub schema: Schema,
    pub prepared: PreparedTemplate,
}
```

The exact fields may stay private if helper methods make `RenderService` and handlers simpler.

The registry stores immutable entries behind `Arc` and protects only the name-to-entry map with a
short-held `RwLock`. Lookups clone the entry `Arc` before request compilation begins. Reloads load,
prepare, and warm-compile a candidate without holding the registry lock, then replace the entry with
one write operation after every step succeeds.

`TemplateRegistry::build()` performs the startup lifecycle in the sorted order returned by the
loader and returns the prepared registry plus warnings, or an aggregate containing every template
failure. `TemplateRegistry::reload()` performs the same load, prepare, generated-payload
validation, and warmup lifecycle for one template and returns any successful warnings. Registry
errors retain the template name and distinguish listing, loading, preparation, invalid generated
samples, and warmup failures.

`RenderService` should receive the registry and compiler. It should not receive `Arc<dyn TemplateLoader>` for normal rendering. The loader remains the abstraction for raw template reads, but startup and watch mode are responsible for translating loader output into prepared registry entries.

`GET /templates` should list names from `TemplateRegistry`, not from a live filesystem scan.

## Watch mode

Watch mode should rebuild prepared registry entries.

When a template-specific file changes, the watcher should reload that template through `TemplateLoader`, prepare it, warm-compile it with the generated sample payload, and replace the existing registry entry only after all steps succeed.

When a file under `_shared/` changes, the watcher should rebuild every template because shared files are included in every template context.

If a watch reload fails, the server should log a warning with the template name and error, keep the last successfully prepared version active, and continue serving requests. This is intentionally different from startup, where the first invalid template aborts the process before the server listens.

Filesystem notifications are processed by one asynchronous worker. This serializes reloads so an
older, slower reload cannot finish after a newer one and overwrite it. If one event includes a
`_shared/` path, the worker rebuilds all current templates once and does not also process the
template-specific paths from that event.

## Test plan

Core tests should cover:

- `RenderService` reads from `TemplateRegistry` and does not call `TemplateLoader::load()` during render.
- validation still runs before compilation;
- missing templates still return `RenderError::TemplateNotFound`;
- sample payload generation covers defaults, required object properties, enums, nested objects, arrays, and nullable fields.

Compiler tests should cover:

- `prepare()` rejects missing or invalid `main.typ`;
- startup warmup compilation maps Typst syntax and import errors to `CompilationError`;
- `compile_prepared()` injects dynamic request payloads through `sys.inputs.data`;
- no `typst::*` or `typst_pdf::*` imports exist outside the compiler module.

Server tests should cover:

- startup fails before binding when a template has a load, schema, compile, or export error;
- successful startup renders using prepared templates;
- `GET /templates` lists names from the prepared registry;
- watch reload success replaces prepared state;
- watch reload failure keeps the last known good prepared state active.

## Assumptions

- Mode 1 filesystem templates remain the only implementation target.
- Store mode, tags, versioning, S3, and compilation cache remain out of scope.
- Startup compilation uses a generated schema-valid sample payload, not a required `payload.json`.
- A startup warmup compile catches syntax, import, font, and many data-access failures, but request-time compilation errors remain possible for payload-dependent Typst branches.
- Warmup is mandatory in the current filesystem MVP. A lazy/disabled warmup policy remains a future
  extension and does not add a CLI or schema option in this phase.
- An empty templates directory produces an empty registry and may start successfully.
