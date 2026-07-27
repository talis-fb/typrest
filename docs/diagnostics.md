# Compiler Diagnostics and Logging

## Goals

Papelon exposes one compiler diagnostic model across:

- `papelon check`;
- `papelon serve` startup warmup;
- `--watch` reloads;
- payload-dependent render failures.

Typst diagnostics are converted immediately into Papelon-owned values. No public diagnostic,
HTTP response, or log record contains a `typst::*` type.

## Human output

Pretty output is the default. Errors and warnings are written to stderr with:

- the template source path;
- one-based line and column ranges;
- an annotated source excerpt;
- Typst hints as `help` messages;
- Typst call/import/include/show traces as `note` messages.

Color defaults to `auto` and can be changed globally:

```text
papelon --color always check
papelon serve --color never
```

Warnings are always shown but never make compilation fail.

## JSON logging

Use `--log-format json` for JSON Lines output:

```text
papelon check --log-format json
papelon serve --log-format json
```

Compiler diagnostics use `compiler.diagnostic` events. Hints and traces use
`compiler.hint` and `compiler.trace`; `report_id` relates them to the primary diagnostic.
Events include context (`check`, `startup`, `watch`, or `render`), compiler stage, template,
message, and virtual source range.

JSON logs intentionally omit source excerpts, request payloads, PDF bytes, and absolute paths.
`RUST_LOG` continues to control log verbosity.

## HTTP diagnostics

Request-time compilation failures retain the existing HTTP status and top-level fields and add
the complete safe diagnostic collection:

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

HTTP diagnostics expose virtual template paths, ranges, hints, and traces. They never expose source
text or physical filesystem paths.

## Typst version compatibility

`typst`, `typst-pdf`, and `typst-assets` are pinned to the same exact version. Upgrade all three in
one change.

Version-sensitive code is restricted to:

- `compiler/world.rs`: `World`, compilation, export, spans, errors, warnings, hints, and traces;
- `compiler/fonts.rs`: font book and font decoding APIs.

The public model stores owned strings, virtual paths, and numeric ranges. It does not mirror
`SourceDiagnostic`, `Tracepoint`, or other Typst enums. If Typst changes or removes diagnostic
information, update the private mapper and fall back to a location-less diagnostic when necessary;
never panic because a span cannot be resolved.

For every Typst upgrade:

1. update the three exact dependency versions together;
2. compile the workspace and adapt only the private compiler ACL;
3. run the compiler diagnostic fixtures and review snapshot changes;
4. test syntax, import, warning, font, payload-dependent, and PDF-export behavior;
5. run the architecture test that rejects Typst imports outside `compiler/`;
6. update `docs/typst-integration.md` when the upstream API contract changed.
