# ADR-0007: Expose Typrest-owned Compiler Diagnostics

- **Date:** 2026-07
- **Status:** Accepted

## Context

Typst's diagnostic and span APIs may change between pinned versions. Callers need rich diagnostic
context without coupling the registry, CLI, server, and HTTP API to those unstable types.

## Decision

Convert Typst errors and warnings immediately into Typrest-owned diagnostic values. Human output may
render source excerpts; JSON logs and HTTP responses expose only safe structured data.

## Alternatives

- Keeping only the first message would lose important context.
- Exposing Typst diagnostics would couple all consumers to an unstable API.
- Keeping only formatted strings would prevent structured HTTP and log output.

## Consequences

- Typst imports stay under `crates/typrest-core/src/compiler/`, including private prepared state.
- Public diagnostics use owned strings, virtual paths, numeric ranges, hints, and traces.
- The three Typst crates remain exactly and jointly pinned.
- HTTP responses never expose source excerpts or physical paths.
