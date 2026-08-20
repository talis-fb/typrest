# Logging and process output

> Status: Current contract for stdout, stderr, pretty logs, and JSON Lines.

## Stream contract

Typrest separates pre-build feedback from backend logs by execution phase. Severity alone does not
select the stream.

| Context | stderr | stdout |
|---|---|---|
| `check` and server pre-build | Template and package progress, successful ready count, warnings | All errors and their related diagnostics |
| Server transition | The pretty-only `Starting server` banner | Nothing |
| Running server, watch, and render | Nothing | All `INFO`, `WARN`, and `ERROR` logs |

This contract applies to both output formats. In JSON mode each stream is independently valid JSON
Lines. An error diagnostic and its hints and traces always stay together on stdout. A successful
`typrest check` writes nothing to stdout.

The two OS streams are independent when redirected. Their relative ordering is guaranteed within
each stream, but consumers must not reconstruct one total ordering across stdout and stderr.

## Pretty output

Template IDs are reported in deterministic order. A one-template server startup is:

```text
Compiling template `invoice`
 Compiled template `invoice`

Templates ready: 1

======== Starting server =========
2026-08-20T12:48:40.351703Z  INFO template watch mode enabled
2026-08-20T12:48:40.352471Z  INFO typrest server listening on 0.0.0.0:8080
```

The compilation lines, ready count, and banner are stderr. Timestamped lines are stdout. The banner
is omitted by `check` and in JSON mode. An empty project reports `Templates ready: 0` without a
leading blank line.

Pretty backend logs contain only a UTC timestamp, level, and readable message. Structured fields
and tracing targets are not appended as `key=value`. Messages include identifiers needed for human
use, such as the listening address or affected template. Watch compilation uses timestamped stdout
messages and does not repeat separate reload-success messages.

Pre-build warnings retain annotated source excerpts on stderr. Pre-build errors retain annotated
source excerpts, hints, and traces on stdout. Runtime diagnostics use timestamped messages on
stdout so the backend never writes to stderr.

## JSON Lines

JSON records retain event names and all available safe structured metadata, including contexts,
template and package IDs, status values, counts, timings, sizes, and operational paths. Existing
diagnostic safety rules remain in force: records omit source excerpts, request payloads, PDF bytes,
ANSI codes, and physical compiler source paths.

Lifecycle event names include `template.compilation.started`,
`template.compilation.completed`, `template.compilation.failed`, `templates.ready`,
`template.ready`, `package.install.started`, `package.install.completed`, and
`package.install.failed`. Backend records additionally include `template.watch.enabled`,
`template.watch.backend.failed`, `server.listening`, watch reload events, and render events.

## Filtering and color

`RUST_LOG` filters tracing-backed operational and JSON records. Direct pretty pre-build progress and
compiler diagnostics are always shown. The default filter is `typrest=info`.

`--color auto` detects the terminal for the stream receiving each pretty diagnostic. JSON never
contains ANSI codes.
