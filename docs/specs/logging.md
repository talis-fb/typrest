# Logging and process output specification

- **Status:** Current
- **Scope:** Process streams, pretty logs, JSON Lines, filtering, and color.

## Purpose

Operators get one ordered stream for an entire process lifetime. Ordinary application code emits
`tracing` events where work occurs: commands log startup and registry progress, package resolution
logs cache and installation activity, watch mode logs reload outcomes, and HTTP code logs server and
render outcomes.

Logging is split into two focused modules:

- `logging/subscriber.rs` owns `RUST_LOG`, stdout selection, timestamps, levels, formatting, and
  tracing color behavior;
- `logging/compiler_diagnostics.rs` writes individual annotated diagnostics and emits individual
  compiler tracing records with safe JSON fields. Commands, watch mode, and HTTP code own error
  matching and iteration.

`AppState` stores the selected `LogFormat` only so request handlers can choose pretty or JSON
compiler-diagnostic emission.

## Requirements

### Stream contract

- **REQ-LOG-010:** Typrest MUST write all application output to stdout.
- **REQ-LOG-011:** Typrest MUST NOT use stderr during normal operation, including successful
  commands, warnings, configuration failures, compiler failures, server bind failures, and watch
  reloads.
- **REQ-LOG-012:** Pretty and JSON modes MUST receive the same operational event set with the same
  levels and structured metadata.
- **REQ-LOG-013:** Annotated pre-build compiler diagnostics MUST be the only direct-output
  exception. They MUST use `codespan_reporting` to retain source excerpts, carets, locations, hints,
  traces, and color while writing directly to stdout.

### Pretty output

- **REQ-LOG-020:** Pretty tracing records MUST contain a UTC timestamp, level, and self-contained
  message, and MUST NOT append structured fields or tracing targets.
- **REQ-LOG-021:** Pre-build compiler warnings and errors MUST interrupt the tracing sequence with
  annotated diagnostics on the same stdout stream.
- **REQ-LOG-022:** Runtime diagnostics MUST use tracing and MUST NOT include source excerpts.

### JSON Lines

- **REQ-LOG-030:** JSON mode MUST write one valid JSON object per stdout line.
- **REQ-LOG-031:** Records MUST retain event names and safe structured metadata, including
  contexts, template and package IDs, statuses, counts, timings, sizes, and operational paths.
- **REQ-LOG-032:** Compiler JSON events MUST be `compiler.diagnostic`, `compiler.hint`, and
  `compiler.trace`, and a `report_id` MUST correlate related records.
- **REQ-LOG-033:** JSON output MUST NOT contain source excerpts, request payloads, PDF bytes, ANSI
  codes, or physical compiler source paths.

### Filtering and color

- **REQ-LOG-040:** `RUST_LOG` MUST filter every tracing record and MUST default to `typrest=info`.
- **REQ-LOG-041:** Package cache hits MUST use `DEBUG`, so they are hidden by default and visible
  with `RUST_LOG=typrest=debug`.
- **REQ-LOG-042:** Pretty pre-build compiler diagnostics MUST bypass tracing and therefore
  `RUST_LOG`; even `RUST_LOG=off` MUST NOT hide them.
- **REQ-LOG-043:** JSON compiler diagnostics and runtime pretty diagnostics MUST use tracing and
  remain filterable.
- **REQ-LOG-044:** `--color auto` MUST detect whether stdout is a terminal. `--color always` and
  `--color never` MUST apply to pretty tracing and annotated diagnostics.
- **REQ-LOG-045:** JSON output MUST NOT contain ANSI codes.

## Examples

A one-template server startup in pretty mode:

```text
2026-08-21T12:48:40.350891Z  INFO compiling template `invoice`
2026-08-21T12:48:40.351234Z  INFO compiled template `invoice`
2026-08-21T12:48:40.351410Z  INFO templates ready: 1
2026-08-21T12:48:40.351703Z  INFO starting typrest server
2026-08-21T12:48:40.351891Z  INFO template watch mode enabled
2026-08-21T12:48:40.352471Z  INFO typrest server listening on 127.0.0.1:8080
```

JSON Lines records:

```json
{"timestamp":"2026-08-21T12:48:40.350891Z","level":"INFO","message":"compiling template `invoice`","event":"template.compilation.started","context":"startup","template":"invoice","status":"started","target":"typrest::commands::serve"}
{"timestamp":"2026-08-21T12:48:40.351410Z","level":"INFO","message":"templates ready: 1","event":"templates.ready","context":"startup","template_count":1,"status":"ok","target":"typrest::commands::serve"}
```

A signal-initiated shutdown that drains in time (`REQ-CLI-047`):

```json
{"timestamp":"2026-08-21T12:50:02.100112Z","level":"INFO","message":"received SIGTERM, draining in-flight requests","event":"server.shutdown.started","signal":"SIGTERM","target":"typrest::http"}
{"timestamp":"2026-08-21T12:50:02.412870Z","level":"INFO","message":"shutdown completed","event":"server.shutdown.completed","duration_ms":312,"target":"typrest::http"}
```

When requests outlast the deadline, `server.shutdown.completed` is replaced by a `WARN`
`server.shutdown.timed_out` record with `timeout_ms`.

## Non-goals

Log shipping, rotation, file output, and metrics.

## Related documents

- [Compiler diagnostics](diagnostics.md)
- [CLI specification](cli.md)
- [ADR-0008](../adr/0008-use-one-stdout-tracing-stream.md): why one stdout tracing stream.
