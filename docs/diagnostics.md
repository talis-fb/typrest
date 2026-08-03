# Compiler Diagnostics and Logging

> Status: Current contract for CLI, logs, and HTTP compilation failures.

## Diagnostic Model

Typrest uses one owned diagnostic model for `typrest check`, server startup, watch reloads, and
payload-dependent render failures. A `CompilationReport` preserves all errors and warnings in
order. Each `CompilationDiagnostic` may contain:

- severity and message;
- template name;
- virtual source path and one-based line/column range;
- hints;
- call, import, include, or show trace frames.

No public type, HTTP response, or log record exposes a `typst::*` type.

## Pretty Output

Pretty output is the default and writes compiler failures and warnings to stderr. When source
location data is available it includes an annotated excerpt, primary range, hints as help messages,
and traces as notes.

```bash
typrest --color always check
typrest serve --color never
```

Warnings are visible but do not turn a successful compile into a failure.

## JSON Lines Output

```bash
typrest check --log-format json
typrest serve --log-format json
```

Compiler events use `compiler.diagnostic`, `compiler.hint`, and `compiler.trace`. A `report_id`
relates secondary events to their primary diagnostic. Events include context (`check`, `startup`,
`watch`, or `render`), compiler stage, template, message, and virtual range.

JSON logs omit source excerpts, payloads, PDF bytes, ANSI codes, and absolute filesystem paths.
`RUST_LOG` controls operational verbosity.

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

`typst`, `typst-pdf`, and `typst-assets` are exactly pinned to the same version in the workspace
manifest and must be upgraded together. Version-sensitive code stays under
`crates/typrest-core/src/compiler/`:

- `world.rs`: world implementation, compilation, export, spans, diagnostics, and inputs;
- `fonts.rs`: font book and decoding APIs;
- `prepared_template.rs`: private Typst-backed prepared state behind public opaque types.

For every Typst upgrade:

1. update all three exact versions in one change;
2. adapt only the private compiler ACL;
3. run compiler and diagnostic fixtures and review snapshots;
4. test syntax, import, warning, font, payload-dependent, and export behavior;
5. run the test that rejects Typst imports outside `compiler/`;
6. update [Typst Integration](typst-integration.md) if the boundary or upstream API changed.

Diagnostic location mapping must degrade to a location-less diagnostic when a span cannot be
resolved; it must not panic.
