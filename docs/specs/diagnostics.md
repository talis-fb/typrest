# Compiler diagnostics specification

- **Status:** Current
- **Scope:** The owned diagnostic model and its pretty and JSON representations for CLI and HTTP
  compilation failures. Stream routing is defined in
  [Logging and process output](logging.md).

## Purpose

Typrest uses one owned diagnostic model for `typrest check`, server startup, watch reloads, and
payload-dependent render failures. A `CompilationReport` preserves all errors and warnings in
order. This specification defines what a diagnostic contains and how it is presented to CLI users
and HTTP clients.

## Requirements

### Diagnostic model

- **REQ-DIAG-010:** Each `CompilationDiagnostic` MUST carry a severity and a message.
- **REQ-DIAG-011:** A `CompilationDiagnostic` MAY carry a template ID, a virtual source path with a
  one-based line and column range, hints, and call, import, include, or show trace frames.
- **REQ-DIAG-012:** A `CompilationReport` MUST preserve all errors and warnings in order.
- **REQ-DIAG-013:** No public type, HTTP response, or log record MUST expose a `typst::*` type.

### Pretty output

- **REQ-DIAG-020:** Pretty output MUST be the default and MUST use stdout.
- **REQ-DIAG-021:** When source location data is available, pre-build compiler diagnostics MUST
  include an annotated excerpt, the primary range, hints as help messages, and traces as notes.
- **REQ-DIAG-022:** Runtime diagnostics MUST be tracing records without source excerpts.
- **REQ-DIAG-023:** Warnings MUST be visible and MUST NOT turn a successful compile into a failure.
- **REQ-DIAG-024:** `check`, server startup, and watch compilation MUST use the same timestamped
  tracing event set. Exact output and stream ownership are defined in
  [Logging and process output](logging.md).

### JSON Lines output

- **REQ-DIAG-030:** Compiler events MUST use the names `compiler.diagnostic`, `compiler.hint`, and
  `compiler.trace`.
- **REQ-DIAG-031:** A `report_id` MUST relate secondary events to their primary diagnostic.
- **REQ-DIAG-032:** Compiler events MUST include the context (`check`, `startup`, `watch`, or
  `render`), compiler stage, template, message, and virtual range.
- **REQ-DIAG-033:** JSON diagnostics MUST NOT contain source excerpts, payloads, PDF bytes, ANSI
  codes, or physical compiler source paths.
- **REQ-DIAG-034:** JSON diagnostics MUST use tracing, so `RUST_LOG` controls their visibility.
- **REQ-DIAG-035:** Lifecycle records MUST use the names `template.compilation.started`,
  `template.compilation.completed`, `template.compilation.failed`, `package.install.started`,
  `package.install.completed`, `package.install.failed`, `templates.ready`, and `template.ready`.
  They MUST include a `context`, a stable template or package identifier where applicable, and
  status or count fields.
- **REQ-DIAG-036:** Cached-package events MUST be logged at `DEBUG`, so the default filter hides
  them.

### HTTP contract

- **REQ-DIAG-040:** Request-time compilation failures MUST use status `500` and the
  `compilation_failed` envelope. The envelope and an example are defined in the
  [HTTP API specification](http-api.md) (`REQ-HTTP-041`, `REQ-HTTP-043`, `REQ-HTTP-044`).
- **REQ-DIAG-041:** The response MUST carry safe structured diagnostics, MAY include `hints` and
  `trace`, and MUST NOT include source text or physical paths.

### Typst compatibility boundary

- **REQ-DIAG-050:** Typst types MUST NOT cross the compiler boundary.
- **REQ-DIAG-051:** Diagnostic location mapping MUST degrade to a location-less diagnostic when a
  span cannot be resolved, and MUST NOT panic.

The pinning rule, private compiler files, and upgrade checklist are in the
[compiler architecture](../architecture/compiler.md).

## Examples

Pretty and JSON Lines output for the same project:

```bash
typrest --color always check
typrest serve --color never
typrest check --log-format json
typrest serve --log-format json
```

## Non-goals

Stream routing and log filtering, which the [logging contract](logging.md) defines, and the
structure of Typst's own diagnostic types, which never cross the compiler boundary.

## Related documents

- [Logging and process output](logging.md)
- [HTTP API specification](http-api.md)
- [Compiler architecture](../architecture/compiler.md)
- [ADR-0003](../adr/0003-expose-typrest-owned-compiler-diagnostics.md): why diagnostics are owned
  by Typrest.
