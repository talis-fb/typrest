# Logging and process output

> Status: Current contract for stdout, pretty logs, and JSON Lines.

## Stream contract

Typrest writes all application output to stdout. Stderr remains unused during normal operation,
including successful commands, warnings, configuration failures, compiler failures, server bind
failures, and watch reloads. This gives operators one ordered stream for an entire process lifetime.

Ordinary application code emits `tracing` events where work occurs. Commands log startup and
registry progress, package resolution logs cache and installation activity, watch mode logs reload
outcomes, and HTTP code logs server and render outcomes. Pretty and JSON modes receive the same
operational event set with the same levels and structured metadata.

The sole direct-output exception is annotated pre-build compiler diagnostics. They use
`codespan_reporting` to retain source excerpts, carets, locations, hints, traces, and color while
writing directly to stdout.

## Implementation ownership

Logging is intentionally split into two focused modules:

- `logging/subscriber.rs` owns `RUST_LOG`, stdout selection, timestamps, levels, formatting, and
  tracing color behavior;
- `logging/compiler_diagnostics.rs` writes individual annotated diagnostics and emits individual
  compiler tracing records with safe JSON fields. Commands, watch mode, and HTTP code own error
  matching and iteration.

`AppState` stores the selected `LogFormat` only so request handlers can choose pretty or JSON
compiler-diagnostic emission.

## Pretty output

Pretty tracing records contain a UTC timestamp, level, and self-contained message. Structured
fields and tracing targets are not appended. A one-template server startup resembles:

```text
2026-08-21T12:48:40.350891Z  INFO compiling template `invoice`
2026-08-21T12:48:40.351234Z  INFO compiled template `invoice`
2026-08-21T12:48:40.351410Z  INFO templates ready: 1
2026-08-21T12:48:40.351703Z  INFO starting typrest server
2026-08-21T12:48:40.351891Z  INFO template watch mode enabled
2026-08-21T12:48:40.352471Z  INFO typrest server listening on 0.0.0.0:8080
```

Pre-build compiler warnings and errors interrupt that tracing sequence with annotated diagnostics
on the same stdout stream. Runtime diagnostics use tracing and do not include source excerpts.

## JSON Lines

JSON mode writes one valid JSON object per stdout line. Records retain event names and safe
structured metadata, including contexts, template and package IDs, statuses, counts, timings,
sizes, and operational paths. For example:

```json
{"timestamp":"2026-08-21T12:48:40.350891Z","level":"INFO","message":"compiling template `invoice`","event":"template.compilation.started","context":"startup","template":"invoice","status":"started","target":"typrest::commands::serve"}
{"timestamp":"2026-08-21T12:48:40.351410Z","level":"INFO","message":"templates ready: 1","event":"templates.ready","context":"startup","template_count":1,"status":"ok","target":"typrest::commands::serve"}
```

Compiler JSON events are `compiler.diagnostic`, `compiler.hint`, and `compiler.trace`. A
`report_id` correlates related records. JSON omits source excerpts, request payloads, PDF bytes,
ANSI codes, and physical compiler source paths.

## Filtering and color

`RUST_LOG` filters every tracing record and defaults to `typrest=info`. Package cache hits use
`DEBUG`, so they are hidden by default and visible with `RUST_LOG=typrest=debug`.

Pretty pre-build compiler diagnostics bypass tracing and therefore bypass `RUST_LOG`; even
`RUST_LOG=off` does not hide them. JSON compiler diagnostics and runtime pretty diagnostics use
tracing and remain filterable.

`--color auto` detects whether stdout is a terminal. `--color always` and `--color never` apply to
pretty tracing and annotated diagnostics. JSON never contains ANSI codes.
