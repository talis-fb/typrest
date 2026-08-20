# Startup Template Compilation

> Status: Current architecture. The implementation is centered on `TemplateRegistry`.

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
8. Run a real `compile_prepared()` and PDF export.
9. Store successful `PreparedTemplateBundle` entries in `TemplateRegistry`.
10. Report every collected failure and pass prepared registry/render state to HTTP only when all
    templates succeed.

Registry build and reload APIs optionally report presentation-neutral template lifecycle events.
The application output layer renders pre-build events as stderr feedback and watch events as stdout
backend logs; callers that do not need progress continue to use the no-observer APIs.
Missing-package lifecycle events remain owned by the application package repository, keeping CLI
presentation and filesystem knowledge out of `typrest-core`.

An empty templates directory produces an empty registry and may start successfully. Compiler
warnings are reported with their full diagnostic context but do not fail startup.

## Deterministic Warmup Payloads

Schema sample generation uses this precedence:

1. explicit `default`, including `null`;
2. first enum value;
3. type seed: `""`, `0`, `false`, recursively generated object, `[]`, or `null`;
4. `null` for an untyped field.

Known object properties are included recursively. Arrays warm with an empty array. The warmup
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
5. The compiler builds request-specific `sys.inputs.data` and reuses the prepared template.
6. The handler returns PDF bytes or maps `RenderError` to structured JSON.

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
- Template compilation and actual missing-package installation progress are timestamped stdout logs
  because watch reloads occur while the backend is running. Successful full rebuilds emit a
  refreshed ready count.

The loader module emits typed watch outcomes. The application output module formats diagnostics
and operational logs, so watch management has no dependency on HTTP or presentation code.

Startup and watch mode intentionally differ: startup requires every template to be valid, while a
watch failure preserves service availability.

## Enforced Scenarios

Tests cover registry-backed rendering without loader access, validation before compilation,
deterministic sample payloads, real Typst warmup, aggregate startup failures, prepared metadata,
successful replacement, and last-known-good behavior after a failed reload.

See [Architecture](architecture.md), [Diagnostics](diagnostics.md), and [Testing](testing.md).
The stream and formatting contract is defined in [Logging and process output](logging.md).
