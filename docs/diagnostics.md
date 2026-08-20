# Compiler diagnostics

> Status: Current contract for CLI and HTTP compilation failures. Stream routing is defined in
> [Logging and process output](logging.md).

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

Pretty output is the default. Pre-build warnings are written to stderr and all errors are written to
stdout. When source location data is available it includes an annotated excerpt, primary range,
hints as help messages, and traces as notes. Runtime diagnostics are stdout backend logs.

```bash
typrest --color always check
typrest serve --color never
```

Warnings are visible but do not turn a successful compile into a failure.

`check` and server startup write stable, non-animated lifecycle feedback to stderr. Watch
compilation happens after the backend starts and uses timestamped stdout logs. See the logging
contract for exact output and stream ownership.

## JSON Lines Output

```bash
typrest check --log-format json
typrest serve --log-format json
```

Compiler events use `compiler.diagnostic`, `compiler.hint`, and `compiler.trace`. A `report_id`
relates secondary events to their primary diagnostic. Events include context (`check`, `startup`,
`watch`, or `render`), compiler stage, template, message, and virtual range.

JSON diagnostics omit source excerpts, payloads, PDF bytes, ANSI codes, and physical compiler source
paths. `RUST_LOG` controls tracing-backed verbosity.

Lifecycle records use `template.compilation.started`, `template.compilation.completed`,
`template.compilation.failed`, `package.install.started`, `package.install.completed`,
`package.install.failed`, `templates.ready`, and `template.ready`. They include a `context`, stable
template or package identifier where applicable, and status/count fields. Cached-package events are
classified as verbose and are not emitted by the current default output policy.

## HTTP Contract

Request-time compilation failures use status `500` and expose safe structured diagnostics:

```json
{
  "error": "compilation_failed",
  "message": "unknown variable: amunt",
  "line": 14,
  "diagnostics": [
    {
      "severity": "error",
      "message": "unknown variable: amunt",
      "location": {
        "path": "main.typ",
        "range": {
          "start": { "line": 14, "column": 12 },
          "end": { "line": 14, "column": 17 }
        }
      }
    }
  ]
}
```

The response may include `hints` and `trace`. It never includes source text or physical paths.
Validation and missing-template errors use their separate error contracts.

## Typst Compatibility Boundary

`typst`, `typst-pdf`, `typst-assets`, `typst-kit`, and `typst-syntax` are exactly pinned to the
same version in the workspace manifest and must be upgraded together. Compiler API-sensitive code
stays under `crates/typrest-core/src/compiler/`:

- `world.rs`: world implementation, compilation, export, spans, diagnostics, and inputs;
- `fonts.rs`: font book and decoding APIs;
- `prepared_template.rs`: private Typst-backed prepared state behind public opaque types.

For every Typst upgrade:

1. update all three exact versions in one change;
2. adapt only the private compiler ACL;
3. run compiler and diagnostic fixtures and review snapshots;
4. test syntax, import, warning, font, payload-dependent, and export behavior;
5. run the architecture test that rejects compiler imports outside `compiler/`;
6. update [Typst Integration](typst-integration.md) if the boundary or upstream API changed.

Diagnostic location mapping must degrade to a location-less diagnostic when a span cannot be
resolved; it must not panic.
