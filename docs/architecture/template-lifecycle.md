# Template Lifecycle: Startup, Registry, and Watch Mode

- **Status:** Current
- **Scope:** Startup, registry, and watch-mode lifecycle, centered on `TemplateRegistry`.

## Purpose

Typrest loads, prepares, compiles, and PDF-exports every template before the HTTP server listens.
This exposes broken templates at startup and lets render requests reuse template-invariant compiler
state. Each request still performs compilation and export because `sys.inputs.data` is
request-specific.

## Startup Lifecycle

1. The application command layer creates the configured `TemplateLoader`; filesystem mode uses
   `loader::FilesystemLoader`.
2. Call `TemplateLoader::list()` to recursively discover template IDs in deterministic order.
3. Load each `TemplateBundle`, including template-local and `_shared/` files.
4. Parse an optional schema while preserving load errors as `LoaderError` variants.
5. Call `PdfCompiler::prepare()` to discover, cache, and load literal `@preview` package imports and
   build reusable compiler state.
6. Generate a deterministic payload from the schema, or use `{}` without a schema.
7. Validate the generated payload when a schema exists.
8. Run a real `compile_prepared()` and PDF export through the `RenderExecutor` with `Background`
   admission.
9. Store successful `PreparedTemplateBundle` entries in `TemplateRegistry`.
10. Report every collected failure and pass prepared registry/render state to HTTP only when all
    templates succeed.

Registry build and reload APIs optionally report presentation-neutral template lifecycle events.
Application callbacks match those events and emit tracing records directly. Callers that do not
need progress continue to use the no-observer APIs. Missing-package lifecycle events are logged
directly by the application package repository, keeping filesystem knowledge out of `typrest-core`.

An empty templates directory produces an empty registry and may start successfully. Compiler
warnings are reported with their full diagnostic context but do not fail startup.

## Deterministic Warmup Payloads

Schema sample generation follows the precedence in [Schema v1](../specs/schema-v1.md)
(`REQ-SCH-100`); a schema-less template warms with `{}`. The warmup
payload catches syntax, import, font, export, and common data-access failures, but payload-dependent
branches can still fail at render time.

Warmup is mandatory in the current filesystem mode. Lazy or per-template warmup policy is future
work and has no configuration field.

## Registry Design

`TemplateRegistry` stores prepared entries in an ID-ordered map. Entries are immutable and shared
through `Arc`; a short-held `RwLock` protects only the map.

Registry build errors retain the template ID and distinguish:

- listing failures;
- load or schema failures;
- compiler preparation failures;
- invalid generated samples;
- warmup compile or export failures.

`GET /_api/templates` and `GET /_api/templates/{*template_id}` read this registry. They do not scan the
filesystem or reparse schemas.

## Request Lifecycle

1. The handler parses the catch-all route as a validated template ID and parses the JSON body.
2. `RenderService` looks up the prepared entry in `TemplateRegistry`.
3. A missing entry becomes `RenderError::TemplateNotFound`.
4. When a schema exists, validation applies defaults and may return `ValidationError` before
   compilation.
5. `RenderService` builds an owned job holding the compiler, the looked-up `Arc` of the prepared
   bundle, and the validated payload, and passes it to `RenderExecutor::execute()` with `Request`
   admission.
6. `BlockingRenderExecutor` rejects the request with `ExecutionError::Saturated` when every slot and
   queue position is taken. Otherwise it waits for a slot in first-come order and runs the job on a
   blocking thread, which builds request-specific `sys.inputs.data` and reuses the prepared
   template.
7. The handler returns PDF bytes or maps `RenderError` to structured JSON.

Steps 1 to 4 run before admission, so `400`, `404`, and `422` responses never wait for capacity or
occupy a slot. The slot and queue position travel into the blocking closure, so an admitted
compilation keeps its slot until it ends even if the client disconnects, while a request still
waiting in the queue releases its position when its future is dropped. A panic in the job releases
the slot and surfaces as `ExecutionError::Aborted`.

Normal requests never read template or package files, call `TemplateLoader::load()` or
`PackageRepository::resolve()`, rebuild fonts, reconstruct source maps, or access the network.

## Watch Mode

Filesystem notifications are processed by one asynchronous worker so older reloads cannot finish
after newer changes and overwrite them.

- A template-local change reloads, prepares, validates, and warms that template.
- A new package import may download during preparation; failure leaves the previous entry active.
- A route-topology or `_shared/` change builds a complete candidate registry after a short debounce.
- The candidate is built without holding the registry write lock.
- Success swaps the affected entry or complete registry with one short write operation.
- Failure is reported and the last known good entry or registry remains active.
- Warmup compiles use `Background` admission: they wait for a slot behind in-flight work, are never
  rejected, and run on blocking threads instead of runtime workers. Preparation and loading I/O
  still run on the watch worker task.
- A request in flight keeps the `Arc` to the prepared bundle it looked up, so it completes against
  that version even when a swap happens meanwhile (`REQ-HTTP-076`).
- Template compilation and actual missing-package installation progress are tracing events on
  stdout. Successful full rebuilds emit a refreshed ready count.

Watch management emits reload outcomes where they occur and delegates only compiler diagnostic
formatting to `logging/compiler_diagnostics.rs`. It has no dependency on HTTP code.

Startup and watch mode intentionally differ: startup requires every template to be valid, while a
watch failure preserves service availability.

## Enforced Scenarios

Tests cover registry-backed rendering without loader access, validation before compilation,
deterministic sample payloads, real Typst warmup, aggregate startup failures, prepared metadata,
successful replacement, and last-known-good behavior after a failed reload.

See [Architecture](overview.md), [Diagnostics](../specs/diagnostics.md), and [Testing](../development/testing.md).
The stream and formatting contract is defined in [Logging and process output](../specs/logging.md).
