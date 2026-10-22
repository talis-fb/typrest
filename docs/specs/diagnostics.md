# Compiler diagnostics

- **Status:** Current. Covers CLI and HTTP compilation failures. Stream routing is defined in
  [Logging and process output](logging.md).
- **Scope:** The owned diagnostic model and its pretty and JSON representations.

## Diagnostic Model

Typrest uses one owned diagnostic model for `typrest check`, server startup, watch reloads, and
payload-dependent render failures. A `CompilationReport` preserves all errors and warnings in
order. Each `CompilationDiagnostic` may contain:

- severity and message;
- template ID;
- virtual source path and one-based line/column range;
- hints;
- call, import, include, or show trace frames.

No public type, HTTP response, or log record exposes a `typst::*` type.

## Pretty Output

Pretty output is the default. All output uses stdout. When source location data is available,
pre-build compiler diagnostics include an annotated excerpt, primary range, hints as help messages,
and traces as notes. Runtime diagnostics are tracing records without source excerpts.

```bash
typrest --color always check
typrest serve --color never
```

Warnings are visible but do not turn a successful compile into a failure.

`check`, server startup, and watch compilation use the same timestamped tracing event set. See the
logging contract for exact output and stream ownership.

## JSON Lines Output

```bash
typrest check --log-format json
typrest serve --log-format json
```

Compiler events use `compiler.diagnostic`, `compiler.hint`, and `compiler.trace`. A `report_id`
relates secondary events to their primary diagnostic. Events include context (`check`, `startup`,
`watch`, or `render`), compiler stage, template, message, and virtual range.

JSON diagnostics omit source excerpts, payloads, PDF bytes, ANSI codes, and physical compiler source
paths. They use tracing, so `RUST_LOG` controls their visibility.

Lifecycle records use `template.compilation.started`, `template.compilation.completed`,
`template.compilation.failed`, `package.install.started`, `package.install.completed`,
`package.install.failed`, `templates.ready`, and `template.ready`. They include a `context`, stable
template or package identifier where applicable, and status/count fields. Cached-package events are
logged at `DEBUG` and are hidden by the default filter.

## HTTP Contract

Request-time compilation failures use status `500` and the `compilation_failed` envelope. The
response carries safe structured diagnostics and may include `hints` and `trace`; it never includes
source text or physical paths. The envelope and an example are defined in the
[HTTP API specification](http-api.md) (`HTTP-041`, `HTTP-043`, `HTTP-044`).

## Typst compatibility boundary

Typst types never cross the compiler boundary, and diagnostic location mapping must degrade to a
location-less diagnostic when a span cannot be resolved; it must not panic. The pinning rule, private
compiler files, and upgrade checklist are in the [compiler architecture](../architecture/compiler.md).
