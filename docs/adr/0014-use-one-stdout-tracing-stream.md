# ADR-0014: Use one stdout tracing stream

- **Date:** 2026-08
- **Status:** Accepted

## Context

Typrest previously split its output by phase and severity: concise pre-build feedback and warnings
went to stderr, while running-server logs and all errors went to stdout. Implementing that contract
required reporter objects, duplicate output enums, presentation callbacks, and a thread-local
tracing writer. It also forced consumers to merge two independently ordered streams.

Compiler excerpts still need specialized terminal rendering, but ordinary lifecycle and
operational events do not need a separate reporting abstraction.

## Decision

Write all Typrest output to stdout. Emit ordinary lifecycle, package, watch, HTTP, and error events
with `tracing` at the operation site. Use one subscriber for filtering, formatting, color, and
stdout selection in both pretty and JSON modes.

Keep a focused compiler-diagnostics module as the only direct-output exception. Pretty pre-build
diagnostics use `codespan_reporting` and bypass `RUST_LOG`; runtime pretty and all JSON diagnostics
use tracing. Keep only `LogFormat` in HTTP state for request-time diagnostic selection.

## Alternatives

- Retaining the phase-aware stream routing keeps substantial coordination code
  and a two-stream operational contract.
- Sending all output through tracing simplifies filtering but loses annotated source excerpts when
  tracing is disabled.
- Sending all compiler diagnostics directly to stdout preserves excerpts at runtime but makes JSON
  filtering inconsistent and adds direct output on request paths.

## Consequences

- Stdout is the single ordered process stream; stderr is unused during normal operation.
- Pretty and JSON formats share one operational event set and structured metadata.
- Package cache hits can use normal `DEBUG` filtering.
- Annotated pre-build diagnostics remain visible under `RUST_LOG=off`.
- The direct-output boundary is isolated and searchable in `logging/compiler_diagnostics.rs`.
