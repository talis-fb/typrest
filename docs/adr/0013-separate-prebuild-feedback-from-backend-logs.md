# ADR-0013: Separate pre-build feedback from backend logs

- **Date:** 2026-08
- **Status:** Accepted

## Context

Typrest previously wrote both human lifecycle feedback and structured operational logs to stderr.
Pretty output mixed plain compilation lines with timestamped records and exposed structured fields
as `key=value`. Operators need the running backend to provide one clean stdout log stream, while CLI
users still need concise pre-build progress and rich compiler diagnostics.

## Decision

Write all running-server logs, at every severity, to stdout. Keep pre-build compilation and package
progress, ready feedback, warnings, and the pretty server-transition banner on stderr. Write all
errors to stdout, including errors that prevent startup and errors from `check` or `init`.

Pretty operational output contains only timestamp, level, and a self-contained message. JSON Lines
uses the same stream split while retaining safe structured fields. Watch compilation is backend
activity and therefore uses timestamped stdout logs.

## Alternatives

- Writing everything to stderr preserves traditional CLI conventions but does not provide the
  required backend stdout stream.
- Splitting operational output by severity scatters one server execution across two streams.
- Writing everything to stdout contaminates concise pre-build feedback and warning output.

## Consequences

- Stream selection depends on execution context as well as severity.
- Pretty and JSON formatters share event data but have different presentation requirements.
- Consumers must capture both streams during startup and stdout alone while the backend is running.
- The exact contract and examples live in [Logging and process output](../logging.md).
